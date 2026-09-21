//! A fight said in the game's own terms: the loadout a player carries through
//! the door, and which door.
//!
//! A [`CombatSetup`] is the small, versioned document any producer can write
//! — a live run projecting the fight it stands in, a fold over a recorded
//! run's per-floor stats, a hand-written file — and the engine stands it up
//! through its own combat start, so nothing a fight's opening does is
//! skipped. It deliberately carries no RNG state, no opening hand, no enemy
//! health rolls, no map and no grab bags: all of those are the engine's to
//! generate, which is what lets one loadout serve many hands and lets a bank
//! of setups outlive every internal the engine cares to change.
//!
//! The complement is [`ScenarioBuilder`](crate::ScenarioBuilder), which says
//! everything and derives nothing — the right tool for a rule test that
//! needs turn three of a specific fight, and the wrong one for a bank.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sts2_core::ModelId;

use crate::{
    CardFingerprint, ConformanceOrigin, EnchantmentFingerprint, EngineError, ErrorCode,
    MapPointType, RelicInstance, RunPhase, Simulator,
};

/// The version of the setup document this build writes and reads.
pub const COMBAT_SETUP_VERSION: u32 = 1;

/// What kind of room an encounter is met in. The engine reads it to count
/// the visit against the act's pools, to scale the fight's rewards, and to
/// tell an elite's relic from a hallway's gold.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupRoom {
    Monster,
    Elite,
    Boss,
}

impl SetupRoom {
    /// The map point a fight of this kind stands on.
    #[must_use]
    pub const fn map_point_type(self) -> MapPointType {
        match self {
            Self::Monster => MapPointType::Monster,
            Self::Elite => MapPointType::Elite,
            Self::Boss => MapPointType::Boss,
        }
    }

    /// The kind of fight a room of this type holds. A fight met anywhere
    /// else — an unknown point that turned into a fight, an event's fight —
    /// is a hallway fight.
    #[must_use]
    pub const fn of(point: Option<MapPointType>) -> Self {
        match point {
            Some(MapPointType::Elite) => Self::Elite,
            Some(MapPointType::Boss) => Self::Boss,
            _ => Self::Monster,
        }
    }
}

/// One card of the deck as a setup names it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetupCard {
    pub id: ModelId,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub upgrade: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enchantment: Option<SetupEnchantment>,
    /// The saved properties the card carries at values other than the ones
    /// a fresh card of its model holds. Almost always empty: a projection
    /// writes only what differs from fresh, and the constructor fills the
    /// rest in.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, Value>,
}

/// An enchantment on a card, and how much of it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetupEnchantment {
    pub id: ModelId,
    pub amount: i32,
}

/// One relic as a setup names it.
///
/// `counter` is one of three things, decided by the relic's own definition:
/// a cycle count, which `null` rolls below the relic's wrap threshold; a
/// ledger, which a producer either knows or does not, so `null` is refused;
/// or nothing, for a relic that keeps no count, where the field is ignored.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetupRelic {
    pub id: ModelId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<i32>,
}

/// A fight's entry: who walks in, carrying what, through which door.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CombatSetup {
    pub setup_version: u32,
    pub compatibility_id: String,
    pub character: ModelId,
    pub ascension: u8,
    /// The act the fight stands in, counted from zero as the engine counts
    /// acts.
    pub act: usize,
    pub floor: u32,
    pub encounter: ModelId,
    pub room: SetupRoom,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    /// Energy the player opens each turn with. `null` keeps what the run
    /// derives for the character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_energy: Option<i32>,
    /// Orb slots the player opens the fight with. `null` keeps what the run
    /// derives for the character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orb_slots: Option<i32>,
    pub deck: Vec<SetupCard>,
    pub relics: Vec<SetupRelic>,
    /// The belt, slot by slot: the list is the belt's size, and `null` is an
    /// empty slot.
    pub potions: Vec<Option<ModelId>>,
    /// Where the setup came from, in the producer's own words. Carried, never
    /// read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Value>,
}

#[allow(clippy::trivially_copy_pass_by_ref, reason = "serde's signature")]
const fn is_zero(value: &u8) -> bool {
    *value == 0
}

/// How a relic's saved count carries between fights, which decides what a
/// setup that does not say the count means.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterKind {
    /// A count that winds on and wraps at a threshold — attacks played toward
    /// the tenth, turns seen toward the third. Standing anywhere below the
    /// threshold is a state the run can be in, so an unsaid count is rolled
    /// below it.
    Cycle { wraps_at: i32 },
    /// A fact about the run — lifts taken, elites put down, fights of use
    /// left. A producer either knows it or does not, and an unsaid one is
    /// refused rather than guessed.
    Ledger,
}

