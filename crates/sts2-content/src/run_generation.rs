use sts2_core::ModelId;
use sts2_engine::{ActOneSetting, ActRoomPools, EngineError, ErrorCode, RelicGrabBagState};
use sts2_rng::{Rng, RunRngSet, RunStream};

use crate::acts::pinned_act_selection;

const SHARED_EVENTS: [&str; 18] = [
    "EVENT.BRAIN_LEECH",
    "EVENT.CRYSTAL_SPHERE",
    "EVENT.DOLL_ROOM",
    "EVENT.FAKE_MERCHANT",
    "EVENT.POTION_COURIER",
    "EVENT.RANWID_THE_ELDER",
    "EVENT.RELIC_TRADER",
    "EVENT.ROOM_FULL_OF_CHEESE",
    "EVENT.SELF_HELP_BOOK",
    "EVENT.SLIPPERY_BRIDGE",
    "EVENT.STONE_OF_ALL_TIME",
    "EVENT.SYMBIOTE",
    "EVENT.TEA_MASTER",
    "EVENT.THE_FUTURE_OF_POTIONS",
    "EVENT.THE_LEGENDS_WERE_TRUE",
    "EVENT.THIS_OR_THAT",
    "EVENT.WAR_HISTORIAN_REPY",
    "EVENT.WELCOME_TO_WONGOS",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpFrontGeneration {
    pub acts: Vec<ActRoomPools>,
    pub shared_relics: RelicGrabBagState,
    pub player_relics: RelicGrabBagState,
    pub up_front_counter: u64,
}

#[derive(Clone, Copy)]
struct Encounter {
    id: &'static str,
    tags: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct ActDefinition {
    id: &'static str,
    events: &'static [&'static str],
    weak: &'static [Encounter],
    normal: &'static [Encounter],
    elite: &'static [Encounter],
    bosses: &'static [&'static str],
    ancients: &'static [&'static str],
    room_count: usize,
    weak_count: usize,
}

const OVERGROWTH_EVENTS: [&str; 13] = [
    "EVENT.AROMA_OF_CHAOS",
    "EVENT.BYRDONIS_NEST",
    "EVENT.DENSE_VEGETATION",
    "EVENT.JUNGLE_MAZE_ADVENTURE",
    "EVENT.LUMINOUS_CHOIR",
    "EVENT.MORPHIC_GROVE",
    "EVENT.SAPPHIRE_SEED",
    "EVENT.SUNKEN_STATUE",
    "EVENT.TABLET_OF_TRUTH",
    "EVENT.UNREST_SITE",
    "EVENT.WELLSPRING",
    "EVENT.WHISPERING_HOLLOW",
    "EVENT.WOOD_CARVINGS",
];
const OVERGROWTH_WEAK: [Encounter; 4] = [
    encounter("ENCOUNTER.FUZZY_WURM_CRAWLER_WEAK", &["crawler"]),
    encounter("ENCOUNTER.NIBBITS_WEAK", &["nibbit"]),
    encounter("ENCOUNTER.SHRINKER_BEETLE_WEAK", &["shrinker"]),
    encounter("ENCOUNTER.SLIMES_WEAK", &["slimes"]),
];
const OVERGROWTH_NORMAL: [Encounter; 12] = [
    encounter("ENCOUNTER.CUBEX_CONSTRUCT_NORMAL", &[]),
    encounter("ENCOUNTER.FLYCONID_NORMAL", &["mushroom", "slimes"]),
    encounter("ENCOUNTER.FOGMOG_NORMAL", &[]),
    encounter("ENCOUNTER.INKLETS_NORMAL", &[]),
    encounter("ENCOUNTER.MAWLER_NORMAL", &[]),
    encounter("ENCOUNTER.NIBBITS_NORMAL", &[]),
    encounter("ENCOUNTER.OVERGROWTH_CRAWLERS", &["shrinker", "crawler"]),
    encounter("ENCOUNTER.RUBY_RAIDERS_NORMAL", &[]),
    encounter("ENCOUNTER.SLIMES_NORMAL", &["slimes"]),
    encounter(
        "ENCOUNTER.SLITHERING_STRANGLER_NORMAL",
        &["jaxfruit", "slimes"],
    ),
    encounter(
        "ENCOUNTER.SNAPPING_JAXFRUIT_NORMAL",
        &["mushroom", "jaxfruit"],
    ),
    encounter("ENCOUNTER.VINE_SHAMBLER_NORMAL", &[]),
];
const OVERGROWTH_ELITE: [Encounter; 3] = [
    encounter("ENCOUNTER.BYGONE_EFFIGY_ELITE", &[]),
    encounter("ENCOUNTER.BYRDONIS_ELITE", &[]),
    encounter("ENCOUNTER.PHROG_PARASITE_ELITE", &[]),
];

