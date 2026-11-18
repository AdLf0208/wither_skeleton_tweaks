# Wither Skeleton Tweaks — port notes (P3 logic tier, second port)

Ported from **Wither Skeleton Tweaks** (mod id `wstweaks`, MIT; Modrinth
`wither-skeleton-tweaks`) for Minecraft Java 1.21.1 NeoForge. Local-only port.

Status: `ported` — conversion, drop cleanup, the skull-fragment item and its
`Shard Drop Chance` config, and the Immolation Blade (content + hit path) are
implemented and verified. Remaining gaps are recorded below (the blade's
fireworks damage source, alias ids and non-modelled config keys).

Campaign pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`, `game.rs` SHA-256
`64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f`
(migrated from `41f822b6`/`2488718…` on 2026-11-09).

## Behaviour (public docs + upstream source)

1. **Wither Skull Fragments** — a wither skeleton that does not drop a Wither
   Skeleton Skull drops a `wstweaks:fragment`. Upstream gates the drop on
   `Shard Drop Chance` (default 1.0, i.e. always) with a `nextFloat() <=`
   roll (`WSTLootModifier.java:44-48`, `WSTConfig.java:18`); the port reads the
   same key from `config/wstweaks.toml`.
2. **Skeleton Conversion** — any skeleton **or stray** that spawns in the
   Nether becomes a wither skeleton. Upstream also replaces the main hand with
   a fresh bow (`Give Bows`, default true); the replacement spawned by this
   port is bare (API gap, below).
3. **Drop Cleanup** — stone swords and bows are removed from the drops of
   skeleton-based creatures (skeletons, wither skeletons, strays, bogged),
   **damaged or not**.
4. **Immolation Blade** — a sword that ignites its target for 150 seconds and
   instantly kills and beheads skeleton-family mobs
   (`ImmolationBladeItem.java:19-32`). Carried as content
   (`wstweaks:immolation_blade`, durability 4096, attack damage 11, attack
   speed -2; `WSTConfig.java:22-25`) and implemented through `hurt_enemy`.

## Hooks

| Hook | Helper | What it does |
|---|---|---|
| `server_about_to_start` (v1, HEAD) | `start.rs` | Read `config/wstweaks.toml` (write the default file when absent). |
| `mob_spawned` (v1, HEAD) | `spawn.rs` | `conversion_target(dim, kind)`; swap the just-spawned skeleton/stray for a wither skeleton at the same position. |
| `loot_drops` (v1, HEAD, `collect`) | `loot.rs` | `context.entity` names a mob: drop stone swords/bows, add a guaranteed skull after a blade kill, and add the fragment when no skull dropped and the shard roll passes. |
| `hurt_enemy` (v1, HEAD) | `hurt.rs` | The acting player's landed hit with `wstweaks:immolation_blade`: ignite 150 s; skeleton family is marked, set to 1 health, killed and shoved. |

All call sites exist at the pin (`creatures.rs` mob death; `native_block_drops`
skips the entity path; `mace.rs::weapon_hit` calls `hurt_enemy`). The helpers
bind the existing parameters only.

## Pure engine (`runtime.rs`, 9 tests)

- `is_skeleton_family` (skeleton, wither_skeleton, stray, bogged).
- `conversion_target(dim, kind)` — Nether + `minecraft:skeleton` or
  `minecraft:stray`.
- `is_cleaned_drop` / `clean_drops` — stone swords and bows only, with no
  damage predicate; other items and other mobs untouched.
- `needs_fragment` — wither skeleton whose drops hold no
  `minecraft:wither_skeleton_skull`.
- `parse_config` / `format_config` — `shard_drop_chance` 0..=1, clamped;
  unknown keys and invalid values keep the default 1.0.
- `shard_roll_passes` / `fragment_for` — upstream's `<=` roll on top of the
  missing-skull check.
- `blade_hit` / `guaranteed_skull` — 150 s (3000 ticks) ignite for every mob;
  behead + wither/skeleton skull for the skeleton family.
- `blade_shove` — upstream's random `[2-4r, 4r, 2-4r]` push.

## Gaps

- **Converted-mob equipment:** upstream converts in place and, with
  `Give Bows` (default true), puts a fresh undamaged bow in the converted
  mob's main hand (`WSTEvents.java:39-42`). `NativeSpawn` has no equipment
  field at the pin (`native_mobs.rs`), so the port's replacement spawns bare;
  keeping the original's weapon instead would need a mob-equipment write API
  (none exists; `native_on_equip` only listens). The previous note claim that
  the converted mob "keeps its weapon" was wrong.
- **Blade damage source:** upstream's beheading deals a 150-damage
  `minecraft:fireworks` hit (`ImmolationBladeItem.java:23-27`), which is also
  what the loot modifier keys on. The pin's `loot_drops` context has no damage
  source (`native_hooks.rs:673-706`; `creatures.rs` passes `tool: None` for
  mob deaths), so the port marks the mob in `hurt_enemy` and adds the
  guaranteed skull in `loot_drops`. Behaviour matches, the damage type does
  not.
- **Blade config keys:** only `Shard Drop Chance` is modelled. `Durability`,
  `Attack Damage` and `Attack Speed` are carried as fixed content components
  (`4096`, `11`, `-2`); changing them needs an item-component/registry write
  the content format does not offer at runtime. `Delete Swords` and
  `Give Bows` are not modelled (defaults true).
- **Blade alias ids:** upstream registers `wstweaks:blaze_blade` and
  `wstweaks:lava_blade` as aliases (`WitherSkeletonTweaks.java:37-38`). The
  content format has no aliases, so old worlds naming those ids get no item.
- **Guaranteed skull for other skeleton-family mods:** the port matches the
  four vanilla ids (`is_skeleton_family`), while upstream's
  `instanceof AbstractSkeleton` also covers modded subclasses (W9).
- **Spawn stage:** upstream converts during `FinalizeSpawnEvent`
  (`WSTEvents.java:23`, natural spawns/spawners/eggs). The pin's `mob_spawned`
  is post-add, so `/summon` (used by `tools/e2e.sh`) also converts here and
  the original entity is removed and re-spawned, losing its name, passengers,
  persistence and goals. `/summon` conversion is a port behaviour, not
  upstream's. `natural: false` covers spawners and eggs too, so gating on it
  would under-convert.
- Duck-typing note: the drop cleanup matches item ids, so a datapack that
  renames skeleton loot tables still works, but a modded bow id would not be
  cleaned.

## Verification

Skills: superpowers/using-superpowers,verification-before-completion,systematic-debugging; BeautifulRust/rust-best-practices

- `cargo test --locked --offline` — 9/9 (slice 2 run below; the 2026-11-02
  run was 5/5).
- `cargo clippy --all-targets --locked --offline -- -D warnings` — clean.
- `stage` → `check` → `test` at the pin — exit 0 for each (slice 2 run below).
- Fast build + `tools/e2e.sh` — slice 2 run below (content loaded with
  `--dev-mod`).

## Licence

MIT — derivatives allowed; local-only until maintainer approval
(ARCHITECTURE.md §8).

## In-game check

`tools/e2e.sh` launches the staged fast build with `--dev-mod content/` and
checks: a Nether skeleton and stray become wither skeletons, an Overworld
skeleton stays one, a wither skeleton killed with a fresh undamaged bow drops
no bow (a zombie control keeps it), both carried items are giveable, the
fragment drops at `Shard Drop Chance=1` and not at 0, the blade sets an iron
golem on fire while a wooden sword does not, and a blade kill guarantees a
skeleton skull while an ordinary death does not. The scenario drives
conversion with `/summon`, which upstream would not convert (it hooks
`FinalizeSpawnEvent`); the port's post-add hook does. The full slice-2
evidence is in the section at the end.

## Verification results

Run 2026-11-02 against pin `c9d474dd9116e84c243f8992d75c11314df56f06`
(tag `nightly-source/2026-11-01-c9d474d`), rustc 1.95, all with
`HORIZON_MIXINS=<campaign>/horizon-mixins`.

### 1. Staged embedded tests — pass

```
$ cd ports/wstweaks
$ python3 tools/horizon.py test --horizon <campaign>/horizon
{ "stage": ".../ports/wstweaks/target/mixins/modded",
  "stage_changed": false,
  "horizon_revision": "c9d474dd9116e84c243f8992d75c11314df56f06",
  "cargo_exit": 0,
  "cargo_log": ".../target/horizon-test.log" }
$ echo $?
0
```

`target/horizon-test.log` (cargo `game::wstweaks::tests`):

```
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 531 filtered out; finished in 0.00s
```

The staged tree was already current (`stage_changed: false`), so 5/5 embedded
tests (`game::wstweaks::tests`) stand for the same source that was staged.

### 2. Fast build — pass

```
$ python3 tools/horizon.py build --horizon <campaign>/horizon
{ "cargo_exit": 0,
  "binary": ".../ports/wstweaks/target/horizon-cargo/fast/horizon-client" }