fn refused(detail: impl Into<Value>) -> EngineError {
    EngineError::new(ErrorCode::InvalidConfig).with_detail("combat_setup", detail)
}

fn unknown(kind: &str, model_id: &ModelId) -> EngineError {
    EngineError::new(ErrorCode::UnsupportedContent)
        .with_detail("model_id", model_id.to_string())
        .with_detail("location", format!("combat setup: {kind}"))
}

impl Simulator {
    /// The fight this simulator stands in, said as a setup: the loadout the
    /// run carries and the door it walked through. `None` outside a live
    /// fight inside a run — on a reward screen the fight is over, and a
    /// scenario fight has no run to say which act or floor it is on.
    ///
    /// What is written is what the constructor reads back: a card's saved
    /// properties only where they differ from a fresh card's, a relic's
    /// count only where the relic keeps one. The exact hand, the enemies'
    /// rolled health and every stream position are deliberately not here.
    #[must_use]
    pub fn combat_setup(&self) -> Option<CombatSetup> {
        let state = self.state();
        let combat = state.combat.as_ref()?;
        let run = state.run.as_ref()?;
        if !combat.in_progress || combat.has_ended {
            return None;
        }
        let player = &state.run_player;
        let character = run
            .config
            .players
            .iter()
            .find(|config| config.slot == player.slot)
            .map(|config| config.character.clone())?;
        // In a fight the player's hit points live on their creature.
        let hp = combat
            .creatures
            .iter()
            .find(|creature| creature.id == combat.player.creature_id)
            .map_or(player.current_hp, |creature| creature.current_hp);
        let deck = player
            .deck
            .iter()
            .map(|fingerprint| {
                let fresh = self.content().fresh_card(&fingerprint.model_id);
                SetupCard {
                    id: fingerprint.model_id.clone(),
                    upgrade: fingerprint.upgrade_level,
                    enchantment: fingerprint.enchantment.as_ref().map(|enchantment| {
                        SetupEnchantment {
                            id: enchantment.model_id.clone(),
                            amount: enchantment.amount,
                        }
                    }),
                    properties: fingerprint
                        .properties
                        .iter()
                        .filter(|(name, value)| fresh.properties.get(*name) != Some(*value))
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect(),
                }
            })
            .collect();
        let relics = player
            .relics
            .iter()
            .map(|relic| SetupRelic {
                id: relic.model_id.clone(),
                counter: self
                    .content()
                    .relic(&relic.model_id)
                    .is_some_and(|definition| definition.counter_property.is_some())
                    .then_some(relic.counter),
            })
            .collect();
        Some(CombatSetup {
            setup_version: COMBAT_SETUP_VERSION,
            compatibility_id: state.compatibility_id.clone(),
            character,
            ascension: run.config.ascension,
            act: run.current_act,
            floor: run.floor,
            encounter: combat.encounter_model.clone(),
            room: SetupRoom::of(run.standing_room_type()),
            hp,
            max_hp: player.max_hp,
            gold: player.gold,
            max_energy: Some(player.max_energy),
            orb_slots: Some(player.base_orb_slots),
            deck,
            relics,
            potions: player.potions.clone(),
            provenance: Some(serde_json::json!({
                "origin": "live_run",
                "seed": run.config.seed,
                "floor": run.floor,
            })),
        })
    }

