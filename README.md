# Wither Skeleton Tweaks — Horizon port

One flat SDK package (`.hmod`): code and carried content together. Ported from
**Wither Skeleton Tweaks 10.1.1** (`wstweaks`, MIT) for Minecraft Java 1.21.1
NeoForge; the port targets Minecraft 26.3 on Horizon's pinned build.

**LOCAL-ONLY.** MIT (upstream, Copyright (c) 2019 Brennan Ward), derivatives
allowed, but never published without the maintainer's approval
(campaign `ARCHITECTURE.md` §8).

## What it does

| Behaviour | Source |
|---|---|
| A skeleton or stray that spawns in the Nether becomes a wither skeleton (a fresh bow in its hand when `give_bows`) | `WSTEvents.java:23-42` |
| Stone swords and bows are removed from every skeleton-family drop, damaged or not | `WSTEvents.java:52-56` |
| A wither skeleton that drops no skull drops `wstweaks:fragment`, gated by `Shard Drop Chance` | `WSTLootModifier.java:44-48` |
| The Immolation Blade ignites its target for 150 s and beheads the skeleton family (guaranteed skull) | `ImmolationBladeItem.java:19-32` |

## Layout

```text
wstweaks.hmod            the delivered package (built by `cargo horizon-mod build`)
horizon-mod.toml         the only manifest: id, version, api = "0.2", [native] pins
Cargo.toml               one crate; depends only on horizon-mod-api from .horizon/sdk/
src/lib.rs               the one `impl Hooks`, the blade's `ItemPart`, the item keys
src/runtime.rs           the pure rules and their 9 tests — no mod_api types
src/config.rs            config/wstweaks.toml through the host's config service
assets/wstweaks/         item definitions, models, textures, lang
data/wstweaks/           item declarations, recipes, creative tab
data/minecraft/tags/     the swords tag entry
notes.md                 behaviour, deviations, gaps, verification, migration recipe
tools/e2e.sh             the in-game scenario (retired staging harness; see its header)
```

## Build and check

```sh
cargo test                                  # the pure rules, no game needed
cargo clippy --all-targets -- -D warnings
rm -f Cargo.lock                            # the SDK crate adds none; a lock changes the hash
cargo horizon-mod build                     # -> target/horizon-mod/wstweaks.hmod
cp target/horizon-mod/wstweaks.hmod ./wstweaks.hmod
cargo horizon-mod check wstweaks.hmod
cargo horizon-mod fits wstweaks.hmod --build nightly
```

A native mod is compiled into a game by the launcher
(`horizon-launcher native-build . --trust-native-code`); `cargo horizon-mod run`
is content-only. `notes.md` §Migration to the SDK format records the commands run
for this package and the evidence paths.
