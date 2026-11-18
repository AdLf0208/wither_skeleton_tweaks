# Wither Skeleton Tweaks — a Horizon port

**Wither Skeleton Tweaks** is a Minecraft mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Shadows-of-Fire/Wither-Skeleton-Tweaks](https://github.com/Shadows-of-Fire/Wither-Skeleton-Tweaks) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Wither Skeleton Tweaks ([source](https://github.com/Shadows-of-Fire/Wither-Skeleton-Tweaks)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `wstweaks.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **Wither Skull Fragments** — a wither skeleton that does not drop a Wither
- **Skeleton Conversion** — any skeleton **or stray** that spawns in the
- **Drop Cleanup** — stone swords and bows are removed from the drops of
- **Immolation Blade** — a sword that ignites its target for 150 seconds and
- no `--seed`: every run generated a new world, and the transcripts show it —
- the beheading target was "the first skeleton within 10 blocks", not the one
- nothing in the phase reported *which* entity it hit, so a red run could not be
- **Reviewer:** [P1] upstream MIT notice not shipped

## Not yet ported

- **Converted-mob equipment:** upstream converts in place and, with
- **Blade damage source:** upstream's beheading deals a 150-damage
- **Blade config keys:** only `Shard Drop Chance` is modelled. `Durability`,
- **Blade alias ids:** upstream registers `wstweaks:blaze_blade` and
- **Guaranteed skull for other skeleton-family mods:** the port matches the
- **Spawn stage:** upstream converts during `FinalizeSpawnEvent`
- Duck-typing note: the drop cleanup matches item ids, so a datapack that
- *Blade damage source*: `context.damage_source` exists and the fireworks rule is implemented.

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `wstweaks.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Wither Skeleton Tweaks was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Shadows-of-Fire/Wither-Skeleton-Tweaks

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run wstweaks` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/wstweaks/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