const HIVE_EVENTS: [&str; 10] = [
    "EVENT.AMALGAMATOR",
    "EVENT.BUGSLAYER",
    "EVENT.COLORFUL_PHILOSOPHERS",
    "EVENT.COLOSSAL_FLOWER",
    "EVENT.FIELD_OF_MAN_SIZED_HOLES",
    "EVENT.INFESTED_AUTOMATON",
    "EVENT.LOST_WISP",
    "EVENT.SPIRIT_GRAFTER",
    "EVENT.THE_LANTERN_KEY",
    "EVENT.ZEN_WEAVER",
];
const HIVE_WEAK: [Encounter; 4] = [
    encounter("ENCOUNTER.BOWLBUGS_WEAK", &["workers"]),
    encounter("ENCOUNTER.EXOSKELETONS_WEAK", &["exoskeletons"]),
    encounter("ENCOUNTER.THIEVING_HOPPER_WEAK", &["thieves"]),
    encounter("ENCOUNTER.TUNNELER_WEAK", &["burrower"]),
];
const HIVE_NORMAL: [Encounter; 10] = [
    encounter("ENCOUNTER.BOWLBUGS_NORMAL", &["workers"]),
    encounter("ENCOUNTER.CHOMPERS_NORMAL", &["chomper"]),
    encounter("ENCOUNTER.EXOSKELETONS_NORMAL", &["exoskeletons"]),
    encounter("ENCOUNTER.HUNTER_KILLER_NORMAL", &[]),
    encounter("ENCOUNTER.LOUSE_PROGENITOR_NORMAL", &[]),
    encounter("ENCOUNTER.MYTES_NORMAL", &[]),
    encounter("ENCOUNTER.OVICOPTER_NORMAL", &[]),
    encounter("ENCOUNTER.SLUMBERING_BEETLE_NORMAL", &["workers"]),
    encounter("ENCOUNTER.SPINY_TOAD_NORMAL", &[]),
    encounter("ENCOUNTER.THE_OBSCURA_NORMAL", &[]),
];
const HIVE_ELITE: [Encounter; 3] = [
    encounter("ENCOUNTER.DECIMILLIPEDE_ELITE", &[]),
    encounter("ENCOUNTER.ENTOMANCER_ELITE", &[]),
    encounter("ENCOUNTER.INFESTED_PRISMS_ELITE", &[]),
];

