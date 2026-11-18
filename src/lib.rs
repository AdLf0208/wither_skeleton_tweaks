//! Wither Skeleton Tweaks for Horizon.
//!
//! Ported from Wither Skeleton Tweaks 10.1.1 (mod id `wstweaks`, MIT) for Minecraft Java
//! 1.21.1 NeoForge; `notes.md` records the behaviour, the gaps and the evidence, and
//! `ports/_shared/upstream/wstweaks/FINDINGS.md` the upstream comparison.
//!
//! The pure rules live in [`runtime`] and are unit-tested by `cargo test`; this file is the
//! glue only — the one `impl Hooks`, the Immolation Blade's [`ItemPart`] and the item keys
//! the rules compare. Four moments (five hook methods):
//!
//! - `register_configs` + `config_loading`: `config/wstweaks.toml` (`Shard Drop Chance`,
//!   `Give Bows`), written with its defaults when it is missing.
//! - `mob_spawned`: a skeleton or stray that spawned in the Nether becomes a wither skeleton.
//! - `loot_drops`: skeleton weapons are removed, a blade kill guarantees its skull, and a
//!   wither skeleton without a skull drops the fragment.
//! - the blade item's `hurt_enemy`: ignite for 150 s and behead the skeleton family.
//!
//! The carried content (`assets/`, `data/`) is in this same package: the fragment and the
//! Immolation Blade are registered beside the code, so there is no split half to load.
pub mod config;
pub mod runtime;

use std::sync::Mutex;
use std::sync::OnceLock;

use mod_api::{
    self, ConfigKind, ConfigLoadingEvent, EntityRef, EntityTypeKey, EquipmentSlot, Hooks, Host,
    HostSide, HurtEnemyEvent, ItemKey, ItemPart, ItemStack, LootDropsEvent, MobSpawnedEvent,
    RandomSource, RegisterConfigsEvent, Registrar, RegistrationContext, ServerHost, SpawnReason,
};

/// How far a just-spawned mob may be from `mob_spawned`'s position and still be the one it
/// names: a metre of slack, squared. The callback's position is the spawn point and a
/// just-spawned mob has not moved yet.
const SPAWN_SLACK_SQUARED: f64 = 1.0;

/// A blade kill stays marked for the guaranteed skull for this long: the pin's own
/// remembered player credit is 100 ticks.
const BLADE_MARK_TICKS: i64 = 100;

/// This mod's id, which the manifest and the config file both use.
const MOD_ID: &str = "wstweaks";

/// The one damage type that guarantees a skull besides the blade itself
/// (`WSTLootModifier.java:33-42`).
const FIREWORKS: &str = "minecraft:fireworks";

/// The keys the runtime's rules compare, resolved once at registration.
///
/// The runtime names items by id (`minecraft:bow`), while the API identifies them by
/// `ItemKey`; this is the translation table, and it is why [`named_id`] can answer for the
/// three ids the rules test and for nothing else.
struct Ids {
    skeleton: EntityTypeKey,
    stray: EntityTypeKey,
    blade: ItemKey,
    fragment: ItemKey,
    stone_sword: ItemKey,
    bow: ItemKey,
    wither_skull: ItemKey,
    skeleton_skull: ItemKey,
}

static IDS: OnceLock<Ids> = OnceLock::new();

/// The mobs a blade kill has beheaded, so `loot_drops` can add the guaranteed skull when the
/// loot rolls after the hit (a delayed death's damage source is the fire, not the blade).
static BLADE_KILLS: Mutex<Vec<(u32, i64)>> = Mutex::new(Vec::new());

#[derive(Default)]
struct Mod;

