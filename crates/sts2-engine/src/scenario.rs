use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use sts2_core::{CardInstanceId, CreatureCombatId, ModelId, PINNED_COMPATIBILITY_ID};
use sts2_rng::{PlayerRngSet, RunRngSet, RunStream};

use crate::{
    CardFingerprint, CardInstance, CombatSide, CombatState, ContentRegistry, Creature, EngineError,
    GameState, Piles, PlayerCombatState, RewardFingerprint, RunPlayerState, Simulator,
};

/// An enemy placed by a scenario, resolved against content at build time.
#[derive(Clone, Debug)]
struct ScenarioEnemy {
    id: CreatureCombatId,
    model_id: ModelId,
    current_hp: i32,
    max_hp: i32,
    move_state: String,
    slot_name: Option<String>,
    locals: Vec<(String, i32)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PileName {
    Draw,
    Hand,
    Discard,
    Exhaust,
    Play,
}

/// Compact deterministic scenario DSL used by engine and content tests.
#[derive(Clone, Debug)]
pub struct ScenarioBuilder {
    seed: String,
    encounter: ModelId,
    player_model: ModelId,
    player_hp: i32,
    player_max_hp: i32,
    energy: i32,
    max_energy: i32,
    stars: i32,
    turn: u32,
    round: u32,
    piles: Piles,
    enemies: Vec<ScenarioEnemy>,
    rng_counters: BTreeMap<RunStream, u64>,
    rewards: Vec<RewardFingerprint>,
    gold: i32,
    potions: Vec<Option<ModelId>>,
    relics: Vec<crate::RelicInstance>,
    powers: Vec<(CreatureCombatId, ModelId, i32, crate::PowerType)>,
    orb_slots: i32,
    orbs: Vec<ModelId>,
}

impl ScenarioBuilder {
    #[must_use]
    pub fn new(seed: impl Into<String>, encounter: ModelId) -> Self {
        Self {
            seed: seed.into(),
            encounter,
            player_model: "CHARACTER.IRONCLAD".parse().expect("static model ID"),
            player_hp: 80,
            player_max_hp: 80,
            energy: 3,
            max_energy: 3,
            stars: 0,
            turn: 1,
            round: 1,
            piles: Piles::default(),
            enemies: Vec::new(),
            rng_counters: BTreeMap::new(),
            rewards: Vec::new(),
            gold: 0,
            relics: Vec::new(),
            powers: Vec::new(),
            potions: vec![None, None, None],
            orb_slots: 0,
            orbs: Vec::new(),
        }
    }

    /// How many orb slots the player stands with, which a fight normally takes
    /// from the character's own base orb slot count.
    #[must_use]
    pub const fn orb_slots(mut self, slots: i32) -> Self {
        self.orb_slots = slots;
        self
    }

    /// An orb already standing in the queue, front first. Each one is stood up
    /// exactly as channelling it would stand it up, so a scenario can open
    /// with a queue rather than having to channel one.
    #[must_use]
    pub fn orb(mut self, model_id: ModelId) -> Self {
        self.orbs.push(model_id);
        self
    }

    /// Which character the scenario's player is. It decides what a body that
    /// reads the walking player's own pool draws against, and what a card
    /// whose target type is worked out rather than declared looks at.
    #[must_use]
    pub fn played_as(mut self, character: ModelId) -> Self {
        self.player_model = character;
        self
    }

    #[must_use]
    pub const fn player_hp(mut self, current: i32, maximum: i32) -> Self {
        self.player_hp = current;
        self.player_max_hp = maximum;
        self
    }

    /// How many stars the player is standing on. There is no maximum: the
    /// pool is whatever the fight has put into it.
    #[must_use]
    pub const fn stars(mut self, stars: i32) -> Self {
        self.stars = stars;
        self
    }

    #[must_use]
    pub const fn energy(mut self, current: i32, maximum: i32) -> Self {
        self.energy = current;
        self.max_energy = maximum;
        self
    }

    #[must_use]
    pub const fn turn(mut self, turn: u32, round: u32) -> Self {
        self.turn = turn;
        self.round = round;
        self
    }

    #[must_use]
    pub fn shuffle_counter(mut self, counter: u64) -> Self {
        self.rng_counters.insert(RunStream::Shuffle, counter);
        self
    }

    #[must_use]
    pub fn rng_counter(mut self, stream: RunStream, counter: u64) -> Self {
        self.rng_counters.insert(stream, counter);
        self
    }

    #[must_use]
    pub const fn gold(mut self, amount: i32) -> Self {
        self.gold = amount;
        self
    }

    #[must_use]
    pub fn potions(mut self, potions: Vec<Option<ModelId>>) -> Self {
        self.potions = potions;
        self
    }

