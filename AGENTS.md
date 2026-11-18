# Working on this Horizon mod (SDK layout, 2026-11-18)

This port is **one flat `.hmod`**: `horizon-mod.toml` + `Cargo.toml` + `src/` (the one
`impl Hooks`) + `assets/` + `data/`. The `.hnmod` container, `mixins/<id>/`, the plan and the
injection schema are **retired** (Horizon's mod-API merge; see `notes.md`
§"Migration to the SDK format"). Do not recreate them.

Read `README.md` and `notes.md` first. Keep this repository separate from Horizon,
horizon-mixins and the launcher; the only things fetched are the pinned SDK (`.horizon/sdk/`,
gitignored, never packed) and the cargo cache. Do not commit binaries, the SDK or Minecraft
assets.

- **The pure rules stay in `src/runtime.rs`** and must stay byte-identical to the recorded
  file: it has no `mod_api` type and is what `cargo test` exercises. The rules go there; the
  glue (hosts, events, handles) goes in `src/lib.rs` / `src/config.rs`.
- **Read the surface, do not guess it.** `cargo horizon-mod hooks <text>` searches the catalog
  offline; the generated `Hooks`, every event struct and every handle method are in
  `.horizon/sdk/api/crates/horizon-mod-vocab/generated/mod_api_generated{,_host}.rs`.
- **A moment that belongs to one item (or block, entity, menu) is a Part**, attached in
  `register` (`r.item_part(key, part)`), not a global hook that checks the stack.
- Run everything through the campaign slot registry (`../tools/build-slot.sh run --job <id>
  --port ports/wstweaks --step <step> -- bash -c 'cd ports/wstweaks && …'`), which blocks
  until a slot is free; `BUILD_LOCAL=1` keeps a compile step on this machine. Never run a bare
  `tools/e2e.sh`, and never start a build or an e2e in the background.
- Ladder: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `rm -f Cargo.lock`, `cargo horizon-mod build` (copy `target/horizon-mod/wstweaks.hmod` to the
  port root), `cargo horizon-mod check wstweaks.hmod`, `cargo horizon-mod fits wstweaks.hmod
  --build nightly`. Deleting `Cargo.lock` before the build is required: it is packed and
  changes the artifact's SHA-256.
- An in-game check needs a game the **launcher** built
  (`horizon-launcher native-build . --trust-native-code`); `cargo horizon-mod run` is
  content-only. `tools/e2e.sh` holds the scenario and refuses without `LEGACY_HARNESS=1`.
- Keep validation claims and supported revisions honest: the SHA-256 and the `check` line in
  `notes.md` are what reviewers reproduce.