const GLORY_EVENTS: [&str; 7] = [
    "EVENT.BATTLEWORN_DUMMY",
    "EVENT.GRAVE_OF_THE_FORGOTTEN",
    "EVENT.HUNGRY_FOR_MUSHROOMS",
    "EVENT.REFLECTIONS",
    "EVENT.ROUND_TEA_PARTY",
    "EVENT.TRIAL",
    "EVENT.TINKER_TIME",
];
const GLORY_WEAK: [Encounter; 3] = [
    encounter("ENCOUNTER.DEVOTED_SCULPTOR_WEAK", &[]),
    encounter("ENCOUNTER.SCROLLS_OF_BITING_WEAK", &["scrolls"]),
    encounter("ENCOUNTER.TURRET_OPERATOR_WEAK", &[]),
];
const GLORY_NORMAL: [Encounter; 9] = [
    encounter("ENCOUNTER.AXEBOTS_NORMAL", &[]),
    encounter("ENCOUNTER.CONSTRUCT_MENAGERIE_NORMAL", &[]),
    encounter("ENCOUNTER.FABRICATOR_NORMAL", &[]),
    encounter("ENCOUNTER.FROG_KNIGHT_NORMAL", &[]),
    encounter("ENCOUNTER.GLOBE_HEAD_NORMAL", &[]),
    encounter("ENCOUNTER.OWL_MAGISTRATE_NORMAL", &[]),
    encounter("ENCOUNTER.SCROLLS_OF_BITING_NORMAL", &["scrolls"]),
    encounter("ENCOUNTER.SLIMED_BERSERKER_NORMAL", &[]),
    encounter("ENCOUNTER.THE_LOST_AND_FORGOTTEN_NORMAL", &[]),
];
const GLORY_ELITE: [Encounter; 3] = [
    encounter("ENCOUNTER.KNIGHTS_ELITE", &["knights"]),
    encounter("ENCOUNTER.MECHA_KNIGHT_ELITE", &[]),
    encounter("ENCOUNTER.SOUL_NEXUS_ELITE", &[]),
];

const UNDERDOCKS_EVENTS: [&str; 10] = [
    "EVENT.ABYSSAL_BATHS",
    "EVENT.DROWNING_BEACON",
    "EVENT.ENDLESS_CONVEYOR",
    "EVENT.PUNCH_OFF",
    "EVENT.SPIRALING_WHIRLPOOL",
    "EVENT.SUNKEN_STATUE",
    "EVENT.SUNKEN_TREASURY",
    "EVENT.DOORS_OF_LIGHT_AND_DARK",
    "EVENT.TRASH_HEAP",
    "EVENT.WATERLOGGED_SCRIPTORIUM",
];
const UNDERDOCKS_WEAK: [Encounter; 4] = [
    encounter("ENCOUNTER.CORPSE_SLUGS_WEAK", &["slugs"]),
    encounter("ENCOUNTER.SEAPUNK_WEAK", &["seapunk"]),
    encounter("ENCOUNTER.SLUDGE_SPINNER_WEAK", &[]),
    encounter("ENCOUNTER.TOADPOLES_WEAK", &[]),
];
const UNDERDOCKS_NORMAL: [Encounter; 10] = [
    encounter("ENCOUNTER.CORPSE_SLUGS_NORMAL", &["slugs"]),
    encounter("ENCOUNTER.CULTISTS_NORMAL", &[]),
    encounter("ENCOUNTER.FOSSIL_STALKER_NORMAL", &[]),
    encounter("ENCOUNTER.GREMLIN_MERC_NORMAL", &[]),
    encounter("ENCOUNTER.HAUNTED_SHIP_NORMAL", &[]),
    encounter("ENCOUNTER.LIVING_FOG_NORMAL", &[]),
    encounter("ENCOUNTER.PUNCH_CONSTRUCT_NORMAL", &[]),
    encounter("ENCOUNTER.SEAPUNK_NORMAL", &["seapunk"]),
    encounter("ENCOUNTER.SEWER_CLAM_NORMAL", &[]),
    encounter("ENCOUNTER.TWO_TAILED_RATS_NORMAL", &[]),
];
const UNDERDOCKS_ELITE: [Encounter; 3] = [
    encounter("ENCOUNTER.PHANTASMAL_GARDENERS_ELITE", &[]),
    encounter("ENCOUNTER.SKULKING_COLONY_ELITE", &[]),
    encounter("ENCOUNTER.TERROR_EEL_ELITE", &[]),
];