$ echo $?
0
```

Log tail: `Finished 'fast' profile [optimized] target(s) in 5m 56s`, exit 0.
The bin emits two dead-code warnings for the injected pure engine
(`Drop::new` and `clean_drops` are used by the runtime tests, not the glue);
they are warnings only and do not affect the exit code.

### 3. Control binary — pass

```
$ cd target/mixins/modded
$ cargo build --profile fast --locked --offline -p horizon-client \
    --bin horizon-ctl --no-default-features --target-dir .../target/horizon-cargo
    Finished `fast` profile [optimized] target(s) in 3.61s   (exit 0)
```

`target/horizon-cargo/fast/horizon-ctl` exists (4.6 MB).

### 4. In-game scenario — pass (`tools/e2e.sh`, exit 0)

Chat lines produced by the scenario (`ctl command` replies, last of four
consecutive passes):

```
{"chat":["[Player30384] NETHER_CONVERTED"],"ok":true}
{"chat":["[Player30384] OVERWORLD_KEPT"],"ok":true}
```

Drop control (a zombie with the identical damaged bow keeps it, which proves
the equipment drop and its forced chance work in this build):

```
{"entities":[{"distance":4.412151299372697,"id":476,
  "item":{"count":2,"item":"minecraft:rotten_flesh"}, ...},
 {"distance":5.211741990039769,"id":477,
  "item":{"components":"{\"minecraft:damage\":25}","count":1,
          "damage":25,"item":"minecraft:bow"}, ...}]}
```

Final `ctl entities 8 item` after killing three wither skeletons that each
held the same damaged bow (bow drop chance forced to 2.0):

```
{"entities":[{"distance":3.30670171826736,"id":527,
  "item":{"count":1,"item":"minecraft:bone"},"kind":"item",
  "pos":[3.028143161013415,56.0005,-0.875],"type":"minecraft:item"}]}
(radius 12) {"entities":[{"distance":3.295720884799975,"id":527,
  "item":{"count":1,"item":"minecraft:bone"}, ...}]}
```

No `minecraft:bow` in the list; the script asserts this with a `case` (and
fails the run if the zombie control does not show a bow). `done: both chat
checks passed and no damaged bow dropped`.

The script was run four times after the fixes; all four exited 0 with the two
chat lines and no bow. One earlier run reused a saved world because the game's
game folder defaulted to the port directory (see fix 3).

### Script fixes needed to reach the game

The first run of the shipped script failed at the first check
(`{"error":"timed out after 10 s","kind":"timeout"}`). Causes and fixes, all
confined to `tools/e2e.sh`:

1. **Dimension switching.** Horizon keeps one level in place; `execute in
   minecraft:the_nether run summon` does enter the Nether level for the command,
   but a freshly opened level streams its chunks asynchronously. The mod's
   replacement spawn is refused while chunk [0,0] is not loaded
   (`native_spawn_mob` checks `native_chunk_loaded`), so the skeleton stayed a
   skeleton. The script now force-loads chunk [0,0] of the Nether
   (`execute in minecraft:the_nether run forceload add 0 0`) and waits 5 s
   before summoning. Verified: `NETHER_CONVERTED` and
   `@e[type=wither_skeleton,x=0,y=64,z=0,distance=..4]` both match.
2. **Item NBT format.** At this pin entity NBT is the 26.3 codec:
   `equipment:{mainhand:{id,count,components:{"minecraft:damage":N}}}` and
   `drop_chances:{mainhand:2.0f}`. The old `HandItems`/`HandDropChances` keys
   are stripped/ignored on load, so the old script gave the mob no bow at all
   and the drop assertion could not exercise the cleanup. The new NBT keeps the
   same assertion but makes it meaningful (control bow drops with damage 25;
   wither-skeleton bows are removed).
3. **Isolation.** `horizon-ctl launch` passes the working directory to the game
   as its game folder by default, so the old script wrote `saves/`,
   `horizon-player.txt` and `.horizon-cache/` into `ports/wstweaks/`. The script
   now `cd`s into its scratch home and passes `--game-dir "$SCRATCH/game"`; a
   rerun leaves the port tree unchanged. (The stray files from the first run
   were removed.)
4. **Positioning.** The player is teleported to (0, 64, 0) and the drop mobs
   are summoned ~5 blocks away with `NoAI:1b`, so gravity, wandering and the
   player's pickup radius cannot move their drops out of the query; the zombie
   control is cleared with `kill @e[type=minecraft:item]` before the
   wither-skeleton half.
5. **Assertions.** The chat checks are read from the `ctl command` reply
   (deterministic; `wait-for chat` without `--since` can miss a line that
   arrived before it started), the final item list is asserted in the script
   (radius 8 and a belt-and-braces radius 12), not only printed.

## Upstream source

Analysed 2026-11-02. Repository
`https://github.com/Shadows-of-Fire/Wither-Skeleton-Tweaks`, branch `1.21`,
mod **10.1.1** (the only 1.21.1 release; the repository has no tags), commit
`1962c69bd63c2c51496c3e57a8cb9b7b9ae5dca2`. The `26.1` branch
(`35677a2`, version 11.0.1) targets Minecraft 26.1.2 and is **not** the pack
version. Areas read: `WSTEvents.java`, `WSTLootModifier.java`, `WSTConfig.java`,
`WSTObjects.java`, `ImmolationBladeItem.java`, `WitherSkeletonTweaks.java`, the
loot-modifier/recipe/tag/lang resources, `changelog.md` and `README.md`.

This source corrected three claims above; all three are now applied to the
code and the sections at the top of this file:

- Drop cleanup has **no damage predicate**: every stone sword and bow dropped
  by an `AbstractSkeleton` is removed (`WSTEvents.java:52-56`), controlled by
  `Delete Swords` (default true). The port's `damage > 0` test was removed and
  the unit test now asserts a fresh sword and a fresh bow are removed too.
- The fragment item is **`wstweaks:fragment`** (`WSTObjects.java:14`), not
  `wstweaks:wither_skull_fragment`, and its drop is gated by `Shard Drop
  Chance` (default 1.0) as well as the missing-skull check
  (`WSTLootModifier.java:44-48`, `WSTConfig.java:18`). The constant was
  renamed; the default 1.0 is kept unconditionally because the port has no
  config surface, and that gap is recorded in the pure-engine docs.
- Conversion targets `instanceof Skeleton` (skeleton **and stray**) during
  spawn finalisation (`WSTEvents.java:23-29`), converts in place, and with
  `Give Bows` (default true) puts a **new bow** in the main hand (`:39-42`).
  It does not keep the original weapon by default. The port now also converts
  `minecraft:stray` in the Nether; it still removes and respawns the entity
  (post-add hook) and the replacement is bare because the pin's `NativeSpawn`
  has no equipment field. Giving a fresh bow — or preserving the original's
  equipment — needs a mob-equipment write API that the pin does not have.

The Nether condition itself is unchanged (`WSTEvents.java:27`, `allBiomes` off
by default).

Also extracted: guaranteed skulls when the killing damage is fireworks or an
Immolation Blade (`WSTLootModifier.java:33-42`), the blade's tier/stats and hit
sequence (`ImmolationBladeItem.java:19-32`, `WSTConfig.java:23-25`), the
all-biomes conversion branch (`WSTEvents.java:27`) and the full config default
table.

Full evidence, per-row comparison and risks:
[`../_shared/upstream/wstweaks/FINDINGS.md`](../_shared/upstream/wstweaks/FINDINGS.md).

## Follow-up run 2026-11-02: upstream alignment (W1, W2, W3)

Applied the three behaviour corrections from `FINDINGS.md`: cleanup without the
damage predicate, `wstweaks:fragment`, and stray conversion. Only
`mixins/wstweaks/runtime.rs`, `mixins/wstweaks/support.rs` and this file
changed; tests were edited first.

RED — `cargo test --locked --offline` before the implementation, exit 101:

```
test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out

---- tests::clean_drops_counts_and_keeps_other_items stdout ----
assertion `left == right` failed
  left: 2
 right: 4
---- tests::nether_skeletons_and_strays_convert_but_nothing_else_does stdout ----
assertion `left == right` failed
  left: None
 right: Some("minecraft:wither_skeleton")
---- tests::skeleton_weapons_are_cleaned_whether_damaged_or_not stdout ----
panicked at mixins/wstweaks/runtime.rs:110:13: minecraft:skeleton
---- tests::fragment_fills_a_missing_skull stdout ----
assertion `left == right` failed
  left: "wstweaks:wither_skull_fragment"
 right: "wstweaks:fragment"
```