/// The Immolation Blade's own behaviour: attached to `wstweaks:immolation_blade` as a Part,
/// so the game calls it only for that item (the retired bundle's global `hurt_enemy` helper
/// had to check the stack itself).
struct Blade;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Resolve the ids the rules compare and attach the blade's Part. The content items are
    /// declared by this same package, and a requested key is validated after all
    /// declarations, so a missing item refuses the mod instead of failing silently.
    fn register(&self, r: &mut dyn Registrar, _context: &RegistrationContext) {
        let ids = Ids {
            skeleton: r.entity_type_key(runtime::SKELETON),
            stray: r.entity_type_key(runtime::STRAY),
            blade: r.item_key(runtime::IMMOLATION_BLADE),
            fragment: r.item_key(runtime::SKULL_FRAGMENT),
            stone_sword: r.item_key(runtime::STONE_SWORD),
            bow: r.item_key(runtime::BOW),
            wither_skull: r.item_key(runtime::WITHER_SKULL),
            skeleton_skull: r.item_key(runtime::SKELETON_SKULL),
        };
        r.item_part(ids.blade, Blade);
        let _ = IDS.set(ids);
    }

    /// Register `config/wstweaks.toml` as this mod's server config: the game writes
    /// `default` there when the file is missing (NeoForge's `registerConfig`), once per world.
    fn register_configs(&self, _host: HostSide, event: &mut RegisterConfigsEvent<'_>) {
        let default = runtime::format_config(&runtime::Config::default());
        event.configs.push(
            mod_api::Config::new(MOD_ID, ConfigKind::Server, &default).file(config::CONFIG_FILE),
        );
    }

    /// The game read a config: ours it is `wstweaks`, parse it. Every mod hears every config,
    /// so the id is checked; `Client` and `Common` load once, `Server` before each world.
    fn config_loading(&self, _host: HostSide, event: &mut ConfigLoadingEvent<'_>) {
        if event.config.mod_id == MOD_ID {
            config::loaded(&event.config.text);
        }
    }

    /// A mob joined the world. Upstream converts during `FinalizeSpawnEvent`, which cannot
    /// change a mob's type here (the lend is read-only and edits only equipment, attributes,
    /// tags and the name), so the mob this callback names is removed without a death and the
    /// wither skeleton is spawned in its place with `Conversion` as the reason.
    fn mob_spawned(&self, host: &mut dyn ServerHost, event: &mut MobSpawnedEvent<'_>) {
        let Some(ids) = IDS.get() else {
            return;
        };
        let Some(target) = runtime::conversion_target(event.dimension, event.kind) else {
            return;
        };
        let Some(level) = host.level(event.dimension) else {
            return;
        };
        let pos = event.pos;
        let min = [pos[0] - 1.0, pos[1] - 1.0, pos[2] - 1.0];
        let max = [pos[0] + 1.0, pos[1] + 1.0, pos[2] + 1.0];
        let Some(candidates) = level.entities_in(host, min, max) else {
            return;
        };
        let Some(spawned) = nearest(candidates, ids, host, pos) else {
            return;
        };
        // `Entity.discard`: no death, no loot, no experience — the skeleton that would
        // have dropped nothing is replaced, not killed.
        if !spawned.discard(host) {
            return;
        }
        let Some(replacement) = level.spawn_mob(host, target, pos, SpawnReason::Conversion) else {
            return;
        };
        if !config::current(host).give_bows {
            return;
        }
        if let Some(living) = host.living(replacement)
            && let Some(bow) = ItemStack::new(ids.bow, 1)
        {
            let _ = living.set_equipment(host, EquipmentSlot::MainHand, Some(bow));
        }
    }

    /// Drops are being rolled (a mob died or a block broke): the mod's say on the list. Only
    /// a mob's drops are touched — `context.entity` names it; a block break names none.
    fn loot_drops(&self, host: &mut dyn ServerHost, event: &mut LootDropsEvent<'_>) {
        let Some(ids) = IDS.get() else {
            return;
        };
        let Some((entity, kind)) = event.context.entity else {
            return;
        };
        let drops = &mut *event.drops;
        // Stone swords and bows, damaged or not (`WSTEvents.java:52-56`).
        drops.retain(|stack| match named_id(ids, stack.item()) {
            Some(id) => !runtime::is_cleaned_drop(kind, &drop_view(id, stack)),
            None => true,
        });
        // The guaranteed skull: the mark covers a death after the hit, the live weapon and
        // upstream's fireworks rule cover the hit itself.
        let level = host.level(event.context.dimension);
        let marked = level
            .and_then(|level| level.game_time(host))
            .is_some_and(|now| take_blade_kill(entity, now));
        let beheaded =
            marked || blade_in_hand(ids, event.context, host) || fireworks_kill(event.context);
        if beheaded
            && let Some(skull) = runtime::guaranteed_skull(kind)
            && let Some(key) = skull_key(ids, skull)
            && !drops.iter().any(|stack| stack.item() == key)
            && let Some(stack) = ItemStack::new(key, 1)
        {
            drops.push(stack);
        }
        // The fragment: a wither skeleton that dropped no skull, on a passing `<=` roll.
        let Some(roll) =
            level.and_then(|level| level.with_random(host, |random| random.next_float()))
        else {
            return;
        };
        let present: Vec<runtime::Drop> = drops
            .iter()
            .filter_map(|stack| named_id(ids, stack.item()).map(|id| drop_view(id, stack)))
            .collect();
        let chance = config::current(host).shard_drop_chance;
        if runtime::fragment_for(kind, &present, chance, roll)
            && !drops.iter().any(|stack| stack.item() == ids.fragment)
            && let Some(stack) = ItemStack::new(ids.fragment, 1)
        {
            drops.push(stack);
        }
    }
}