    /// A simulator standing at the first decision of the setup's fight, its
    /// run otherwise this one's: the loadout replaced with the setup's, the
    /// act and floor set, and the encounter entered through the same path a
    /// map step takes — the deck shuffled on the run's own stream, enemy
    /// health rolled, every on-entry and on-combat-start hook dispatched.
    /// Nothing inside the combat state is written by hand.
    ///
    /// This simulator must stand in a run, outside a live fight, on a run
    /// that has not ended; whatever room it was standing in — a reward
    /// screen, a shop, an event page — is left the way walking on leaves it.
    /// The setup must be for this run's character at this run's ascension,
    /// on this build, and every model it names must be one this run's
    /// content knows; anything else is refused with the field named, and
    /// this simulator is not touched either way.
    ///
    /// A world built this way is analysis, never fidelity evidence: it
    /// cannot be scripted into the real game or replayed against a
    /// recording, and it says so by its origin.
    pub fn enter_combat_setup(&self, setup: &CombatSetup) -> Result<Self, EngineError> {
        self.refuse_unless_enterable(setup)?;
        let deck = self.setup_deck(setup)?;
        let seed = self
            .state()
            .run
            .as_ref()
            .map(|run| run.config.seed.clone())
            .unwrap_or_default();
        let relics = self.setup_relics(setup, &seed)?;
        for potion in setup.potions.iter().flatten() {
            if self.content().potion(potion).is_none() {
                return Err(unknown("potion", potion));
            }
        }
        // From here on nothing can fail for a reason the setup owns, so the
        // clone is taken and rewritten.
        let mut state = self.state().clone();
        state.combat = None;
        state.origin = ConformanceOrigin::Analysis;
        {
            let player = &mut state.run_player;
            player.current_hp = setup.hp;
            player.max_hp = setup.max_hp;
            player.gold = setup.gold;
            if let Some(energy) = setup.max_energy {
                player.max_energy = energy;
            }
            if let Some(slots) = setup.orb_slots {
                player.base_orb_slots = slots;
            }
            player.deck = deck;
            player.relics = relics;
            player.potions.clone_from(&setup.potions);
        }
        let point_type = setup.room.map_point_type();
        walk_onto(
            state.run.as_mut().expect("checked above"),
            setup,
            point_type,
        );
        let mut simulator = Self::from_scenario(state, self.content_handle())?;
        let definition = simulator.generate_encounter_room(&setup.encounter)?;
        {
            let run = simulator.run_mut().expect("checked above");
            run.current_room_model = definition.model_id().cloned();
            run.current_room_id = run.next_room_id;
            run.next_room_id += 1;
        }
        simulator.enter_room(definition)?;
        simulator.rebuild_boundary()?;
        Ok(simulator)
    }

    /// Everything about this run and the setup's own fields that decides
    /// whether the setup can be entered from here, refused with the field
    /// named; the models the setup names are checked as they are built.
    fn refuse_unless_enterable(&self, setup: &CombatSetup) -> Result<(), EngineError> {
        let state = self.state();
        let run = state
            .run
            .as_ref()
            .ok_or_else(|| refused("a setup is entered from a run"))?;
        if state.terminal.is_some() {
            return Err(refused("the run has ended"));
        }
        if state
            .combat
            .as_ref()
            .is_some_and(|combat| combat.in_progress && !combat.has_ended)
        {
            return Err(refused("the run stands inside a fight"));
        }
        if setup.setup_version != COMBAT_SETUP_VERSION {
            return Err(refused(format!(
                "setup_version {} is not the {COMBAT_SETUP_VERSION} this build reads",
                setup.setup_version
            )));
        }
        if setup.compatibility_id != state.compatibility_id {
            return Err(EngineError::new(ErrorCode::UnsupportedBuild)
                .with_detail("compatibility_id", setup.compatibility_id.clone()));
        }
        let character = run
            .config
            .players
            .iter()
            .find(|config| config.slot == state.run_player.slot)
            .map(|config| &config.character)
            .ok_or_else(|| refused("the run names no player for its slot"))?;
        if setup.character != *character {
            return Err(refused(format!(
                "the setup is for {}, the run plays {character}",
                setup.character
            )));
        }
        if setup.ascension != run.config.ascension {
            return Err(refused(format!(
                "the setup is at ascension {}, the run is at {}",
                setup.ascension, run.config.ascension
            )));
        }
        if setup.act >= run.acts.len() {
            return Err(refused(format!(
                "act {} is past the {} acts the run holds",
                setup.act,
                run.acts.len()
            )));
        }
        if setup.hp < 1 || setup.hp > setup.max_hp {
            return Err(refused(format!(
                "hp {} is outside 1..={}",
                setup.hp, setup.max_hp
            )));
        }
        if self.content().encounter(&setup.encounter).is_none() {
            return Err(unknown("encounter", &setup.encounter));
        }
        Ok(())
    }

