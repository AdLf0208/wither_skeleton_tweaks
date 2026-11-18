# Validation

Validated on Linux with Rust 1.95.0 on 2026-11-10, using
Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (`game.rs` SHA-256
`64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f`) and the
real bundle committed here.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Pass, 0 warnings |
| `cargo test --workspace --locked --offline` | 9/9 runtime unit tests pass |
| `python3 tools/horizon.py stage` | Pinned `1d63e0a5` export stages; exit 0 |
| `python3 tools/horizon.py check` | `cargo_exit: 0`, 0 warnings in `target/horizon-check.log` |
| `python3 tools/horizon.py test` | `game::wstweaks::tests`: 9 passed, 1240 filtered; `cargo_exit: 0` |
| `python3 tools/horizon.py build` (fast) | `cargo_exit: 0`, 0 warnings |
| `cargo-horizon-mod horizon-mod build` in `content/` | rebuilt `.hmod` byte-identical to the delivered file |
| `python3 tools/package.py` | `wstweaks.hnmod` 17,254 B, SHA-256 `25db76a2…0edfe`, 10 entries; reproduces the recorded bytes |
| `content/wstweaks.hmod` | 8,049 B, SHA-256 `6d10ecac…7c8d86`, 24 entries |
| `tools/e2e.sh` (default) | `PASS` + `EXIT:0` (`_shared/evidence/wstweaks/e2e-default.log`) |
| `NEGATIVE=1 tools/e2e.sh` | `FAIL: NEGATIVE: the Nether skeleton converted…` + `EXIT:1` (fail-capable) |
| `EMBEDDED=1 tools/e2e.sh` | `PASS` + `EXIT:0` (content from the bundle, no `--dev-mod`) |
| `EMBEDDED=1 NEGATIVE=1 tools/e2e.sh` | `FAIL` + `EXIT:1` |
| `python3 ports/tools/check-e2e-logs.py wstweaks --strict` | 5 logs, 0 missing/bad exit statuses, exit 0 |
| `python3 ports/tools/upstream_ready.py wstweaks` | READY (MIT), exit 0 |

The earlier "e2e blocked" state is resolved: both hypotheses in the brief (the
embedded content's load path, and a pin behaviour change) were tested and
refuted, and the red runs' only remaining variables were the harness's random
world and its nearest-skeleton target selection. `tools/e2e.sh` now seeds the
world, builds its own arena in both levels and targets the skeleton it just
summoned; the assertion is unchanged. The full measurement, the four legs and
the one gap it exposed (`blade_held_by` cannot fire at this pin) are in
[notes.md](notes.md), section "The blade-behead red runs (2026-11-10)".

All builds/tests/e2e went through `ports/tools/build-slot.sh` with
`CARGO_INCREMENTAL=0`, no bare `cargo`, no `--target-dir` and no private target
dir. The e2e legs ran in one lease on a fast binary built from these sources,
against a scratch `HORIZON_HOME`; no player data was touched.
