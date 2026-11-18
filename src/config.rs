//! The port's config file, `config/wstweaks.toml`.
//!
//! Upstream reads `Shard Drop Chance` and `Give Bows` from Placebo's config, which NeoForge
//! loads through `ModContainer.registerConfig` and announces with `ModConfigEvent.Loading`.
//! The SDK has the same pair (`register_configs` writes the default file when it is missing,
//! `config_loading` hands over the text in the Config phase), so this module holds only the
//! parsed value.
//!
//! Upstream reloads on a datapack reload; there is no such moment for a mod's own file, so the
//! file is re-read when its modification time changes — `config_files()` is available in the
//! world phase, where every mob death and loot roll runs.
use std::sync::Mutex;
use std::sync::MutexGuard;

use mod_api::{ConfigPath, Host};

use crate::runtime;

/// The config file's name, relative to the game's `config/` folder.
pub const CONFIG_FILE: &str = "wstweaks.toml";

/// The config in force, with the file state it was read from.
struct Cached {
    config: runtime::Config,
    /// The file's modification time in milliseconds, when it was last looked at. `None` after
    /// a load, so the first world-phase look re-reads once.
    modified_millis: Option<u64>,
}

static CONFIG: Mutex<Option<Cached>> = Mutex::new(None);

fn cached() -> MutexGuard<'static, Option<Cached>> {
    CONFIG
        .lock()
        .expect("the wstweaks config lock is held for one short read")
}

/// Takes the text the game loaded for this mod (`config_loading`).
pub fn loaded(text: &str) {
    *cached() = Some(Cached {
        config: runtime::parse_config(text),
        modified_millis: None,
    });
}

/// The config in force, re-read when the file changed since the last look.
pub fn current(host: &dyn Host) -> runtime::Config {
    let Ok(path) = ConfigPath::new(CONFIG_FILE) else {
        return runtime::Config::default();
    };
    let mut guard = cached();
    let Some(state) = guard.as_mut() else {
        return runtime::Config::default();
    };
    let modified_millis = modified(host, &path);
    if modified_millis == state.modified_millis {
        return state.config;
    }
    if let Ok(bytes) = host.config_files().read(&path) {
        state.config = runtime::parse_config(&String::from_utf8_lossy(&bytes));
    }
    state.modified_millis = modified_millis;
    state.config
}

fn modified(host: &dyn Host, path: &ConfigPath) -> Option<u64> {
    host.config_files()
        .stat(path)
        .ok()
        .flatten()
        .map(|stat| stat.modified_millis)
}