const ACTS: [ActDefinition; 4] = [
    ActDefinition {
        id: "ACT.OVERGROWTH",
        events: &OVERGROWTH_EVENTS,
        weak: &OVERGROWTH_WEAK,
        normal: &OVERGROWTH_NORMAL,
        elite: &OVERGROWTH_ELITE,
        bosses: &[
            "ENCOUNTER.CEREMONIAL_BEAST_BOSS",
            "ENCOUNTER.THE_KIN_BOSS",
            "ENCOUNTER.VANTOM_BOSS",
        ],
        ancients: &["EVENT.NEOW"],
        room_count: 15,
        weak_count: 3,
    },
    // The other act one. It draws from its own tables, and the shapes it shares
    // with the overgrowth -- fifteen rooms and three weak encounters -- are the
    // act's own declarations rather than anything about the index.
    ActDefinition {
        id: "ACT.UNDERDOCKS",
        events: &UNDERDOCKS_EVENTS,
        weak: &UNDERDOCKS_WEAK,
        normal: &UNDERDOCKS_NORMAL,
        elite: &UNDERDOCKS_ELITE,
        // The act's boss encounters, in the order the game's encounter list
        // declares them. The boss roll indexes into that order.
        bosses: &[
            "ENCOUNTER.LAGAVULIN_MATRIARCH_BOSS",
            "ENCOUNTER.SOUL_FYSH_BOSS",
            "ENCOUNTER.WATERFALL_GIANT_BOSS",
        ],
        ancients: &["EVENT.NEOW"],
        room_count: 15,
        weak_count: 3,
    },
    ActDefinition {
        id: "ACT.HIVE",
        events: &HIVE_EVENTS,
        weak: &HIVE_WEAK,
        normal: &HIVE_NORMAL,
        elite: &HIVE_ELITE,
        bosses: &[
            "ENCOUNTER.KAISER_CRAB_BOSS",
            "ENCOUNTER.KNOWLEDGE_DEMON_BOSS",
            "ENCOUNTER.THE_INSATIABLE_BOSS",
        ],
        ancients: &["EVENT.OROBAS", "EVENT.PAEL", "EVENT.TEZCATARA"],
        room_count: 14,
        weak_count: 2,
    },
    ActDefinition {
        id: "ACT.GLORY",
        events: &GLORY_EVENTS,
        weak: &GLORY_WEAK,
        normal: &GLORY_NORMAL,
        elite: &GLORY_ELITE,
        bosses: &[
            "ENCOUNTER.AEONGLASS_BOSS",
            "ENCOUNTER.QUEEN_BOSS",
            "ENCOUNTER.TEST_SUBJECT_BOSS",
        ],
        ancients: &["EVENT.NONUPEIPE", "EVENT.TANX", "EVENT.VAKUU"],
        room_count: 13,
        weak_count: 2,
    },
];