    /// Relics the player is already holding when the scenario starts.
    #[must_use]
    pub fn relics(mut self, relics: &[ModelId]) -> Self {
        self.relics = relics
            .iter()
            .cloned()
            .map(crate::RelicInstance::new)
            .collect();
        self
    }

    /// The relics the player is holding, each with whatever per-run state the
    /// scenario wants them standing at.
    #[must_use]
    pub fn relic_instances(mut self, relics: Vec<crate::RelicInstance>) -> Self {
        self.relics = relics;
        self
    }

    #[must_use]
    pub fn reward(mut self, reward: RewardFingerprint) -> Self {
        self.rewards.push(reward);
        self
    }

    #[must_use]
    pub fn card(
        mut self,
        pile: PileName,
        id: u64,
        fingerprint: CardFingerprint,
        energy_cost: i32,
    ) -> Self {
        let card = CardInstance {
            id: CardInstanceId::from(id),
            fingerprint,
            energy_cost,
            star_cost: -1,
            costs_x: false,
            captured_x: 0,
            last_stars_spent: 0,
            temporary_star_costs: Vec::new(),
            cost_this_turn: 0,
            cost_this_combat: 0,
            deck_version: None,
            replay_count: 0,
            ethereal: false,
            retain: false,
            sly: false,
            sly_this_turn: false,
            retain_this_turn: false,
            affliction: None,
            damage_bonus: 0,
            temporary_costs: Vec::new(),
            cost_until_played: 0,
            enchantment_status: crate::EnchantmentStatus::Normal,
            enchantment_locals: BTreeMap::new(),
            clone_of: None,
            last_played_turn: None,
            dupe: false,
            locals: std::collections::BTreeMap::new(),
        };
        match pile {
            PileName::Draw => self.piles.draw.push(card),
            PileName::Hand => self.piles.hand.push(card),
            PileName::Discard => self.piles.discard.push(card),
            PileName::Exhaust => self.piles.exhaust.push(card),
            PileName::Play => self.piles.play.push(card),
        }
        self
    }

    /// Puts a power on a creature, in the order the calls are made. Listener
    /// order follows power order, so a test can say which of two listeners the
    /// dispatch reaches first.
    #[must_use]
    pub fn power(
        mut self,
        creature: u64,
        model_id: ModelId,
        amount: i32,
        power_type: crate::PowerType,
    ) -> Self {
        self.powers.push((
            CreatureCombatId::from(creature),
            model_id,
            amount,
            power_type,
        ));
        self
    }

    /// Places an enemy already standing on `move_state`, which must be a move
    /// state of its definition. The shown intent comes from that state, so a
    /// scenario cannot invent an intent the content does not define.
    #[must_use]
    pub fn enemy(
        mut self,
        id: u64,
        model_id: ModelId,
        current_hp: i32,
        max_hp: i32,
        move_state: impl Into<String>,
    ) -> Self {
        self.enemies.push(ScenarioEnemy {
            id: CreatureCombatId::from(id),
            model_id,
            current_hp,
            max_hp,
            move_state: move_state.into(),
            slot_name: None,
            locals: Vec::new(),
        });
        self
    }

    /// The slot the enemy added last stands in: which of its encounter's named
    /// slots it occupies. A scenario that names none leaves every creature at
    /// the slot index an unnamed one gets, which is no index at all.
    #[must_use]
    pub fn in_slot(mut self, slot_name: &str) -> Self {
        if let Some(enemy) = self.enemies.last_mut() {
            enemy.slot_name = Some(slot_name.to_owned());
        }
        self
    }