GREEN — `cargo test --locked --offline`, exit 0:

```
running 5 tests
test tests::a_non_skeleton_drop_list_is_untouched ... ok
test tests::fragment_fills_a_missing_skull ... ok
test tests::clean_drops_counts_and_keeps_other_items ... ok
test tests::skeleton_weapons_are_cleaned_whether_damaged_or_not ... ok
test tests::nether_skeletons_and_strays_convert_but_nothing_else_does ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo fmt --all -- --check` and
`cargo clippy --all-targets --locked --offline -- -D warnings` — exit 0.

Ladder at the pin with `HORIZON_MIXINS=<campaign>/horizon-mixins`:

| command | exit | note |
|---|---|---|
| `tools/horizon.py stage` | 0 | `stage_changed: true` (source changed) |
| `tools/horizon.py check` | 0 | only the two pre-existing injected-engine dead-code warnings (`Drop::new`, `clean_drops`) |
| `tools/horizon.py test` | 0 | 5/5 embedded, 531 filtered |

`target/horizon-test.log`:

```
running 5 tests
test game::wstweaks::tests::a_non_skeleton_drop_list_is_untouched ... ok
test game::wstweaks::tests::fragment_fills_a_missing_skull ... ok
test game::wstweaks::tests::nether_skeletons_and_strays_convert_but_nothing_else_does ... ok
test game::wstweaks::tests::clean_drops_counts_and_keeps_other_items ... ok
test game::wstweaks::tests::skeleton_weapons_are_cleaned_whether_damaged_or_not ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 531 filtered out; finished in 0.00s
```

The fast build and `tools/e2e.sh` were not re-run here (the dispatcher batches
them). The e2e asserts a *damaged* bow, so it cannot distinguish the removed
predicate; the unit test does. Its `/summon` conversion is the port's broader
post-add behaviour, not upstream's `FinalizeSpawnEvent`.

## Verification results (in-game, corrected)

Run 2026-11-02 against pin `c9d474dd9116e84c243f8992d75c11314df56f06`, with
`HORIZON_MIXINS=<campaign>/horizon-mixins`. This is the re-run the earlier
follow-up deferred, and the e2e now exercises both upstream corrections that
the old scenario missed: **Nether stray conversion** and a **fresh, undamaged
bow** in the drop cleanup.

### 1. Fast build — exit 0

```
$ cd ports/wstweaks
$ python3 tools/horizon.py build --horizon <campaign>/horizon
{ "cargo_exit": 0,
  "cargo_seconds": 101.745,
  "binary": ".../ports/wstweaks/target/horizon-cargo/fast/horizon-client" }
$ echo $?
0
```

`target/horizon-build.log` ends with `Finished 'fast' profile [optimized]
target(s) in 1m 41s`. The binary is 78,842,976 bytes (2026-11-02 21:44). Only
the two pre-existing injected-engine dead-code warnings (`Drop::new`,
`clean_drops`) are reported; `cargo_exit` is 0.

### 2. Control binary — exit 0

```
$ cd target/mixins/modded
$ cargo build --profile fast --locked --offline -p horizon-client \
    --bin horizon-ctl --no-default-features --target-dir .../target/horizon-cargo
    Finished `fast` profile [optimized] target(s) in 0.44s
$ echo $?
0
```

`target/horizon-cargo/fast/horizon-ctl` (4,640,216 bytes). Cargo found it
current against the staged pin tree, so no sources were recompiled.

### 3. In-game scenario — `timeout 500 tools/e2e.sh`, exit 0

Full transcript: `/tmp/opencode/wstweaks-e2e.log` (the script now tees there).

Chat replies from `ctl command` (verbatim):

```
{"chat":["[Player46333] NETHER_CONVERTED"],"ok":true}
{"chat":["[Player46333] STRAY_CONVERTED"],"ok":true}
{"chat":["[Player46333] OVERWORLD_KEPT"],"ok":true}
```

The stray check is a real differential: the skeleton's converted wither
skeleton is killed before the stray is summoned, and the script first asserts
that no `minecraft:stray` remains (`STRAY_LEFT_BEHIND` would fail the run).
A leftover wither skeleton therefore cannot satisfy `STRAY_CONVERTED`; only a
stray that actually converted can.

Zombie control (identical fresh bow, same forced chance) keeps it — and the
item has no `components`/`damage` field at all, proving the drop path works
with an undamaged stack:

```
{"entities":[{"distance":6.248575634480509,"id":527,
  "item":{"count":1,"item":"minecraft:bow"},"kind":"item",
  "pos":[5.979311816742274,57.007302976080005,1.8145176561637737],
  "type":"minecraft:item"}, ...]}
```

Final item list after killing three wither skeletons that each held the same
fresh bow (`FRESH_BOW='equipment:{mainhand:{id:"minecraft:bow",count:1}},drop_chances:{mainhand:2.0f}'`):

```
{"entities":[{"distance":7.406681649433917,"id":569,
  "item":{"count":2,"item":"minecraft:bone"},"kind":"item",
  "pos":[7.309936314588811,56.0005,0.6517391216713088],
  "type":"minecraft:item"}]}
(radius 12) {"entities":[{"distance":7.413021464660381,"id":569,
  "item":{"count":2,"item":"minecraft:bone"}, ...}]}
```

No `minecraft:bow` at radius 8 or 12. The script's `case` assertion is
unchanged in force (it still fails on any bow), and the zombie half still
requires `minecraft:bow` to appear, so a broken equipment drop cannot make the
negative check pass vacuously. The only scenario change is the bow: no
`components` block (damage 0) instead of `components:{"minecraft:damage":25}`.
This is the in-game counterpart of the unit test that removed the damage
predicate.

Final script line:

```
done: nether skeleton + stray converted, overworld skeleton kept, zombie control dropped its fresh bow, no fresh bow dropped by wither skeletons
```

## Slice 2 (2026-11-05): fragment + Shard Drop Chance + Immolation Blade

### Semantics (upstream `1.21` @ `1962c69`, mod 10.1.1)

- **Fragment id and roll:** `WSTObjects.java:14` registers `fragment`;
  `WSTLootModifier.java:44-48` adds one fragment to an exact
  `WitherSkeleton`'s loot when `random.nextFloat() <= shardDropChance` and the
  loot has no wither skeleton skull. `WSTConfig.java:18` sets
  `Shard Drop Chance` default `1.0`, range 0–1. The `<=` is upstream's, so a
  roll of exactly 0 passes chance 0 (vanishingly rare).
- **Immolation Blade item:** `WSTObjects.java:16` (`immolation_blade`);
  `WitherSkeletonTweaks.java:29` tier: incorrect-for-netherite-tool,
  durability `4096`, mining speed 0, attack-damage bonus `11` (12 total),
  enchantment value 30, repaired with a nether star; attack-speed modifier
  `-2` (`WSTConfig.java:22-25`). Alias ids `blaze_blade`/`lava_blade`
  (`WitherSkeletonTweaks.java:37-38`).
- **Blade hit:** `ImmolationBladeItem.java:19-32`: ignite the target for
  150 s; apply the normal sword hit; for an `AbstractSkeleton`, set health to
  1, deal a 150-damage `minecraft:fireworks` hit, and push by
  `[2-4r, 4r, 2-4r]`.
- **Guaranteed skull:** `WSTLootModifier.java:33-42,53-59`: a fireworks or
  Immolation Blade kill gives a wither skeleton skull to an exact
  `WitherSkeleton` (if none) and a skeleton skull to any other
  `AbstractSkeleton`.
- **Recipes/lang/assets:** `data/wstweaks/recipe/immolation_blade.json`
  (3×3 blaze rods + nether star + stick), `.../wither_skeleton_skull.json`
  (nine fragments), `data/minecraft/tags/item/swords.json`,
  `assets/wstweaks/{lang,models/item,textures/items}`.

### TDD

Tests were added first; RED was `5 passed; 4 failed` on the new tests
(`shard_chance_gates_the_fragment`, `config_parses_the_shard_chance_and_clamps_it`,
`a_blade_hit_ignites_every_mob_and_beheads_the_skeleton_family`,
`the_blade_shove_matches_upstreams_random_push`). Implementation then made
them green; the standalone suite is now **9/9**.

`cargo test --locked --offline` (standalone, exit 0):

```
running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo clippy --all-targets --locked --offline -- -D warnings` — exit 0.

### Config (port choice)

`config/wstweaks.toml`:

```toml
shard_drop_chance = 1.0
```