const fn encounter(id: &'static str, tags: &'static [&'static str]) -> Encounter {
    Encounter { id, tags }
}

/// The acts this crate carries room tables for: an act model lives here
/// rather than in the content registry, so this is its listing.
#[must_use]
pub fn implemented_act_ids() -> Vec<ModelId> {
    ACTS.iter().map(|act| model(act.id)).collect()
}

/// Every event this act's own room pool can be built out of: the act's own
/// list and the shared one, which is what `shuffled_events` draws from.
#[must_use]
pub fn act_event_ids(act_id: &ModelId) -> Vec<ModelId> {
    ACTS.iter()
        .filter(|act| act.id == act_id.to_string())
        .flat_map(|act| act.events.iter().chain(&SHARED_EVENTS))
        .copied()
        .map(model)
        .collect()
}

fn model(value: &str) -> ModelId {
    value.parse().expect("pinned model ID is valid")
}

fn shares_tags(left: Encounter, right: Encounter) -> bool {
    left.tags.iter().any(|tag| right.tags.contains(tag))
}

fn random_index(rng: &mut Rng, length: usize) -> usize {
    rng.next_index(length)
        .expect("pinned content list fits game integer")
        .expect("pinned content list is nonempty")
}

/// A draw over one bag that repeats no tag, appending to a list that may
/// already hold what an earlier bag drew.
///
/// The weak encounters and the regular ones are drawn from two bags of their
/// own but into the same list, and the test each draw makes is against the last
/// entry of that list. So the first regular draw of an act will not repeat the
/// last weak one, nor anything tagged like it.
fn draw_encounters_into(
    rng: &mut Rng,
    output: &mut Vec<Encounter>,
    candidates: &'static [Encounter],
    count: usize,
) {
    let target = output.len() + count;
    let mut bag: Vec<Encounter> = Vec::new();
    while output.len() < target {
        if bag.is_empty() {
            bag.extend(candidates.iter().copied());
        }
        let last = output.last().copied();
        let has_valid = bag.iter().any(|candidate| {
            last.is_none_or(|previous| {
                candidate.id != previous.id && !shares_tags(*candidate, previous)
            })
        });
        let selected = if has_valid {
            loop {
                let selected = random_index(rng, bag.len());
                let candidate = bag[selected];
                if last.is_none_or(|previous| {
                    candidate.id != previous.id && !shares_tags(candidate, previous)
                }) {
                    break selected;
                }
            }
        } else {
            random_index(rng, bag.len())
        };
        output.push(bag.remove(selected));
    }
}

fn draw_encounters(rng: &mut Rng, candidates: &'static [Encounter], count: usize) -> Vec<ModelId> {
    let mut output = Vec::with_capacity(count);
    draw_encounters_into(rng, &mut output, candidates, count);
    output.into_iter().map(|entry| model(entry.id)).collect()
}

fn shuffled_events(rng: &mut Rng, act_events: &[&str]) -> Vec<ModelId> {
    let mut events = act_events
        .iter()
        .chain(&SHARED_EVENTS)
        .copied()
        .collect::<Vec<_>>();
    rng.shuffle(&mut events);
    events.into_iter().map(model).collect()
}

/// Generates the room pools for the act list the baseline's own act selection
/// rolls for `seed`.
pub fn standard_act_room_pools(seed: &str) -> Result<UpFrontGeneration, EngineError> {
    let selection =
        pinned_act_selection(seed, ActOneSetting::Random, &model("CHARACTER.IRONCLAD"))?;
    act_room_pools_for(
        seed,
        &selection.acts,
        &selection.character,
        sts2_engine::BASELINE_ASCENSION,
    )
}

/// The room pools of one named act list, after the two relic inventories
/// consume their fixed-size up-front shuffles.
///
/// The act list is an input rather than a constant because act one is rolled:
/// what the up-front stream generates against is whichever acts the run opened
/// with, and an act this baseline has no room tables for stops here.
pub fn act_room_pools_for(
    seed: &str,
    act_ids: &[ModelId],
    character: &ModelId,
    ascension: u8,
) -> Result<UpFrontGeneration, EngineError> {
    act_room_pools_with_history(seed, act_ids, character, ascension, None)
}

/// `None` keeps the rolled bosses (custom mode or the veteran baseline).
pub(crate) fn act_room_pools_with_history(
    seed: &str,
    act_ids: &[ModelId],
    character: &ModelId,
    ascension: u8,
    encounters_seen: Option<&[String]>,
) -> Result<UpFrontGeneration, EngineError> {
    let acts = act_ids
        .iter()
        .map(|act_id| {
            ACTS.iter()
                .find(|act| act.id == act_id.to_string())
                .ok_or_else(|| {
                    EngineError::new(ErrorCode::UnsupportedContent)
                        .with_detail("model_id", act_id.to_string())
                        .with_detail("location", "run generation: act room pools")
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut rngs = RunRngSet::new(seed);
    let rng = rngs.stream_mut(RunStream::UpFront);
    let (shared_relics, player_relics) = crate::relic_pool::generate_relic_bags(rng, character);

    let mut shared_ancients = vec!["EVENT.DARV"];
    rng.shuffle(&mut shared_ancients);
    let mut act_shared_ancients = vec![Vec::new(); acts.len()];
    for shared in act_shared_ancients.iter_mut().skip(1) {
        let count = random_index_inclusive_zero(rng, shared_ancients.len());
        *shared = shared_ancients.drain(..count).collect();
    }

    let last_act = acts.len() - 1;
    let acts = acts
        .iter()
        .enumerate()
        .map(|(index, act)| {
            let events = shuffled_events(rng, act.events);
            // One list, two bags: the weak encounters first and the regular
            // ones after them, each draw tested against the last entry of the
            // list they share.
            let mut drawn = Vec::with_capacity(act.room_count);
            draw_encounters_into(rng, &mut drawn, act.weak, act.weak_count);
            draw_encounters_into(rng, &mut drawn, act.normal, act.room_count - act.weak_count);
            let normal_encounters: Vec<ModelId> =
                drawn.into_iter().map(|entry| model(entry.id)).collect();
            let elite_encounters = draw_encounters(rng, act.elite, 15);
            let mut boss = model(act.bosses[random_index(rng, act.bosses.len())]);
            let mut ancient_candidates = act.ancients.to_vec();
            ancient_candidates.extend(&act_shared_ancients[index]);
            let ancient = model(ancient_candidates[random_index(rng, ancient_candidates.len())]);
            // The game rolls rooms first, then applies discovery without any
            // draws. Double Boss excludes this final boss, not the original roll.
            if let Some(seen) = encounters_seen {
                if let Some(unseen) = sts2_core::BOSS_DISCOVERY_ENCOUNTERS.iter().find(|boss| {
                    act.bosses.contains(boss) && !seen.iter().any(|entry| entry == **boss)
                }) {
                    boss = model(unseen);
                }
            }
            // Under Double Boss the last act of the run draws a second boss
            // out of the ones it did not already roll, on the same up-front
            // stream and immediately after that act's own rooms. Every earlier
            // act reads no dice for it.
            let second_boss = (index == last_act
                && sts2_engine::has_ascension(ascension, sts2_engine::AscensionLevel::DoubleBoss))
            .then(|| {
                let others: Vec<&str> = act
                    .bosses
                    .iter()
                    .copied()
                    .filter(|candidate| model(candidate) != boss)
                    .collect();
                model(others[random_index(rng, others.len())])
            });
            ActRoomPools {
                act_model: model(act.id),
                ancient,
                boss,
                second_boss,
                normal_encounters,
                elite_encounters,
                events,
                normal_encounters_visited: 0,
                elite_encounters_visited: 0,
                events_visited: 0,
                boss_encounters_visited: 0,
            }
        })
        .collect();

    Ok(UpFrontGeneration {
        acts,
        shared_relics,
        player_relics,
        up_front_counter: rng.counter(),
    })
}

fn random_index_inclusive_zero(rng: &mut Rng, length: usize) -> usize {
    rng.next_int(i32::try_from(length + 1).expect("pinned list fits game integer"))
        .expect("inclusive-zero range is nonempty") as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_discovery_keeps_rng_and_other_rooms_and_precedes_double_boss() {
        let character = model("CHARACTER.IRONCLAD");
        for act in &ACTS {
            let ids = [model(act.id)];
            let baseline = act_room_pools_with_history("ABC", &ids, &character, 10, None).unwrap();
            let order: Vec<_> = sts2_core::BOSS_DISCOVERY_ENCOUNTERS
                .iter()
                .filter(|boss| act.bosses.contains(boss))
                .copied()
                .collect();
            for seen_count in 0..=order.len() {
                let seen: Vec<_> = order[..seen_count]
                    .iter()
                    .map(|boss| (*boss).to_owned())
                    .collect();
                let generated =
                    act_room_pools_with_history("ABC", &ids, &character, 10, Some(&seen)).unwrap();
                let expected = order
                    .get(seen_count)
                    .map_or_else(|| baseline.acts[0].boss.clone(), |boss| model(boss));
                let rooms = &generated.acts[0];
                assert_eq!(rooms.boss, expected);
                assert_ne!(rooms.second_boss.as_ref(), Some(&expected));
                assert_eq!(generated.up_front_counter, baseline.up_front_counter);
                assert_eq!(generated.shared_relics, baseline.shared_relics);
                assert_eq!(generated.player_relics, baseline.player_relics);
                assert_eq!(rooms.normal_encounters, baseline.acts[0].normal_encounters);
                assert_eq!(rooms.elite_encounters, baseline.acts[0].elite_encounters);
                assert_eq!(rooms.events, baseline.acts[0].events);
                assert_eq!(rooms.ancient, baseline.acts[0].ancient);
                if seen_count == order.len() {
                    assert_eq!(generated, baseline);
                }
            }
        }
    }
}