    /// A number the enemy added last has already written down for itself.
    #[must_use]
    pub fn enemy_local(mut self, name: &str, value: i32) -> Self {
        if let Some(enemy) = self.enemies.last_mut() {
            enemy.locals.push((name.to_owned(), value));
        }
        self
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one line per field of the state it stands up"
    )]
    pub fn build(self, content: Arc<ContentRegistry>) -> Result<Simulator, EngineError> {
        let mut rng = RunRngSet::new(self.seed.clone());
        for (stream, counter) in self.rng_counters {
            rng.stream_mut(stream)
                .fast_forward(counter)
                .expect("scenario counter moves forward");
        }
        let encounter_slots = content
            .encounter(&self.encounter)
            .map(|encounter| encounter.slots.clone())
            .unwrap_or_default();
        let player_id = CreatureCombatId::from(0);
        let mut creatures = vec![Creature {
            slot_name: None,
            pet_owner: None,
            last_spawned: None,
            id: player_id,
            model_id: self.player_model,
            side: CombatSide::Player,
            current_hp: self.player_hp,
            max_hp: self.player_max_hp,
            block: 0,
            unblocked_blows_taken: 0,
            powers: Vec::new(),
            intent: None,
            move_state: String::new(),
            performed_first_move: false,
            spawned_this_turn: false,
            death_handled: false,
            stunned_follow_up: None,
            stunned_move: Vec::new(),
            off_balance: false,
            hp_display: crate::HpDisplay::Normal,
            slot: -1,
            performed_log: Vec::new(),
            locals: std::collections::BTreeMap::new(),
            state_log: Vec::new(),
        }];
        for enemy in self.enemies {
            creatures.push(build_enemy(&enemy, &content, &encounter_slots)?);
        }
        place_powers(&mut creatures, self.powers)?;
        // A card instance takes the keyword its model declares, the same way
        // one dealt into a real combat does.
        let mut piles = self.piles;
        for card in piles.all_cards_mut() {
            if let Some(definition) = content.card(&card.fingerprint.model_id) {
                card.ethereal = definition.ethereal_at(card.fingerprint.upgrade_level);
                card.costs_x = definition.costs_x;
            }
        }
        // Every card the scenario put in a pile is also a card of the run
        // deck, in `all_cards` order, so each of them is the combat's copy of
        // the deck card standing at the same place.
        let deck: Vec<CardFingerprint> = all_cards(&piles)
            .map(|card| card.fingerprint.clone())
            .collect();
        let deck_order: Vec<CardInstanceId> = all_cards(&piles).map(|card| card.id).collect();
        for card in piles.all_cards_mut() {
            card.deck_version = deck_order.iter().position(|id| *id == card.id);
        }
        let next_card_instance_id = all_cards(&piles)
            .map(|card| card.id.get())
            .max()
            .unwrap_or(0)
            + 1;
        let orb_queue = {
            let mut queue = crate::OrbQueue {
                capacity: self.orb_slots,
                orbs: Vec::new(),
                next_orb_instance: 0,
            };
            for model_id in self.orbs {
                let definition = content.orb(&model_id).ok_or_else(|| {
                    EngineError::new(crate::ErrorCode::UnsupportedContent)
                        .with_detail("model_id", model_id.to_string())
                        .with_detail("location", "scenario: orb")
                })?;
                let id = sts2_core::OrbInstanceId::from(queue.next_orb_instance);
                queue.next_orb_instance += 1;
                queue.orbs.push(crate::OrbInstance {
                    id,
                    model_id: model_id.clone(),
                    passive_base: definition.passive_value.base(),
                    evoke_base: definition.evoke_value.base(),
                    passive: 0,
                    evoke: 0,
                    removed_from_state: false,
                });
            }
            queue
        };
        let state = GameState {
            compatibility_id: PINNED_COMPATIBILITY_ID.to_owned(),
            origin: crate::ConformanceOrigin::Scenario,
            rng,
            player_rng: PlayerRngSet::for_player(&self.seed, 0),
            run_player: RunPlayerState {
                slot: "p0".to_owned(),
                max_energy: self.max_energy,
                current_hp: self.player_hp,
                max_hp: self.player_max_hp,
                gold: self.gold,
                deck,
                potions: self.potions,
                relics: self.relics,
                base_orb_slots: self.orb_slots,
                odds: crate::PlayerOdds::start_of_run(),
            },
            run: None,
            combat: Some(CombatState {
                turn_end_is_owed: false,
                powered_blows_this_turn: Vec::new(),
                next_power_instance: 0,
                // A scenario stands a fight up already announced.
                in_progress: true,
                has_ended: false,
                player_hurt_this_turn: false,
                player_took_an_unstopped_blow_this_turn: false,
                player_took_an_unstopped_blow_last_turn: false,
                player_actions_disabled: false,
                played_card_was_attack: false,
                played_card_was_chosen: false,
                played_card_type: None,
                played_card: None,
                played_card_energy_spent: 0,
                played_card_play_index: 0,
                taking_extra_turn: false,
                fatal_kill_this_play: false,
                encounter_model: self.encounter,
                round: self.round,
                side: CombatSide::Player,
                player: PlayerCombatState {
                    slot: "p0".to_owned(),
                    creature_id: player_id,
                    energy: self.energy,
                    max_energy: self.max_energy,
                    stars: self.stars,
                    stars_gained_this_turn: 0,
                    cards_generated_this_combat: 0,
                    turn: self.turn,
                    piles,
                    phase: crate::PlayerTurnPhase::Play,
                    ready_to_end_turn: false,
                    cards_played_this_turn: 0,
                    cards_discarded_this_turn: 0,
                    cards_drawn_this_combat: 0,
                    cards_drawn_this_turn_outside_hand_draw: 0,
                    pet_attacks_this_turn: 0,
                    play_types_started_this_turn: Vec::new(),
                    powers_applied_this_turn: Vec::new(),
                    card_plays_finished: 0,
                    plays_finished: Vec::new(),
                    attack_or_skill_plays_this_turn: 0,
                    attacks_played_this_turn: 0,
                    cards_exhausted_this_turn: 0,
                    block_gains_this_turn: Vec::new(),
                    energy_spent_this_turn: 0,
                    first_plays_started_this_turn: 0,
                    zero_cost_attack_plays_this_turn: 0,
                    status_cards_drawn_this_turn: 0,
                    orbs_channeled_this_combat: Vec::new(),
                    orbs: orb_queue,
                },
                next_creature_id: creatures.len() as u64,
                // The models the encounter counts as having spawned: the
                // creatures the fight was laid out with, one entry per model.
                spawned_enemy_models: {
                    let mut models: Vec<ModelId> = Vec::new();
                    for creature in &creatures {
                        if creature.side == CombatSide::Enemy
                            && !models.contains(&creature.model_id)
                        {
                            models.push(creature.model_id.clone());
                        }
                    }
                    models
                },
                creatures,
                queue: std::collections::VecDeque::new(),
                suspended_choice: None,
                victory_rewards: self.rewards,
                generated_victory_rewards: None,
                reward_set: None,
                rewards_open: false,
                finish_owed: false,
                next_choice_id: 1,
                next_reward_set_id: 1,
                escaped_creatures: Vec::new(),
                escaped_stolen_gold: 0,
            }),
            terminal: None,
            visible_history: crate::VisibleHistory::default(),
            event_sequence: 1,
            next_card_instance_id,
        };
        Simulator::from_scenario(state, content)
    }
}