The parser is pure (`parse_config`/`format_config`), clamps to 0..=1, ignores
unknown keys and invalid values, and writes the default file when absent. The
pin has no datapack-reload hook; the glue re-reads a changed file (mtime) on
the next mob death. `Delete Swords`, `Give Bows`, `Convert All Biomes`,
`All Biomes Chance` and the blade stat keys are **not** modelled (defaults are
fixed in behaviour/content; see Gaps).

### Content

Source: upstream MIT assets at `1962c69` (no ATM10 jar was present under
`ports/_shared/carry-work/wstweaks/`; no decompilation). 26.3 conversions:
`textures/items/` → `textures/item/`, new `assets/wstweaks/items/*.json`
definitions, `models/item` layer paths updated, recipes converted to 26.3
string ingredients, and item declarations under
`data/wstweaks/horizon/item/`. The fragment uses the `minecraft:stick`
template with `!minecraft:cooking_fuel`; the blade uses
`minecraft:iron_sword` with `max_damage` 4096, `attack_damage` 11,
`attack_speed` -2, `enchantable` 30 and nether-star repair. A mod creative tab
holds both (the format cannot add to vanilla Combat/Ingredients tabs).

`content/horizon-mod.toml` (id `wstweaks`, version `10.1.1`, API 0.2, both,
registries); full provenance and file list in `content/README.md`.

`horizon-mod build`/`check` (exit 0 each):

```
wstweaks 10.1.1 (mod, API 0.2, both)
  8049 bytes, SHA-256 6d10ecacca7669367fb81844f834d700b04ac1a15d17d0ccb465ec669b7c8d86, unsigned
  may add blocks, items, mob effects and entity types to your worlds
  no code: content only, nothing to run
```

The delivered `content/wstweaks.hmod` is 8,049 bytes, SHA-256
`6d10ecacca7669367fb81844f834d700b04ac1a15d17d0ccb465ec669b7c8d86`.

### Ladder at the pin

Pin `c9d474dd9116e84c243f8992d75c11314df56f06`, horizon-mixins
`d9c83d7096b7ea5b83678da2efdfd6161f74997e`, rustc 1.95.0. The campaign-wide
`clean-targets.sh` deleted the first stage mid-run (and SIGTERMed a live
build); the stage was rebuilt from a pristine pin export and the ladder was
then completed with `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2`.

| command | `cargo_exit` | evidence |
|---|---|---|
| `tools/horizon.py stage` | — | exit 0, `stage_changed: true`, 2 modules, 4 injections, 100.49 s |
| `tools/horizon.py check` | 0 | **0 warnings** in `target/horizon-check.log`, `Finished dev` 58.42 s |
| `tools/horizon.py test` | 0 | 9 passed, 531 filtered in `target/horizon-test.log` |
| `tools/horizon.py build` | 0 | **0 warnings** in `target/horizon-build.log`, `Finished fast` 13m27s; `horizon-client` 81,651,248 bytes, SHA-256 `b3d027131728b68793c2c8d004580839e1b6726db8229d31f6cf87d97d972578` |
| `cargo build --profile fast --bin horizon-ctl` | 0 | 4,812,176 bytes, SHA-256 `f758151729f78d1255cf2ce39ddaee744d45f8972c63af706069ca11b4406282` |

`Drop::new` and `clean_drops` are now `#[cfg(test)]` (test-only helpers), so the
injected module contains no dead code and the check/build logs have no
warnings. The fresh `target/horizon-check.log` and `target/horizon-build.log`
are the warning evidence above; the old 2026-11-02 logs were stale.

### In-game e2e (`tools/e2e.sh`, exit 0)

Final transcript: `/tmp/opencode/wstweaks-slice2-final.log`; exit file
`/tmp/opencode/wstweaks-slice2-final.exit` contains `0`, and the last line is
`PASS: wstweaks slice 2 (conversion, cleanup, fragment chance 1/0, blade fire
differential, blade beheading)`. The content folder is loaded with
`--dev-mod content/`; the config is written into the scratch game folder. The
run keeps the slice-1 conversion and fresh-bow-cleanup phases.

| check | observed |
|---|---|
| Nether skeleton → wither skeleton | `NETHER_CONVERTED` |
| Nether stray → wither skeleton | `STRAY_CONVERTED` (and no `STRAY_LEFT_BEHIND`) |
| Overworld skeleton kept | `OVERWORLD_KEPT` |
| items giveable | `inventory: fragment=1 blade=1` |
| fragment at `Shard Drop Chance=1` | `fragments at chance 1: 3` (three wither skeletons) |
| fragment at `Shard Drop Chance=0` | `fragments at chance 0: 0` |
| blade burns an iron golem | `100.0 -> 96.384 -> 92.384` (health keeps falling) |
| wooden sword does not burn | `100.0 -> 99.154 -> 99.154` |
| ordinary skeleton death has no skull | only bone(s) in the drop list |
| blade kill beheads a skeleton | `minecraft:skeleton_skull` in the drop list |

The e2e is fail-capable: the fragment half is a differential (same mobs, only
the config chance changes), the fire half compares the blade with a wooden
sword, and the beheading half compares a blade kill with an ordinary death of
the same mob. The blade fallback (`blade_held_by`) was added after one run
lost the guaranteed skull to a same-tick loot-event race; the script now waits
5 s after the blade hit so the instant kill (or, if that races, the fire's
lethal tick) and its loot land before the query. The final run passed with an
explicit `PASS` line. The lazy-config reload is exercised by rewriting the
file between the chance-1 and chance-0 halves.

### Blade implementation notes

`hurt_enemy` (API 39, `mace.rs::weapon_hit`) is the attack path: it gives the
held stack and the mob id, and fires for the host and guests. The port
ignites `Mob::ignite_for_ticks(3000)`, marks the skeleton-family mob, sets
health 1 and kills it with `native_kill_mob` (as `/kill`), then applies the
shove with `native_push_mob`. The guaranteed skull is added in `loot_drops`:
the mark is the delayed-death path, and `context.player` + the acting player's
held blade covers the same-tick case. `loot_drops` has no damage source at the
pin (`creatures.rs` passes `tool: None` for mob deaths), so the port cannot
label the hit `minecraft:fireworks`; the outcome (guaranteed skull) matches.

### Packaging and template-test note

`python3 tools/package.py` (exit 0) wrote
`target/wstweaks-0.1.0.hnmod`: 10,457 bytes, SHA-256
`761e66e21dc5a22c93ec5d7d5851b74da145b863a1deaf27f4ca6a3210f00fbf`.
It contains `start.rs` and `hurt.rs` alongside the existing sources.

`python3 tools/test_template.py` cannot validate this bundle: its tiny fixture
has no `native-hooks.json`, so the mixins planner rejects the named
`server_about_to_start`/`mob_spawned`/`loot_drops`/`hurt_enemy` hooks with
`E_HOOK_CATALOG`. The real pinned `stage`/`check`/`test` ladder above is the
validation for named hooks. `tools/test_stage.py` was not re-run (it re-exports
the full pin; the same AST application was already exercised by `stage`).

### Skills

Loaded `test-driven-development` (tests first, RED then GREEN),
`systematic-debugging` (the e2e race/mark and the `next_float` trait import),
`verification-before-completion` (every claim above is from a run), and
Beautiful Rust `rust-best-practices` (fmt, clippy `-D warnings`, expressive
types, `#[allow]` only for the injected-module dead code used by standalone
tests).

## Pin migration (2026-11-09): `41f822b6` → `1d63e0a5`

Mode A of `ports/_shared/horizon-mods/brief-artifact-current.md`. The stamped
hook catalog is byte-identical across the two pins (`repin-1d63e0a.md` §1), so
the bundle needed only the pin edits: `compatibility.json` (`horizon_revision` +
`game.rs` hash `2488718…` → `64813cf4…`), `mixins/wstweaks/horizon-mixin.toml`
`[compatibility]`, and `tools/horizon.py`'s defaults (pin checkouts
`ports/_shared/pins/horizon-1d63e0a` / `horizon-mixins-5a64af9`). No hook,
binding, helper or runtime source changed.

Ladder (all through `ports/tools/build-slot.sh`, `CARGO_INCREMENTAL=0`, no bare
cargo, no `--target-dir`):

| command | exit | evidence |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | standalone lease `wstweaks-migrate` |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | 0 warnings |
| `cargo test --workspace --locked --offline` | 0 | 9/9 (`tests::…`, 0 filtered) |
| `tools/horizon.py stage` | 0 | `stage_changed: true`, pin `1d63e0a5`, 92.5 s |
| `tools/horizon.py check` | 0 | **0 warnings** in `target/horizon-check.log`, 73.9 s |
| `tools/horizon.py test` | 0 | `game::wstweaks::tests`: 9 passed, 0 failed, 1240 filtered |
| `tools/horizon.py build` (fast) | 0 | exit 0, `target/horizon-build.log`, 421.8 s |