    /// The deck a setup names, each card checked against this run's content
    /// and filled in the way a fresh card is.
    fn setup_deck(&self, setup: &CombatSetup) -> Result<Vec<CardFingerprint>, EngineError> {
        setup
            .deck
            .iter()
            .map(|card| {
                if self.content().card(&card.id).is_none() {
                    return Err(unknown("card", &card.id));
                }
                let mut fingerprint = self.content().fresh_card(&card.id);
                fingerprint.upgrade_level = card.upgrade;
                fingerprint.floor_added_to_deck = None;
                if let Some(enchantment) = &card.enchantment {
                    if self.content().enchantment(&enchantment.id).is_none() {
                        return Err(unknown("enchantment", &enchantment.id));
                    }
                    fingerprint.enchantment = Some(EnchantmentFingerprint {
                        model_id: enchantment.id.clone(),
                        amount: enchantment.amount,
                        properties: BTreeMap::new(),
                    });
                }
                for (name, value) in &card.properties {
                    fingerprint.properties.insert(name.clone(), value.clone());
                }
                if !self.content().supports_card(&fingerprint) {
                    return Err(unknown("card", &card.id));
                }
                Ok(fingerprint)
            })
            .collect()
    }

    /// The relics a setup names, each checked against this run's content,
    /// its count settled by the relic's own kind.
    fn setup_relics(
        &self,
        setup: &CombatSetup,
        seed: &str,
    ) -> Result<Vec<RelicInstance>, EngineError> {
        setup
            .relics
            .iter()
            .enumerate()
            .map(|(index, relic)| {
                let definition = self
                    .content()
                    .relic(&relic.id)
                    .ok_or_else(|| unknown("relic", &relic.id))?;
                let mut instance = RelicInstance::new(relic.id.clone());
                if definition.counter_property.is_none() {
                    // A relic that keeps no saved count stands at what it
                    // was obtained holding, whatever the setup says.
                    instance.counter = definition.initial_counter;
                    return Ok(instance);
                }
                instance.counter = match (relic.counter, definition.counter_kind) {
                    (Some(counter), _) => counter,
                    (None, Some(CounterKind::Cycle { wraps_at })) if wraps_at > 0 => {
                        // Rolled off the run's seed and the relic's place on
                        // the belt: deterministic, and never off a stream
                        // the fight will spend.
                        let hash =
                            sts2_rng::deterministic_hash(&format!("{seed}:{}:{index}", relic.id));
                        hash.rem_euclid(wraps_at)
                    }
                    (None, Some(CounterKind::Cycle { .. }) | None) => {
                        return Err(refused(format!(
                            "{} keeps a count this build has not classified; say it",
                            relic.id
                        )));
                    }
                    (None, Some(CounterKind::Ledger)) => {
                        return Err(refused(format!(
                            "{} keeps a ledger, which a setup has to say",
                            relic.id
                        )));
                    }
                };
                Ok(instance)
            })
            .collect()
    }
}

/// Walks the run onto the setup's floor the way a map step does: whatever
/// room it was standing in is left, and everything a settling screen still
/// owed is dropped, then the act, the floor and the point are set and the
/// visit joins the history.
fn walk_onto(run: &mut crate::RunState, setup: &CombatSetup, point_type: MapPointType) {
    // Walking on ends whatever the run was standing in, exactly as a map
    // step does, and everything a settling screen still owed.
    run.active_room = None;
    run.pushed_room = None;
    run.begun_event = None;
    run.reward_set = None;
    run.dismissed_reward_set = None;
    run.rest_option_owed = None;
    run.rest_options_owed.clear();
    run.suspended_choice = None;
    run.pending.clear();
    run.owed_by_rewards.clear();
    run.purchase_owed_by_rewards = None;
    run.finish_event_after_rewards = false;
    run.finish_event_after_pending = false;
    run.current_act = setup.act;
    run.floor = setup.floor;
    run.previous_room_type = run.current_room_type;
    run.current_room_type = Some(point_type);
    // The fight stands on a point of its kind where the act's map has
    // one, so the walk on from it is a walk from somewhere.
    let coord = run.maps.get(setup.act).and_then(|map| {
        map.points
            .iter()
            .find(|point| point.point_type == point_type)
            .map(|point| point.coord)
    });
    run.current_coord = coord;
    if let Some(coord) = coord
        && !run.visited.contains(&coord)
    {
        run.visited.push(coord);
    }
    run.next_room_id = 0;
    run.phase = RunPhase::Room;
    let act = setup.act;
    if run
        .map_history
        .last()
        .is_none_or(|history| history.act != act)
    {
        run.map_history.push(crate::ActHistory {
            act,
            points: Vec::new(),
        });
    }
    if let Some(history) = run.map_history.last_mut() {
        let floor = history.points.len() as u32;
        history.points.push(crate::MapPointVisit {
            floor,
            point_type,
            rooms: Vec::new(),
        });
    }
}