fn unsupported_monster(model_id: &ModelId) -> EngineError {
    EngineError::new(crate::ErrorCode::UnsupportedContent)
        .with_detail("model_id", model_id.to_string())
}

fn all_cards(piles: &Piles) -> impl Iterator<Item = &CardInstance> {
    piles
        .draw
        .iter()
        .chain(&piles.hand)
        .chain(&piles.discard)
        .chain(&piles.exhaust)
        .chain(&piles.play)
}

/// Stands an enemy up already inside its own move machine.
fn build_enemy(
    enemy: &ScenarioEnemy,
    content: &ContentRegistry,
    slots: &[String],
) -> Result<Creature, EngineError> {
    let definition = content
        .monster(&enemy.model_id)
        .ok_or_else(|| unsupported_monster(&enemy.model_id))?;
    let monster_move = definition
        .monster_move(&enemy.move_state)
        .ok_or_else(|| unsupported_monster(&enemy.model_id))?;
    Ok(Creature {
        slot: enemy.slot_name.as_ref().map_or(-1, |name| {
            slots
                .iter()
                .position(|slot| slot == name)
                .and_then(|index| i32::try_from(index).ok())
                .unwrap_or(-1)
        }),
        slot_name: enemy.slot_name.clone(),
        pet_owner: None,
        last_spawned: None,
        id: enemy.id,
        model_id: enemy.model_id.clone(),
        side: CombatSide::Enemy,
        current_hp: enemy.current_hp,
        max_hp: enemy.max_hp,
        block: 0,
        unblocked_blows_taken: 0,
        powers: Vec::new(),
        intent: Some(monster_move.intent.clone()),
        move_state: enemy.move_state.clone(),
        performed_first_move: false,
        spawned_this_turn: false,
        death_handled: false,
        stunned_follow_up: None,
        stunned_move: Vec::new(),
        off_balance: false,
        hp_display: crate::HpDisplay::Normal,
        performed_log: Vec::new(),
        locals: enemy.locals.iter().cloned().collect(),
        state_log: vec![enemy.move_state.clone()],
    })
}

/// Puts each scenario power on its creature, in the order the builder was told
/// about them.
fn place_powers(
    creatures: &mut [Creature],
    powers: Vec<(CreatureCombatId, ModelId, i32, crate::PowerType)>,
) -> Result<(), EngineError> {
    for (creature_id, model_id, amount, power_type) in powers {
        let creature = creatures
            .iter_mut()
            .find(|creature| creature.id == creature_id)
            .ok_or_else(|| {
                EngineError::new(crate::ErrorCode::InternalInvariant)
                    .with_detail("scenario_power_owner", creature_id.get())
            })?;
        creature.powers.push(crate::PowerInstance {
            instance: 0,
            target: None,
            model_id,
            held_card: None,
            bound_damage_card: None,
            amount,
            counter: 0,
            amount_on_turn_start: amount,
            amounts_at_play_start: Vec::new(),
            play_tallies: Vec::new(),
            skip_next_duration_tick: false,
            applier: None,
            power_type,
            downgraded_cards: Vec::new(),
            latched: false,
        });
    }
    Ok(())
}