Packages (`tools/package.py`, `cargo-horizon-mod horizon-mod build`; a second
build reproduced each hash):

- `content/wstweaks.hmod` — 8,049 B, SHA-256
  `6d10ecacca7669367fb81844f834d700b04ac1a15d17d0ccb465ec669b7c8d86`,
  24 entries (unchanged bytes; repacked and re-checked by
  `cargo-horizon-mod check`, exit 0).
- `wstweaks.hnmod` (port root) — 10,837 B, SHA-256
  `b82f361a5b865e30fe55d6bd8a04baea3857a2d601b61fb12fb80cadf681f856`,
  8 entries (`horizon-native.json` + the 7 `mixins/wstweaks/` files).

In-game e2e on the new pin pair (`build-slot.sh e2e`: rebuild + scenario in one
lease; shared `horizon-ctl` from `_shared/bin/horizon-1d63e0a5`, the port's own
fast `horizon-client`):

- `_shared/evidence/wstweaks/e2e-default.log` — `PASS` + `EXIT:0`: Nether
  skeleton/stray convert, Overworld skeleton kept, items giveable, fragment
  chance 1 → drop / chance 0 → none, blade burns a golem and a wooden sword does
  not, blade kill yields a `minecraft:skeleton_skull` while an ordinary death
  does not.
- `_shared/evidence/wstweaks/e2e-negative.log` — `NEGATIVE=1` expects the
  Nether skeleton **not** to convert, so the run stops at the conversion
  assertion: `FAIL` + `EXIT:1`, proving that assertion is not vacuous.

`python3 ports/tools/check-e2e-logs.py wstweaks --strict` → 2/2 ok.
`python3 ports/tools/upstream_ready.py wstweaks` → ready.

## Pin migration (2026-11-10): `41f822b6` → `1d63e0a5` — ladder green, e2e blocked

Mode A of `ports/_shared/horizon-mods/brief-artifact-current.md`, resumed after a
concurrent job added the embedded-content half to this port at 01:03–01:04
(`mixins/wstweaks/embedded_content.rs` and
`mixins/wstweaks/data/wstweaks-content.hmod`, a ninth `embedded-content`
injection at `horizon_client::game::mods::content_root`). My earlier migration
ladder (2026-11-09) predates that change; this section re-runs everything against
the **current** tree, which is what the delivered bundle now contains.

Pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`, `game.rs` SHA-256
`64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f`. Edits:
`compatibility.json`, `mixins/wstweaks/horizon-mixin.toml` `[compatibility]`,
`tools/horizon.py` defaults (pin checkouts).

Ladder (`ports/tools/build-slot.sh`, `CARGO_INCREMENTAL=0`, one lease):

| command | exit | evidence |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | 0 warnings |
| `cargo test --workspace --locked --offline` | 0 | 9/9 |
| `tools/horizon.py stage` | 0 | `stage_changed: true`, 156.4 s |
| `tools/horizon.py check` | 0 | **0 warnings** (`target/horizon-check.log`) |
| `tools/horizon.py test` | 0 | `game::wstweaks::tests`: 9 passed, 1240 filtered |
| `tools/horizon.py build` (fast) | 0 | `cargo_exit: 0` |

Packages (reproducible across two runs):

- `content/wstweaks.hmod` — 8,049 B, SHA-256
  `6d10ecacca7669367fb81844f834d700b04ac1a15d17d0ccb465ec669b7c8d86`, 24
  entries (unchanged; also the embedded `data/wstweaks-content.hmod`).
- `wstweaks.hnmod` — 17,254 B, SHA-256
  `25db76a2459a68354a1790f688fe6f9a4c6ceb5bc409eae613b22129aa90edfe`, 10
  entries (the 7 original files + `embedded_content.rs` + the embedded content
  `.hmod` + the manifest).

**In-game e2e: blocked, not passed.** The default leg fails deterministically
(two runs, identical result) at the last assertion:

```
== blade: a blade kill guarantees a skeleton skull ==
-- blade-killed skeleton items:
{"entities":[{"item":{"count":2,"item":"minecraft:arrow"}, …}]}
FAIL: the blade kill did not guarantee a skeleton skull
EXIT:1
```

Every earlier phase passes on the same run (Nether conversion with a fresh bow,
`Give Bows=false`, Overworld control, fragment chance 1 → drop / 0 → none, blade
ignites a golem while a wooden sword does not). The `NEGATIVE=1` leg fails at the
conversion assertion as designed (`FAIL` + `EXIT:1`), so the harness is
fail-capable. This same assertion **passed** on 2026-11-09 before the embedded
content landed; the runtime, `support.rs`, `hurt.rs` and `loot.rs` are byte-
unchanged, so the new variable is the embedded-content injection/loading path
(the game log shows one content mod loaded from
`.horizon-cache/mod-content/<hash>`). Diagnosing/fixing it belongs to the
embedded-content slice (another writer); this job does not claim the behaviour.

Evidence: `_shared/evidence/wstweaks/e2e-default.log` (FAIL + EXIT:1, two runs),
`e2e-negative.log` (FAIL + EXIT:1). `check-e2e-logs.py wstweaks --strict` fails
on the default leg because it is a real failure, not a missing exit line.

## Embedded content verified in game (2026-11-10, supersedes the "e2e blocked" note above)

The port now ships its content inside the native bundle (the AE2 pattern,
ARCHITECTURE.md §8), so one file installs the items and the hooks:

- `mixins/wstweaks/data/wstweaks-content.hmod` — the built
  `content/wstweaks.hmod`, byte for byte (8,049 B);
- `mixins/wstweaks/embedded_content.rs` — `native_data!` compiles the file in,
  writes `.horizon-cache/wstweaks-embedded-content.hmod` when the copy on disk
  differs, appends it to the dev list and breaks with
  `content_root(root, enabled, &with)`;
- one `[[injection]] id = "embedded-content"` on
  `horizon_client::game::mods::content_root` (`kind = "HEAD"`, `cancellable =
  true`, bindings `root`/`enabled`/`dev`);
- `tools/package.py` packs `mixins/<id>/data/` and refuses a data file that
  differs from `content/<id>.hmod`.

| Artifact | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `wstweaks.hnmod` | 17,254 B | `25db76a2459a68354a1790f688fe6f9a4c6ceb5bc409eae613b22129aa90edfe` | 10 |
| …of which `mixins/wstweaks/data/wstweaks-content.hmod` | 8,049 B | `6d10ecac…b7c8d86` (unchanged content) | — |

`cargo-horizon-mod horizon-mod check wstweaks.hnmod --kind native-mod` →
`wstweaks 0.1.0 (native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3)`,
17,254 bytes, same SHA-256, unsigned, **exit 0**. (An intermediate 17,479 B
package was built at 07:59 while this diagnosis temporarily instrumented
`support.rs`; the diagnostics were removed and `tools/package.py` reproduces the
recorded 17,254 B / `25db76a2…` bytes exactly, so the delivered behaviour code
is the verified one. The embedded helper and the plan are byte-identical to the
01:04 build.)

Ladder at `1d63e0a` through the slot registry: `cargo fmt --check`,
`cargo clippy --all-targets --locked --offline -D warnings`, `cargo test
--locked --offline` (9 passed) clean; `horizon.py stage` → `check` → `test` →
`build` (`cargo_exit: 0`, 0 warnings, `game::wstweaks::tests`: 9 passed, 1240
filtered); `content/wstweaks.hmod` rebuilt through the pin's
`cargo-horizon-mod` is byte-identical to the delivered file.

**In-game proof from one file** — launched with **no `--dev-mod`**, twice,
both green:

| Leg | Result |
|---|---|
| `EMBEDDED=1 tools/e2e.sh` (`e2e-embedded.log`) | `PASS` + `EXIT:0` (re-run on the final sources) |
| the same again (`e2e-embedded-repeat.log`) | `PASS` + `EXIT:0` — the 08:26 repeat, on sources that still carried the temporary diagnosis; superseded by the four legs below |
| `EMBEDDED=1 NEGATIVE=1 tools/e2e.sh` (`e2e-embedded-selftest.log`) | `FAIL: NEGATIVE: the Nether skeleton converted, but this leg expects it not to` + `EXIT:1` |

The bundle writes `8049 B at …/game/.horizon-cache/wstweaks-embedded-content.hmod,
identical to content/wstweaks.hmod` (the game's working directory is its
`--game-dir`), and the whole scenario runs from that file: Nether skeleton and
stray conversion, the Overworld control, drop cleanup, the fragment chance 1 → 1
and 0 → 0, `Give Bows=false` leaving the mob bare, the blade's fire
differential against a wooden sword, and — the assertion that failed twice on
the 01:04 build — **the blade kill's guaranteed skeleton skull**, on both runs,
with the same sources. The cause of those two red runs is measured in the next
section; it was not the embedded half.

**Control** (same run, before the main leg): `.horizon-cache` is a plain file in
both the scratch root and the game folder, so the helper cannot write. The same
binary then answers
`{"chat":["Unknown item 'wstweaks:immolation_blade'", …]}` and
`{"chat":["Unknown item 'wstweaks:fragment'", …]}` to `give`, holds neither
item, and still gives a vanilla stick.

`python3 ports/tools/check-e2e-logs.py wstweaks --strict` → 5 logs, all with a
matching exit status (0); `python3 ports/tools/upstream_ready.py wstweaks` →
**READY**.

## The blade-behead red runs (2026-11-10): measured cause, not the port

Two default-leg runs failed at the last assertion — the blade kill dropped no
skeleton skull — and the same assertion had passed the day before, with the
behaviour Rust byte-unchanged. Both brief hypotheses were tested and **both are
refuted by measurement**; the section records what was measured, because the
symptom (a normal loot roll, no skull) has only two possible causes and neither
is in the shipped code.

### Hypothesis 1 — the embedded content's load path: refuted

In the default leg both content paths are in the dev list, but
`content_root` skips a mod id it has already loaded, and `--dev-mod content/`
comes first, so the embedded copy is *not* loaded: the game log says
`Horizon: content from 1 mod(s)`. Measured across three launch modes with the
same scenario and the same binary (`tools/diag-behead.sh`):

| mode | content path | blade-kill skull |
|---|---|---|
| `default` | `--dev-mod content/` + the bundle's embedded copy | yes (8/8 rounds) |
| `devinert` | `--dev-mod content/`, the helper's write made to fail (read-only cache file) | yes (1/1) |
| `embedded` | no `--dev-mod`; the bundle's copy only | yes (`e2e-embedded.log`) |

### Hypothesis 2 — the 26.3 pin's loot/behead behaviour: refuted

`mace.rs::weapon_hit` calls `hurt_enemy` **after** the sword damage, and a mob
whose loot is rolled names `context.entity`; both are unchanged at this pin.
The one ordering that could lose the mark — the vanilla hit already lethal, so
the loot rolled *before* the hook — does not happen: with the target merged to
5 HP, `hurt_enemy` sees `health 0.0`, `set_mob_health`/`kill_mob` both return
`false`, and the loot still runs **after** the hook (`take_blade_kill` true) and
the skull is added (3/3 rounds). `loot_drops` also still sees the entity, which
the fragment phases prove on every run.

### What actually differed, and what the fix is

The two red runs shared one binary, and that binary now passes twice with the
same script (7 green default runs in total, 2 of them on the 05:41 artefact).
The moving parts left were **the world and the target**, both in the harness:

- no `--seed`: every run generated a new world, and the transcripts show it —
  the red world's ground put the drops at y≈55/56, the green ones at y≈57–60.
  A site whose floor height moves between runs is a different scenario;
- the beheading target was "the first skeleton within 10 blocks", not the one
  the phase had just summoned;
- nothing in the phase reported *which* entity it hit, so a red run could not be
  explained after the fact.

`tools/e2e.sh` now (a) passes `--seed 20261105`, (b) builds its own arena — a
21×21 stone floor at y=63 with clear air above, in the Overworld **and** the
Nether — so no phase depends on the seed's terrain, (c) targets the skeleton at
the summon point (`mob_id_near skeleton 3`), and (d) prints the target id, the
held item and the skeletons left after the swing, so the next failure is
self-describing. The assertion itself is untouched: it still encodes upstream's
guarantee.

The four legs on the final sources:

| Leg | Evidence | Result |
|---|---|---|
| default | `e2e-default.log` | `PASS` + `EXIT:0` |
| `NEGATIVE=1` | `e2e-negative.log` | `FAIL: NEGATIVE: the Nether skeleton converted…` + `EXIT:1` |
| `EMBEDDED=1` | `e2e-embedded.log` | `PASS` + `EXIT:0` |
| `EMBEDDED=1 NEGATIVE=1` | `e2e-embedded-selftest.log` | `FAIL …` + `EXIT:1` |

`check-e2e-logs.py wstweaks --strict` → 5 logs, 0 missing/bad exit statuses;
`upstream_ready.py wstweaks` → READY.

### Gap found while measuring: the same-tick fallback cannot fire

`on_loot_drops` keeps a second path — `blade_held_by`, added for "a same-tick
loot-event race" — but at this pin it is **inert**: the loot context's `player`
is `killer_uuid`'s `hub.guest(conn).uuid`, while `native_acting_id()` is
`ints_uuid(acting_player_uuid())`; the two are different uuids for the same
player, so `player != game.native_acting_id()` is always true. Measured on every
blade kill in the diagnosis (`held=false` while the blade is in hand). The
guarantee therefore rests on the mark alone, which is sound here because the pin
rolls a killed mob's loot *after* `hurt_enemy` (measured above). The code is
left as it is — it can only return `false`, so it is harmless — and this note is
the record. A pin that exposes a damage type (upstream keys on
`minecraft:fireworks`) would let the port drop the mark entirely.

### Infrastructure fault that cost three rebuilds (not this port)

While re-running the ladder, `horizon.py check`/`build` failed three times with
`could not parse/generate dep info at … (os error 2)` and once with
`horizon-client`'s `build.rs` panicking on a missing
`horizon-client-render/src/gfx/native_render.rs` in the staged tree. Both are
the same fault: the slot runner closed the lease fd in the parent shell, so the
flock was released immediately and several jobs shared one slot, deleting each
other's `stage/modded` mid-build (15 ports have such logs today). The runner was
fixed at 14:35 to close the fd only in the subshell; the ladder above was
re-run on that runner and is green.

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-wstweaks.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** [P1] upstream MIT notice not shipped
  **Repair:** The `LICENSE` now carries `Copyright (c) 2019 Brennan Ward`, and both archives carry the notice (`mixins/wstweaks/LICENSE`, `data/wstweaks/license/`).
- **Reviewer:** description truncated and its conversion limitation stale
  **Repair:** Rewritten: the `Give Bows` behaviour is described as implemented (the packaged `support.rs` does call `native_living_set_equipment`), the old "no equipment write API" claim is gone, and the blade limitations are complete sentences.
- **Reviewer:** release text/PR body hash mismatch
  **Repair:** The repackaged file has a new hash; the release asset and the PR body must be re-aligned on re-push (orchestrator).
- **Reviewer:** relative link
  **Repair:** Already absolute (`https://github.com/Shadows-of-Fire/Wither-Skeleton-Tweaks`).
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Migration to the SDK format (2026-11-18): `.hnmod` → one flat `.hmod`

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, cleanup PR #17992) retired the
`.hnmod` container. The registry closed every `.hnmod` submission on 2026-11-18 with one
reason, *"republish this as a native `.hmod` with the SDK from the next Nightly"*, and this
port's **PR #589** was one of them. The new unit is **one flat `.hmod`** — content, native
code, or both. A resubmission is a **new version**: this one is `0.2.0` (the closed PR was
`0.1.0`).

**Pins.** nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78, `cargo-horizon-mod` from
`horizon-rust/horizon@main`. Both are in `horizon-mod.toml` `[native]`; the SDK is fetched
into `.horizon/sdk/` by any `cargo horizon-mod` command and is never packed.

The campaign's own rule is what survived: **`src/runtime.rs` is the retired file byte for
byte** (`sha256 47027d76449ebca917655fa255750fe951db8dc29d32400331adb03304982a58`, also the
hash of the same path *inside* the delivered package), and its nine tests were green before a
line of glue was written.

### Files created

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity: id, version `0.2.0`, `api = "0.2"`, `side`, `capabilities`, `license`, `homepage`, `issues`, `[native]` |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`, the blade's `#[mod_api::horizon_part] impl ItemPart`, the resolved item/entity keys and the blade mark |
| `src/runtime.rs` | the retired `runtime.rs`, copied with `cp` and checked with `cmp` (byte-identical) |
| `src/config.rs` | `config/wstweaks.toml` through `host.config_files()` (read / write / stat) |
| `assets/wstweaks/**`, `data/wstweaks/**`, `data/minecraft/tags/item/swords.json` | the carried content, moved out of the retired `content/` folder into the flat layout |
| `wstweaks.hmod` | the delivered package (port root, never `target/`) |

`LICENSE` and `LICENSE-NOTICE.md` stay at the package root: the format packs root
`LICENSE*`/`NOTICE*`/`README*` text files, so the MIT notice and the port's provenance travel
inside the archive (the retired split pair carried them in `mixins/wstweaks/LICENSE` and
`data/wstweaks/license/*.txt` instead).

