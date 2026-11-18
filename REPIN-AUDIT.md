# Wither Skeleton Tweaks — re-pin audit (Horizon `83ad384f`)

**Read-only audit.** No port code, build, stage or e2e was run for this file.
Every claim is a static reading of the new pin; a realizable verdict still needs
the ladder and e2e to become evidence.

- Pin: `83ad384f48d0742746d518737511882680aab5b7` (427 hooks, 159 helpers),
  `ports/_shared/pins/horizon-83ad384f`.
- Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`; planner schema still **6**.
- Port baseline: `c9d474dd…` / mixins `d9c83d70…`; slice 2 verified in game.
- Paths relative to the pin root unless they start with `ports/`.
- Staging note: both pin directories are real git checkouts of the exact
  commits (`git cat-file -e <sha>^{commit}` succeeds), so
  `--horizon <campaign>/ports/_shared/pins/horizon-83ad384f` and
  `HORIZON_MIXINS=<campaign>/ports/_shared/pins/horizon-mixins-e8cd0aa`
  satisfy `tools/horizon.py`'s checkout checks. `compatibility.json` must name
  both revisions before `stage`.

## Deferred features and what the new pin offers

| # | Deferred feature (`notes.md` "Gaps") | Needs | Available at `83ad384f` | Class | Verdict |
|---|---|---|---|---|---|
| 1 | Converted mob is bare; upstream gives a fresh bow (`Give Bows`) or keeps the weapon | spawn/equip a mob with equipment | `native_summon(dimension, kind, pos, nbt)` — `crates/horizon-client/src/game/native_entities.rs:141` (**API 59**, full `/summon` NBT incl. `equipment`/`drop_chances`); `native_living_set_equipment` — `native_living_refs.rs:122` (**API 382**, every mob slot); read with `native_living_equipment` — `native_living_refs.rs:73`; `native_entity_nbt` — `native_entity_data.rs:16` (**API 386**) to copy the original's name/passengers/persistence | **usable** (write) | **realizable now** |
| 2 | Blade deals a `minecraft:fireworks` damage source | create a typed damage / read it in `loot_drops` | `native_loot_drops` context still has no damage source (`crates/horizon-client/src/mod_api.rs:135-172`, `tool` only at `:155`); `native_hurt_enemy` — `native_hooks.rs:7601` (**API 39**) hears the hit, no type; `native_living_death` — `native_hooks.rs:6045` (**API 392**) *reports* `damage_type`; a mod's own loot kind sees `NativeLootContext.damage.kind` — `crates/horizon-world/src/loot.rs:106,362` via `native_loot_condition`/`native_loot_function` — `loot.rs:70,84` (**API 133**); **no helper deals typed damage** | **absent** to create; **hook-only** to read | **still blocked** (outcome already matches) |
| 3 | Blade stats (`Durability`, `Attack Damage`, `Attack Speed`) were fixed content components | change an item's default components at runtime | `native_modify_default_components(item, components)` — `crates/horizon-world/src/item_defaults.rs:77` (**API 79**; `items.json` shape, insert/replace/remove, asked on every read) | **usable** (helper) | **realizable now** |
| 4 | Alias ids `wstweaks:blaze_blade` / `wstweaks:lava_blade` | extra ids with the same behaviour | plain content can declare three item files; `native_items` — `native_hooks.rs:2942` (**API 145**) can declare them from code. A true **alias** (same id) does not exist | **usable** (distinct ids) | **realizable now** (not a literal alias) |
| 5 | Only vanilla skeleton ids (`is_skeleton_family`), not modded `AbstractSkeleton` subclasses | class/hierarchy or a type tag | `native_entity_kind` — `native_entity_refs.rs:76` (**API 373**); tag helpers exist only for **item**/**block** tags (`native_server_item_in_tag` — `native_world_read.rs:79`, **API 63**) — no entity-type tag test, no class hierarchy | **absent** (kind list only) | **still blocked** (config list is the only fallback) |
| 6 | Spawn stage: post-add `mob_spawned`, not `FinalizeSpawn`; replacement loses name/passengers/persistence/goals | edit the mob before its first tick | `native_entity_join_level` — `native_hooks.rs:7262` (**API 47**, entity id + kind + pos, `cancel`) fires at `addEntity` and *before the mob ticks*; `native_entity_nbt` (386) copies the saved compound to the replacement. There is still no pre-add finalize hook | **hook-only** (pre-tick join) + **usable** for the NBT copy | **partially realizable** (losses fixed, timing still post-add) |
| 7 | Duck-typed drop cleanup (a modded bow id is not cleaned) | tag-aware item matching | `native_server_item_in_tag` — `native_world_read.rs:79` (**API 63**) can test `c:tools/bow`-style tags while filtering `drops` in `loot_drops` | **usable** | **realizable now** (optional; content change) |

Counts: deferred **7** · newly realizable **4** (#1 equipment, #3 blade stats,
#4 alias ids as distinct items, #7 tag-aware cleanup) · partially realizable
**1** (#6 spawn stage) · still blocked **2** (#2 damage source, #5 modded
skeletons).

## Detail

### 1. Converted-mob equipment — the old blocker is gone

The slice-2 note said `NativeSpawn` has no equipment field and there is no
mob-equipment write. Both halves are now available:

- `native_summon` takes the mob's full compound:
  ```rust
  // native_entities.rs:141 — "as /summon <kind> <pos> <nbt> takes it"
  pub(super) fn native_summon(&mut self, dimension: &str, kind: &str, pos: [f64; 3], nbt: Compound) -> Option<u32>
  ```
  so `equipment:{mainhand:{id:"minecraft:bow",count:1}},drop_chances:{mainhand:2.0f}`
  spawns the wither skeleton already holding the bow (the NBT shape the port's
  own e2e already writes for `/summon`).
- or spawn bare and call `native_living_set_equipment(native_living(native_entity(id))?,
  EquipmentSlot::MainHand, Some(bow))` — a mob has every slot
  (`native_living_refs.rs:104-122`).

**Next step (concrete).** In `spawn.rs`/`support.rs`, when converting, read the
original's saved compound with `native_entity_nbt`, strip its id, insert
`equipment.mainhand` (a fresh bow when `give_bows`), and pass it to
`native_summon`; when `give_bows = false`, carry the original's
`equipment.mainhand` instead. Unit-test the pure
`conversion_nbt(original, give_bows) -> Compound` in `runtime.rs`; e2e: a Nether
skeleton converts and drops a bow when killed with `drop_chances` forced, while
the `give_bows = false` run does not (differential).

### 3. Blade config keys — `modify_default_components`

The content carries `max_damage` 4096, `attack_damage` 11 and `attack_speed` -2
as fixed components. API 79 lets the mod answer the item's defaults at load,
per item, in `items.json` form (`item_defaults.rs:65-77`), so a config read at
`server_about_to_start` can drive them:

```rust
// item_defaults.rs:77
pub fn native_modify_default_components(item: &str, components: &mut Map<String, Value>)
```

**Next step.** Model `durability`, `attack_damage`, `attack_speed` in
`config/wstweaks.toml` (the parser already exists), and subscribe
`modify_default_components` to write `minecraft:max_damage` and
`minecraft:attribute_modifiers` for `wstweaks:immolation_blade` and its two new
ids. e2e: set durability 50, `ctl` shows the stack's max damage 50; a control
run with the default shows 4096.

### 4. Alias ids

There is no id alias in the content schema, but the *observable* need is that an
old world naming `wstweaks:blaze_blade` gets a working blade. Declaring
`blaze_blade`/`lava_blade` as own content item files (same template, model and
components) satisfies that; `native_items` (API 145) is only needed if the port
prefers code-declared ids. Stacks of the three ids do not merge — say so in the
notes.

### 6. Spawn stage

`entity_join_level` (`native_hooks.rs:7244-7272`) is told at
`PersistentEntitySectionManager.addEntity` with the entity id and kind, *before
the mob's first tick*, and can cancel. That is strictly better than
`mob_spawned` ("the mob is already among the game's mobs", `native_hooks.rs:7219`)
for two reasons: the original has not ticked yet, and its id is available so
`native_entity_nbt` can copy name, passengers, persistence and equipment to the
replacement. `FinalizeSpawnEvent` still has no equivalent, so a `/summon` still
converts (a port behaviour, as today).

**Next step.** Move the conversion subscription from `mob_spawned` to
`entity_join_level` (skip `loaded_from_disk`, guard against recursion with the
`wither_skeleton` kind), copy the NBT as in #1, and keep the e2e's
`/summon`-based scenario (note the deliberate deviation).

### 2 and 5 — still blocked

- **Blade damage source:** the death-loot context *does* carry
  `LootDamage.kind` (`horizon-world/src/loot.rs:362`) and a mod's own loot
  condition sees it (API 133), but the port cannot *produce* a
  `minecraft:fireworks` hit: no helper applies typed damage
  (`native_set_mob_health` 276 / `native_kill_mob` 293 are untyped,
  `native_hurt_enemy` only hears). A content condition on a wither-skeleton loot
  table would therefore never see `fireworks` from this port. The current
  mark-in-`hurt_enemy` + skull-in-`loot_drops` outcome stays.
- **Modded skeleton subclasses:** no class hierarchy and no entity-type tag
  helper. A config list of extra ids (`skeletons = ["rats:…"]`) is the only
  honest fallback; keep the vanilla four as the default.

## Highest-value next step

**#1, the fresh bow (and the NBT copy that comes with it).** It is the gap the
notes call out twice ("the replacement is bare", "losing its name, passengers,
persistence and goals"), it is one `native_summon` change plus a pure
`conversion_nbt` helper, and it has a clean differential (`give_bows` on/off).
Do it together with #6 (subscribe `entity_join_level`) since both touch the same
path. Then #3 (config-driven blade stats) and #7 (tag-aware cleanup). #2 and #5
stay recorded as pin limits.