#[mod_api::horizon_part]
impl ItemPart for Blade {
    /// A player's hit landed on `event.entity` while holding the blade: ignite it for 150 s
    /// and, for the skeleton family, set it to one health, kill it (its loot rolls, and the
    /// mark guarantees the skull) and shove it (`ImmolationBladeItem.java:19-32`).
    fn hurt_enemy(&self, host: &mut dyn ServerHost, event: &mut HurtEnemyEvent<'_>) {
        let Some(entity) = host.entity(event.entity) else {
            return;
        };
        let hit = runtime::blade_hit(event.kind);
        entity.set_remaining_fire_ticks(host, hit.fire_ticks as i32);
        if !hit.behead {
            return;
        }
        let level = entity.level(host);
        if let Some(now) = level.and_then(|level| level.game_time(host)) {
            mark_blade_kill(event.entity, now);
        }
        if let Some(living) = host.living(entity) {
            let _ = living.set_health(host, 1.0);
        }
        let dimension = host.dimension().to_owned();
        let shove = level.and_then(|level| {
            level.with_random(host, |random| {
                runtime::blade_shove(
                    f64::from(random.next_float()),
                    f64::from(random.next_float()),
                    f64::from(random.next_float()),
                )
            })
        });
        // `/kill`'s path: it dies, and its loot rolls through `loot_drops` above, which the
        // mark has prepared to hand it the skull.
        let _ = host.kill_mob(&dimension, event.entity);
        if let Some(shove) = shove {
            // A kill can leave the handle stale; a refused shove is cosmetic only.
            let _ = entity.push(host, shove);
        }
    }
}

/// The id a rule may compare this key as, or `None` for an item the rules do not name.
fn named_id(ids: &Ids, key: ItemKey) -> Option<&'static str> {
    if key == ids.stone_sword {
        Some(runtime::STONE_SWORD)
    } else if key == ids.bow {
        Some(runtime::BOW)
    } else if key == ids.wither_skull {
        Some(runtime::WITHER_SKULL)
    } else {
        None
    }
}

fn skull_key(ids: &Ids, skull: &str) -> Option<ItemKey> {
    match skull {
        runtime::WITHER_SKULL => Some(ids.wither_skull),
        runtime::SKELETON_SKULL => Some(ids.skeleton_skull),
        _ => None,
    }
}

/// The runtime's view of a dropped stack.
fn drop_view(id: &str, stack: &ItemStack) -> runtime::Drop {
    runtime::Drop {
        item: id.to_owned(),
        count: stack.count(),
        damage: stack.damage(),
    }
}

/// The skeleton or stray this callback names: the closest one at the spawn point, inside the
/// slack. Another mob at the same spot is not the one.
fn nearest(
    candidates: Vec<EntityRef>,
    ids: &Ids,
    host: &dyn Host,
    pos: [f64; 3],
) -> Option<EntityRef> {
    let mut best: Option<(f64, EntityRef)> = None;
    for entity in candidates {
        let Some(kind) = entity.kind(host) else {
            continue;
        };
        if kind != ids.skeleton && kind != ids.stray {
            continue;
        }
        let Some(distance) = entity.distance_to_sqr(host, pos) else {
            continue;
        };
        if distance > SPAWN_SLACK_SQUARED {
            continue;
        }
        if best.is_none_or(|(closest, _)| distance < closest) {
            best = Some((distance, entity));
        }
    }
    best.map(|(_, entity)| entity)
}

/// Whether the recorded weapon of this death is the blade (a melee kill while it is held).
fn blade_in_hand(ids: &Ids, context: &mod_api::LootDrops<'_>, host: &dyn Host) -> bool {
    context
        .weapon_item(host)
        .is_some_and(|stack| stack.item() == ids.blade)
}

/// Upstream's other guaranteed skull: a `minecraft:fireworks` kill (`WSTLootModifier.java:33`).
fn fireworks_kill(context: &mod_api::LootDrops<'_>) -> bool {
    context
        .damage_source
        .is_some_and(|source| source.damage_type == FIREWORKS)
}

fn mark_blade_kill(id: u32, now: i64) {
    let mut marks = BLADE_KILLS
        .lock()
        .expect("the wstweaks blade-mark lock is held for one short write");
    marks.retain(|(_, at)| now - *at <= BLADE_MARK_TICKS);
    marks.push((id, now));
}

fn take_blade_kill(id: u32, now: i64) -> bool {
    let mut marks = BLADE_KILLS
        .lock()
        .expect("the wstweaks blade-mark lock is held for one short write");
    let Some(index) = marks
        .iter()
        .position(|(marked, at)| *marked == id && now - *at <= BLADE_MARK_TICKS)
    else {
        return false;
    };
    marks.remove(index);
    true
}