### Files deleted, and where the evidence went

Everything below is in **`ports/_shared/evidence/wstweaks/retired-hnmod-0.1.0/`**:

| Deleted | Why | What it was |
|---|---|---|
| `mixins/wstweaks/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, `start.rs`, `spawn.rs`, `loot.rs`, `hurt.rs`, `embedded_content.rs`, `data/wstweaks-content.hmod`) | the container is retired; `runtime.rs` moved to `src/` | the plan/injection schema, 4 hook helpers, the state module and the AE2-style embedded-content injection |
| `wstweaks.hnmod` | unloadable on launcher ≥ 0.2.70 | **20,188 B**, `sha256 05b93ffbfc169c7e2007cfa8b9f478194e74c5efd0dba82d5bc16560cab89b7b` — the 2026-11-11 reviewer-repair repack (the `17,254 B` / `25db76a2…` bundle in the sections above was superseded by that repair) |
| `content/` (`horizon-mod.toml`, `wstweaks.hmod`, `README.md`) | the content lives in the flat package now | **9,500 B**, `sha256 50f8acb1f5a92c016dbce6b81d8fd191370bfb0487f9431bd8c8a3645d8737e3` (also the reviewer-repair repack; `6d10ecac…` is the earlier one) |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` pins the two commits; the launcher decides the toolchain | the pin record of the retired format |
| `tools/horizon.py`, `tools/package.py`, `tools/test_stage.py`, `tools/test_template.py`, `tools/stamp_license.py`, `tools/diag-behead.sh`, `tools/audit/` | staging, `.hnmod` packaging and the retired-format checks are gone | the build loop of the retired format |
| `Cargo.lock` | not needed without registry dependencies, and it changes the artifact's hash (trap 2) | — |
| `.github/workflows/ci.yml` | the retired template's workflow: it installs Rust 1.95 and the horizon-mixins transformer and reads the deleted `compatibility.json`, so it would fail on every push. Deleted rather than rewritten: a CI file this machine cannot run would be an untested claim | — |

**Kept:** `tools/e2e.sh` (the scenario and every assertion are format-independent; its header
now says it drove the retired staging harness and refuses without `LEGACY_HARNESS=1`),
`notes.md`, `VALIDATION.md`, `REPIN-AUDIT.md`, `README.md` (rewritten for the flat layout).

### Manifest mapping

| Old | New |
|---|---|
| `horizon-mixin.toml` `[mod] id = "wstweaks"` | `horizon-mod.toml` `id = "wstweaks"` (also the resource namespace) |
| `[mod] version = "0.1.0"` | `version = "0.2.0"` (a resubmission needs a new one) |
| `[mod] priority = 100` | gone: ordering is `#[listener(priority = …)]` or `[dependencies] order` |
| `[compatibility] horizon_revision = "1d63e0a5…"` | `[native] horizon = "0b14f8a…"` |
| the bundle's mixins pin `5a64af93…` | `[native] mixins = "e59da9094f0c…"` (**it moved too**) |
| — | `[native] game = "26.3"` |
| `content/horizon-mod.toml` (`id`, `version = 10.1.1`, api 0.2, side, capabilities) | the same fields in the one root manifest; `version` is the **port's** `0.2.0`, not upstream's `10.1.1` |
| `[[module]] runtime` / `[[module]] state` | plain Rust modules: `pub mod runtime;`, `pub mod config;` (`pub`, or every item warns `dead_code`) |
| `[[injection]] hook = …` + `helper = <file>.rs` + `bindings` | one `Hooks` method of the same name; `host` is the game, `event` is the moment |
| `horizon-native.json`, the plan schema | gone; the manifest is the only one |

### Hook → `Hooks` / `ItemPart` mapping

| Old injection (helper) | New method | Mapping notes |
|---|---|---|
| `server_about_to_start` HEAD (`start.rs`), `game.native_config_dir()` + `fs` | **`Hooks::register_configs` + `Hooks::config_loading`** | this pair *is* NeoForge's `ModContainer.registerConfig` + `ModConfigEvent.Loading`: `register_configs` pushes `mod_api::Config::new("wstweaks", ConfigKind::Server, default_text).file("wstweaks.toml")` and the game writes the default file when it is missing; `config_loading` hands the text back (checked against `config.mod_id`) and `config::loaded` parses it. A `Server` config loads before each world. The mid-session re-read (`Shard Drop Chance` rewritten live, the e2e's chance-1 → chance-0 halves) stays, through `host.config_files().read/stat` in the world phase, comparing `ConfigStat::modified_millis` instead of `SystemTime` |
| `mob_spawned` HEAD (`spawn.rs`), `game.native_mobs(dimension)` + `native_remove_mob` + `native_spawn_mob` + `native_living_set_equipment` | `Hooks::mob_spawned(&self, host, MobSpawnedEvent { dimension, kind, pos, natural })` | `level.entities_in(host, min, max)` finds the mob (it is already in the level), `entity.kind(host)` matches the resolved `EntityTypeKey`s, `entity.discard(host)` is `Entity.discard` (no death, no loot), `level.spawn_mob(host, target, pos, SpawnReason::Conversion)` replaces it, `living.set_equipment(host, MainHand, bow)` is API 382 |
| `loot_drops` HEAD (`loot.rs`), `game.native_loot_drops` bindings `context`, `drops` | `Hooks::loot_drops(&self, host, LootDropsEvent { context, drops })` | `context.entity` is `Option<(u32, &str)>`; `context.damage_source` now exists (`HurtSource { damage_type, causing, direct, position }`), and `context.weapon_item(host)` reads the recorded/live weapon |
| `hurt_enemy` HEAD (`hurt.rs`), global helper checking `stack.item` | **`#[mod_api::horizon_part] impl ItemPart for Blade`**, attached to `wstweaks:immolation_blade` in `register` with `r.item_part(key, Blade)` | the game calls the Part only for that item, so the stack check disappears. `entity.set_remaining_fire_ticks(host, 3000)` replaces `Mob::ignite_for_ticks`; `living.set_health(host, 1.0)`; `host.kill_mob(dimension, id)` is `/kill` (its loot rolls through `loot_drops`, which is what carries the mark); `entity.push(host, shove)` with the level's own random stream |
| `embedded-content` HEAD on `game::mods::content_root` (`embedded_content.rs`, `native_data!`) | **deleted** | the content is in the same `.hmod`; there is no second archive to write out or append to the dev list |
| `support.rs` statics (`CONFIG`, `BLADE_KILLS`) + 5 private helpers | `IDS: OnceLock<Ids>`, `CONFIG: Mutex<…>` (in `config.rs`), `BLADE_KILLS: Mutex<Vec<(u32, i64)>>`, one free function per job | `Hooks: Any + Send + Sync`; the `ItemPart` is a separate `Arc` from the `Mod` the macro boxes, so the blade mark is a process-wide `Mutex` as it was |

**Item ids are keys now.** The retired helpers compared id strings (`"minecraft:bow"`) against
the game's stacks. The API identifies items and entity types by opaque `ItemKey`/
`EntityTypeKey` and offers no public key → id read, so `register` resolves the ten keys the
rules need (`r.item_key`, `r.entity_type_key`) and the glue compares keys directly; the
runtime's `Drop { item: String, … }` view is built only for the three ids its predicates
test (`stone_sword`, `bow`, `wither_skeleton_skull`), which is exactly what they compare.
`runtime.rs` is unchanged; its rules never see a stack.

### Fidelity changes this migration made (each a `native_*` → typed call, not new behaviour)

1. **The blade is an item Part, not a global hook.** Before, `hurt_enemy` fired for every
   item and the helper returned early unless the stack was the blade; now the game calls it
   only for `wstweaks:immolation_blade`. Same observable behaviour, one less per-hit check.
2. **`damage_source` is read.** Upstream's guaranteed skull keys on a `minecraft:fireworks`
   kill *or* the Immolation Blade (`WSTLootModifier.java:33-42`); the pin had no damage
   source, so the old port could only implement the blade half. `loot_drops` now also adds
   the skull when `context.damage_source.damage_type == "minecraft:fireworks"`.
3. **`blade_held_by`'s dead path is replaced.** The old same-tick fallback compared
   `context.player` with `native_acting_id()`, which `ARCHITECTURE.md` §3 records as *always
   false* (two uuids for one player). `context.weapon_item(host)` is the supported read now
   (the recorded projectile weapon or the live melee weapon, resolving `damage_source.causing`
   and its main hand), so the same-tick case is covered by a path that can actually answer;
   the mark stays for a delayed death, whose source is the fire, not the blade.
4. **The `SKULL_FRAGMENT` registration check is gone.** The old glue asked
   `game.res.items().get(SKULL_FRAGMENT).is_some()` because the content could be a separate
   file that was not installed. Content and code are one package now, and `r.item_key`
   validates against all declarations, so a missing item refuses the mod at load instead of
   silently dropping the fragment.
5. **The config is a registered config.** `register_configs` + `config_loading` are
   NeoForge's `registerConfig` + `ModConfigEvent.Loading`, so the game writes the default file
   and a world's `serverconfig/wstweaks.toml` takes precedence for that world — NeoForge's own
   rule, which the retired port could not express. The file name stays `wstweaks.toml` and the
   keys are unchanged (`shard_drop_chance`, `give_bows`).
6. **Logging is gone.** The old support module logged the config value and reloads through
   `log::info!`. The API has no logging surface and this crate takes no registry dependency,
   so the config is silent. Nothing observable changes; an operator reads the file instead.

### Build and check (the ladder for an SDK package)

Every command ran in the port through the campaign slot registry
(`BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-wst --port ports/wstweaks --step …`,
`CARGO_INCREMENTAL=0`, no `--target-dir`); `build-slot.sh` does not `cd`, so the command is
`bash -c 'cd ports/wstweaks && …'`. Logs: `ports/_shared/evidence/wstweaks/sdk-*.log`.

| Step | Command | Exit | Evidence |
|---|---|---|---|
| Formatting | `cargo fmt --check` | 0 | `sdk-fmt.log` |
| Lints | `cargo clippy --all-targets -- -D warnings` | 0 | `sdk-clippy.log`, 0 warnings |
| Pure rules | `cargo test` | 0 | `sdk-test.log`: **9 passed**, 0 failed |
| Package | `rm -f Cargo.lock; cargo horizon-mod build` | 0 | `sdk-build.log` → `target/horizon-mod/wstweaks.hmod`, copied to the port root |
| Registry check | `cargo horizon-mod check wstweaks.hmod` | 0 | `sdk-check.log`, text below |
| What it uses | `cargo horizon-mod check wstweaks.hmod --uses` | 0 | `sdk-check-uses.log`; the package's `uses.json` lists **160** entries |
| Machine form | `cargo horizon-mod check wstweaks.hmod --json` | 0 | `sdk-check-json.log` |
| Fits the pin | `cargo horizon-mod fits wstweaks.hmod --build nightly` | 0 | `sdk-fits.log`: `Wither Skeleton Tweaks:` / `0b14f8a  made for` |
| `runtime.rs` inside the package | `unzip -p wstweaks.hmod src/runtime.rs \| sha256sum` | — | `47027d76…82a58` = the retired file, byte for byte (`sdk-sha256.txt`) |

`check` output, verbatim:

```
wstweaks 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  26954 bytes, SHA-256 3e9b3e82e3ea0c7dccee62b821d37467101b95f5d0599ad546acbc18978cb98b, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

A second `cargo horizon-mod build` (after `rm -f Cargo.lock`) reproduced the same bytes:
`cmp` clean against the delivered file. The delivered package: **`wstweaks.hmod`**, **26,954 B**,
`sha256 3e9b3e82e3ea0c7dccee62b821d37467101b95f5d0599ad546acbc18978cb98b`, **32 entries**
(`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, 110 content
files under `assets/`/`data/`, `src/{lib,runtime,config}.rs`, `uses.json`). No `Cargo.lock`
(trap 2), no `.horizon/`, no `target/`.

### Traps hit in this migration

1. **The retired paths refuse the package.** `mixins/`, the `.hnmod` and `content/` are not
   package paths; they were moved to evidence before the first build, so the tool never saw
   them.
2. **`Cargo.lock` changes the artifact.** Any `cargo test`/`cargo clippy` writes one, and
   `cargo horizon-mod build` packs it — the first package in this run was **28,795 B**
   (`4d05f3c6…`) with a lock and the stale template README; deleting the lock and rewriting the
   README produced a 26,370 B artifact, moving the config to `register_configs` a 26,907 B one,
   and the last doc-comment edit the recorded 26,954 B one. Always `rm -f Cargo.lock` immediately before the build.
3. **`cargo fmt --all` reports the fetched SDK.** Use `cargo fmt --check` (this crate only).
4. **`pub mod runtime;` matters.** As the old crate's lib root its items were the public API;
   inside a private module every one warns `dead_code`. The file itself stays byte-identical.
5. **`cargo horizon-mod check` with no file checks wasm** and fails without the
   `wasm32-unknown-unknown` target; the package check is `check <file.hmod>`, the native check
   is `cargo check`/`cargo test`.
6. **`build-slot.sh` does not change directory.** Its `--` command runs from the campaign
   root, so either `--manifest-path` or `bash -c 'cd <port> && …'`; without that, `cargo` finds
   no manifest and (worse) `cargo horizon-mod` would fetch an SDK into the campaign root.
   Its `check`/`test` steps also try to forward to the second machine, whose copy of a port
   directory only exists after a sync — `BUILD_LOCAL=1` is the supported local run.
7. **The handle surface, not the host traits.** Host-area *reads* exist as `host.*` methods,
   but entity/level/living writes are **handle methods** (`entity.discard(host)`,
   `living.set_health(host, …)`, `level.spawn_mob(host, …)`) forwarded to a hidden backend;
   `host.entity_kind(..)`/`host.entity_push(..)` do not exist. `Hooks` and `ItemPart` also
   need different attribute macros (`horizon_mod` / `horizon_part`), and a Part must implement
   the generated `item_subscribed`, which the macro writes.
8. **A mod's own config file is a Config-phase read.** `config_files()` refuses (panics,
   naming the phase) outside the Config and World phases, so `server_about_to_start` — the
   obvious place, and the one the retired bundle used — is the wrong hook for it. The SDK's
   surface for this is `register_configs` (push the default text; the game writes the file) +
   `config_loading` (parse the text it read); a world-phase read is only for the lazy re-read.
9. **A `Part` is not the `Mod`.** The macro boxes one `Default` `Mod` for the hooks, and the
   registrar takes an `Arc<dyn ItemPart>` built in `register`; state shared between them has
   to be a process-wide `Mutex`/`OnceLock`, not a field.

### Gaps after the migration

**Closed by the SDK (were gaps in the sections above):**

- *Blade damage source*: `context.damage_source` exists and the fireworks rule is implemented.
  What remains is that **this port's own blade hit is not labelled `minecraft:fireworks`** —
  it still kills through `kill_mob` (the verified 2026-11-10 path) instead of
  `entity_hurt(entity, "minecraft:fireworks", 150.0, …)`, which the SDK now offers. Using it
  would make the label faithful; it is **untested here**, so the verified path stays and this
  is the next slice's first item.
- *`blade_held_by` can never be true*: replaced with `context.weapon_item(host)`, which is the
  supported read (`ARCHITECTURE.md` §3 may drop the row when this bundle is recorded).

**Still open (unchanged by the format):**

- **The in-game e2e has not been re-run in the new format.** `horizon-launcher` is not
  installed on this machine, and a native `.hmod` can only be run from a game the launcher
  built (`horizon-launcher native-build . --trust-native-code`); `cargo horizon-mod run` is
  content-only. `tools/e2e.sh` keeps the whole scenario and refuses without
  `LEGACY_HARNESS=1`. The last verified in-game runs (pin `1d63e0a`, four legs, `PASS` +
  `EXIT:0` and two negative legs `EXIT:1`) remain the behaviour's evidence; §"Migration to the
  SDK format" claims **no new in-game result**.
- **Spawn stage.** Upstream converts during `FinalizeSpawnEvent`; the SDK's `finalize_spawn`
  hands a read-only `UnspawnedMobRef` whose edits are equipment, attributes, tags and the
  custom name — it cannot change the entity type — and its host is read-only, so a spawn
  cannot be replaced there. The conversion therefore still runs in `mob_spawned`
  (post-add) and removes/re-spawns the mob, which also converts `/summon` (a port behaviour,
  not upstream's). `natural: false` covers spawners and eggs too, so gating on it would
  under-convert.
- **Modded skeleton subclasses** match by the four vanilla ids (`is_skeleton_family`), not by
  the class; the API exposes kinds, not Java inheritance.
- **Blade alias ids** `wstweaks:blaze_blade` / `wstweaks:lava_blade` cannot be declared (the
  content format has no aliases).
- **Config keys** other than `Shard Drop Chance` and `Give Bows`: `Durability`,
  `Attack Damage`, `Attack Speed` are fixed content components; `Delete Swords` and
  `Convert All Biomes` are not modelled (defaults true / false).
- **No log surface**: the config read and its reloads are silent (fidelity change 5).
- **Immolation Blade stats** stay content components (the runtime cannot change item
  components).

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
