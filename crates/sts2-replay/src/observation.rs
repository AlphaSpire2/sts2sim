//! Rendering engine state into the recorded Projection v1 shape, and comparing
//! the two.
//!
//! A recorded observation carries far more than this simulator models. The
//! renderer emits only what the engine actually owns, and the comparison walks
//! the rendered value: a recorded field the renderer never emits is reported as
//! uncovered rather than silently passing.

use serde_json::{Map, Value, json};
use sts2_engine::{
    CardFingerprint, CardInstance, CombatSide, CombatState, Creature, GameState, PlayerTurnPhase,
    PowerInstance, RelicInstance, RunState, Simulator,
};
use sts2_rng::{PlayerStream, RunStream};

use crate::FieldDifference;

/// Renders the parts of the state this simulator owns, shaped like a recorded
/// Projection v1 observation.
#[must_use]
pub fn render(simulator: &Simulator) -> Value {
    let state = simulator.state();
    let mut rendered = Map::new();
    rendered.insert(
        "players".to_owned(),
        json!([render_player(state, simulator.content())]),
    );
    if let Some(run) = &state.run {
        rendered.insert("run".to_owned(), render_run(state, run));
        rendered.insert("rooms".to_owned(), render_rooms(run));
        rendered.insert("acts".to_owned(), render_acts(run));
    }
    rendered.insert("choices".to_owned(), render_choices(state));
    // What this renderer produces is the run-core projection, every time.
    rendered.insert("scope".to_owned(), json!("run.core"));
    if let Some(combat) = &state.combat {
        rendered.insert("combat".to_owned(), render_combat(combat));
    }
    Value::Object(rendered)
}

fn render_player(state: &GameState, content: &sts2_engine::ContentRegistry) -> Value {
    let player = &state.run_player;
    json!({
        "slot": player.slot,
        "character_model": state
            .run
            .as_ref()
            .and_then(|run| run.config.players.first())
            .map(|config| config.character.to_string()),
        "current_hp": player.current_hp,
        "max_hp": player.max_hp,
        "gold": player.gold,
        "deck": player.deck.iter().map(render_card_fingerprint).collect::<Vec<_>>(),
        "relics": player
            .relics
            .iter()
            .map(|relic| render_relic(relic, content))
            .collect::<Vec<_>>(),
        "max_energy": player.max_energy,
        "max_potion_slots": player.potions.len(),
        // The character's base orb slot count, which is what a fight opens
        // the orb queue with.
        "base_orb_slots": player.base_orb_slots,
        "unlocks": state.run.as_ref().map(|run| json!({
            "epochs": run.unlocks.epochs,
            "encounters_seen": run.unlocks.encounters_seen,
            "number_of_runs": run.unlocks.number_of_runs,
        })),
        "potion_slots": player
            .potions
            .iter()
            .enumerate()
            .map(|(slot, held)| json!({
                "slot": slot,
                "model_id": held.as_ref().map(ToString::to_string),
            }))
            .collect::<Vec<_>>(),
        "odds": {
            "card_rarity": player.odds.card_rarity_offset(),
            "potion_reward": player.odds.potion_reward(),
        },
        // Only the part of `extra` this engine owns. The rest of the recorded
        // object is run statistics and badge state; leaving it unrendered names
        // each missing field precisely instead of hiding them behind `extra`.
        "extra": state
            .run
            .as_ref()
            .map(|run| json!({"card_shop_removals_used": run.card_removals_used})),
        "relic_grab_bag": state
            .run
            .as_ref()
            .map(|run| render_player_grab_bag(&run.player_relic_grab_bag)),
        "rng": {
            "seed": state.player_rng.seed(),
            "counters": {
                "rewards": state.player_rng.stream(PlayerStream::Rewards).counter(),
                "shops": state.player_rng.stream(PlayerStream::Shops).counter(),
                "transformations": state
                    .player_rng
                    .stream(PlayerStream::Transformations)
                    .counter(),
            },
        },
    })
}

/// Renders a card fingerprint exactly as the recorder writes it.
#[must_use]
pub fn render_card_fingerprint(card: &CardFingerprint) -> Value {
    json!({
        "model_id": card.model_id.to_string(),
        "upgrade_level": card.upgrade_level,
        "floor_added_to_deck": card.floor_added_to_deck,
        "enchantment": render_enchantment(card),
        "properties": render_saved_properties(&card.properties),
    })
}

/// A card's recorded enchantment: an object rather than a bare model ID,
/// because the amount is the enchantment's own state and two cards of the same
/// model with the same enchantment are not the same card.
///
/// A card carrying none renders nothing at all.
fn render_enchantment(card: &CardFingerprint) -> Value {
    card.enchantment
        .as_ref()
        .map_or(Value::Null, |enchantment| {
            json!({
                "model_id": enchantment.model_id.to_string(),
                "amount": enchantment.amount,
                "properties": render_saved_properties(&enchantment.properties),
            })
        })
}

/// A card's saved properties: everything a card model saves, bucketed by
/// type. A card that carries none renders nothing at all.
///
/// The build orders each bucket by the declared order a property carries
/// first, then by the property name — and here they come out in name order,
/// which is the same thing wherever a model gives all of its properties one
/// order. A recording is compared name by name rather than in the order it
/// wrote them, so the two agree.
fn render_saved_properties(properties: &std::collections::BTreeMap<String, Value>) -> Value {
    if properties.is_empty() {
        return Value::Null;
    }
    let mut ints = Vec::new();
    let mut bools = Vec::new();
    let mut strings = Vec::new();
    for (name, value) in properties {
        let entry = json!({"name": name, "value": value});
        match value {
            Value::Bool(_) => bools.push(entry),
            Value::String(_) => strings.push(entry),
            _ => ints.push(entry),
        }
    }
    let bucket = |entries: Vec<Value>| {
        if entries.is_empty() {
            Value::Null
        } else {
            Value::Array(entries)
        }
    };
    json!({
        "bools": bucket(bools),
        "card_arrays": Value::Null,
        "cards": Value::Null,
        "int_arrays": Value::Null,
        "ints": bucket(ints),
        "model_ids": Value::Null,
        "strings": bucket(strings),
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "one relic's saved properties, bucket by bucket, as the recorder writes them"
)]
fn render_relic(relic: &RelicInstance, content: &sts2_engine::ContentRegistry) -> Value {
    // A relic's saved properties: everything the relic's model saves,
    // bucketed by type. The buckets are filled independently and the whole
    // object is nothing only when a relic saves nothing at all. A relic that
    // keeps a count carries it, one whose boolean is saved carries that even
    // while it is false, and one that was set up for this player carries what
    // it saved.
    let definition = content.relic(&relic.model_id);
    let mut counters = Vec::new();
    if let Some(name) = definition.and_then(|definition| definition.counter_property.clone())
        && !(relic.counter == 0
            && definition.is_some_and(|definition| definition.counter_saved_only_when_set))
    {
        counters.push(json!({"name": name, "value": relic.counter}));
    }
    // The one relic that keeps two, in the order its model declares them.
    if let Some(name) = definition.and_then(|definition| definition.second_counter_property.clone())
    {
        counters.push(json!({"name": name, "value": relic.second_counter}));
    }
    // The golden compass saves `GoldenPathAct`, the act it was taken in, the
    // moment it is obtained and before anything observes the relic. It is the
    // one map-rule property rendered; the fur coat's four saved properties are
    // not.
    if definition.is_some_and(|definition| {
        definition.map_rule == Some(sts2_engine::RelicMapRule::GoldenPath)
    }) && let Some(act) = relic.map_act
    {
        counters.push(json!({"name": "GoldenPathAct", "value": act}));
    }
    let mut bools = Vec::new();
    let mut int_arrays = Vec::new();
    // A fur coat writes down the act it was taken in, and once it has marked
    // its rooms, their columns and rows as two arrays beside a flag that says
    // they are set.
    if definition.is_some_and(|definition| {
        matches!(
            definition.map_rule,
            Some(sts2_engine::RelicMapRule::MarksCombatRooms { .. })
        )
    }) {
        if let Some(act) = relic.map_act {
            counters.push(json!({"name": "FurCoatActIndex", "value": act}));
        }
        if !relic.marked_coords.is_empty() {
            bools.push(json!({"name": "FurCoatCoordsSet", "value": true}));
            int_arrays.push(json!({
                "name": "FurCoatCoordCols",
                "value": relic.marked_coords.iter().map(|coord| coord.col).collect::<Vec<_>>(),
            }));
            int_arrays.push(json!({
                "name": "FurCoatCoordRows",
                "value": relic.marked_coords.iter().map(|coord| coord.row).collect::<Vec<_>>(),
            }));
        }
    }
    if let Some(name) = definition.and_then(|definition| definition.latch_property.clone()) {
        bools.push(json!({"name": name, "value": relic.latched}));
    }
    // `IsWax` and `IsMelted` are saved by every relic there is, but only
    // where they differ from their default: they are written only once they
    // are true, which is why an ordinary relic carries neither.
    if relic.wax {
        bools.push(json!({"name": "IsWax", "value": true}));
    }
    if relic.melted {
        bools.push(json!({"name": "IsMelted", "value": true}));
    }
    // A relic that keeps a whole list of cards saves it under one name; one
    // that saves cards singly names each of them. The list is a property of
    // the relic itself and stands whether or not it holds anything, so a relic
    // that has handed its last card back still writes an empty list down;
    // cards saved singly are written only where there is one to write.
    let card_array = definition.and_then(|definition| definition.saved_card_array_property.clone());
    let mut card_arrays = Vec::new();
    let mut cards = Vec::new();
    match &card_array {
        Some(name) => card_arrays.push(json!({
            "name": name,
            "value": relic
                .saved_cards
                .iter()
                .map(|(_, card)| render_card_fingerprint(card))
                .collect::<Vec<_>>(),
        })),
        None => {
            cards.extend(
                relic.saved_cards.iter().map(
                    |(name, card)| json!({"name": name, "value": render_card_fingerprint(card)}),
                ),
            );
        }
    }
    let model_ids = relic
        .saved_models
        .iter()
        .map(|(name, model_id)| json!({"name": name, "value": model_id.to_string()}))
        .collect::<Vec<_>>();
    let strings = relic
        .saved_strings
        .iter()
        .map(|(name, value)| json!({"name": name, "value": value}))
        .collect::<Vec<_>>();
    // Saved properties are sorted by declared order and then by name,
    // ordinally, so a relic's own property and one it inherits from a wider
    // kind of relic sit in one alphabet rather than in separate groups. No
    // relic declares an order, which leaves the name.
    let bucket = |mut entries: Vec<Value>| {
        if entries.is_empty() {
            Value::Null
        } else {
            entries.sort_by(|left, right| {
                left["name"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["name"].as_str().unwrap_or_default())
            });
            Value::Array(entries)
        }
    };
    let any = !counters.is_empty()
        || !bools.is_empty()
        || !int_arrays.is_empty()
        || !card_arrays.is_empty()
        || !cards.is_empty()
        || !model_ids.is_empty()
        || !strings.is_empty();
    let properties = any.then(|| {
        json!({
            "bools": bucket(bools),
            "card_arrays": bucket(card_arrays),
            "cards": bucket(cards),
            "int_arrays": bucket(int_arrays),
            "ints": bucket(counters),
            "model_ids": bucket(model_ids),
            "strings": bucket(strings),
        })
    });
    json!({
        "model_id": relic.model_id.to_string(),
        "floor_added_to_deck": relic.floor_added,
        "properties": properties,
    })
}

fn render_run(state: &GameState, run: &RunState) -> Value {
    let odds = run.unknown_room_odds.weights();
    json!({
        "act": run.current_act,
        "current_act_model": run
            .acts
            .get(run.current_act)
            .map(|act| act.act_model.to_string()),
        "floor": run.floor,
        "total_floor": run.floor,
        "seed": run.config.seed,
        "ascension": run.config.ascension,
        "current_map_coord": run
            .current_coord
            .map(|coord| json!({"col": coord.col, "row": coord.row})),
        "odds": {
            "unknown_map_point_monster": odds[0],
            "unknown_map_point_elite": odds[1],
            "unknown_map_point_treasure": odds[2],
            "unknown_map_point_shop": odds[3],
        },
        "reloads": run.reloads,
        "extra": {"started_with_neow": run.started_with_neow},
        "mode": match run.config.mode {
            sts2_engine::RunMode::Standard => "standard",
            sts2_engine::RunMode::Custom => "custom",
        },
        "modifiers": run
            .config
            .modifiers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        "map": render_map(run),
        "visited_map": run
            .visited
            .iter()
            .map(|coord| json!({"col": coord.col, "row": coord.row}))
            .collect::<Vec<_>>(),
        "map_history": run
            .map_history
            .iter()
            .map(|act| json!({
                "act": act.act,
                "points": act
                    .points
                    .iter()
                    .map(|visit| json!({
                        "floor": visit.floor,
                        "type": map_point_type(visit.point_type),
                        "rooms": visit
                            .rooms
                            .iter()
                            .map(|room| json!({
                                "index": room.index,
                                "model_id": room.model_id.as_ref().map(ToString::to_string),
                                "type": room_family(room.room_type),
                                "monsters": room
                                    .monsters
                                    .iter()
                                    .map(|monster| monster.model_id.to_string())
                                    .collect::<Vec<_>>(),
                                "turns_taken": room.turns_taken,
                            }))
                            .collect::<Vec<_>>(),
                    }))
                    .collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "visited_events": run
            .visited_events
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        "shared_relic_grab_bag": render_grab_bag(&run.shared_relic_grab_bag),
        "rng": {
            "seed": run.config.seed,
            "counters": {
                "up_front": counter(state, RunStream::UpFront),
                "shuffle": counter(state, RunStream::Shuffle),
                "unknown_map_point": counter(state, RunStream::UnknownMapPoint),
                "combat_card_generation": counter(state, RunStream::CombatCardGeneration),
                "combat_potion_generation": counter(state, RunStream::CombatPotionGeneration),
                "combat_card_selection": counter(state, RunStream::CombatCardSelection),
                "combat_energy_costs": counter(state, RunStream::CombatEnergyCosts),
                "combat_targets": counter(state, RunStream::CombatTargets),
                "monster_ai": counter(state, RunStream::MonsterAi),
                "niche": counter(state, RunStream::Niche),
                "combat_orbs": counter(state, RunStream::CombatOrbs),
                "treasure_room_relics": counter(state, RunStream::TreasureRoomRelics),
            },
        },
    })
}

fn counter(state: &GameState, stream: RunStream) -> u64 {
    state.rng.stream(stream).counter()
}

fn render_grab_bag(bag: &sts2_engine::RelicGrabBagState) -> Value {
    let ids = |relics: &Vec<sts2_core::ModelId>| {
        relics.iter().map(ToString::to_string).collect::<Vec<_>>()
    };
    json!({
        "common": ids(&bag.common),
        "uncommon": ids(&bag.uncommon),
        "rare": ids(&bag.rare),
        "shop": ids(&bag.shop),
        "event": ids(&bag.event),
        "ancient": ids(&bag.ancient),
    })
}

/// The player's own bag carries only the rarities a player can be offered.
fn render_player_grab_bag(bag: &sts2_engine::RelicGrabBagState) -> Value {
    let ids = |relics: &Vec<sts2_core::ModelId>| {
        relics.iter().map(ToString::to_string).collect::<Vec<_>>()
    };
    json!({
        "common": ids(&bag.common),
        "uncommon": ids(&bag.uncommon),
        "rare": ids(&bag.rare),
        "shop": ids(&bag.shop),
    })
}

/// The act's map, point by point, as the run walks it.
fn render_map(run: &RunState) -> Value {
    render_act_map(run.map())
}

fn render_act_map(map: &sts2_engine::ActMap) -> Value {
    json!({
        "width": map.width,
        "height": map.height,
        // Where the act opens onto: the start point's own children.
        "start_nodes": map
            .points
            .iter()
            .find(|point| point.coord == map.start)
            .map(|point| point
                .children
                .iter()
                .map(|child| json!({"col": child.col, "row": child.row}))
                .collect::<Vec<_>>()),
        // A second boss is written out beside the first, not among the
        // grid's own points.
        "second_boss": map
            .second_boss
            .and_then(|coord| map.points.iter().find(|point| point.coord == coord))
            .map(render_map_point),
        "start": map
            .points
            .iter()
            .find(|point| point.coord == map.start)
            .map(render_map_point),
        "boss": map
            .points
            .iter()
            .find(|point| point.coord == map.boss)
            .map(render_map_point),
        "points": map
            .points
            .iter()
            .filter(|point| {
                point.coord != map.start
                    && point.coord != map.boss
                    && Some(point.coord) != map.second_boss
            })
            .map(render_map_point)
            .collect::<Vec<_>>(),
    })
}

fn render_map_point(point: &sts2_engine::MapPoint) -> Value {
    json!({
        "id": format!("{},{}", point.coord.col, point.coord.row),
        "col": point.coord.col,
        "row": point.coord.row,
        "type": map_point_type(point.point_type),
        "can_modify": point.can_modify,
        "children": point
            .children
            .iter()
            .map(|child| json!({"col": child.col, "row": child.row}))
            .collect::<Vec<_>>(),
    })
}

fn map_point_type(point_type: sts2_engine::MapPointType) -> &'static str {
    match point_type {
        sts2_engine::MapPointType::Monster => "monster",
        sts2_engine::MapPointType::Elite => "elite",
        sts2_engine::MapPointType::Boss => "boss",
        sts2_engine::MapPointType::Shop => "shop",
        sts2_engine::MapPointType::Treasure => "treasure",
        sts2_engine::MapPointType::RestSite => "rest_site",
        sts2_engine::MapPointType::Ancient => "ancient",
        sts2_engine::MapPointType::Unknown => "unknown",
    }
}

/// What the player is being offered right now.
///
/// Only the offer families this engine models are emitted, so an unmodelled one
/// is named as its own uncovered path rather than hidden behind `choices`.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per screen the projection describes"
)]
fn render_choices(state: &GameState) -> Value {
    let mut choices = Map::new();
    let slot = state.run_player.slot.clone();
    // A set the player has proceeded past is still on the build's rewards
    // stack, and the recorder still carries it, until the next point walked
    // onto pops the room out from under it.
    let reward_set = state
        .combat
        .as_ref()
        .map_or_else(
            || state.run.as_ref().and_then(|run| run.reward_set.as_ref()),
            |combat| combat.reward_set.as_ref(),
        )
        .or_else(|| {
            state
                .run
                .as_ref()
                .and_then(|run| run.dismissed_reward_set.as_ref())
        });
    let mut rewards: Vec<_> = state
        .run
        .as_ref()
        .into_iter()
        .flat_map(|run| &run.reward_parents)
        .map(|parent| render_reward_set(state, &parent.set))
        .collect();
    rewards.extend(reward_set.map(|set| render_reward_set(state, set)));
    choices.insert("rewards".to_owned(), json!(rewards));
    let active = state.run.as_ref().and_then(|run| run.active_room.as_ref());
    choices.insert(
        "treasure".to_owned(),
        match active {
            Some(sts2_engine::ActiveRoom::Treasure { relics }) => json!({
                "relics": relics
                    .iter()
                    .enumerate()
                    .map(|(index, relic)| json!({
                        "index": index,
                        "model_id": relic.to_string(),
                    }))
                    .collect::<Vec<_>>(),
            }),
            _ => Value::Null,
        },
    );
    // The run's begun event, not the room: an event the run began is
    // projected until the next one begins, whether or not its room is still
    // standing. A page with no options on it — which is what an event that has
    // finished is left with — is no choice at all.
    let begun = state
        .run
        .as_ref()
        .and_then(|run| run.begun_event.as_ref())
        .filter(|begun| !begun.options.is_empty());
    choices.insert(
        "events".to_owned(),
        match begun {
            Some(begun) => json!([{
                "event_id": begun.model_id.to_string(),
                "player": slot,
                "options": begun
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| json!({
                        "index": index,
                        "option_id": option.option_id,
                        "is_proceed": option.is_proceed,
                    }))
                    .collect::<Vec<_>>(),
            }]),
            None => json!([]),
        },
    );
    let suspended = state.combat.as_ref().map_or_else(
        || {
            state
                .run
                .as_ref()
                .and_then(|run| run.suspended_choice.as_ref())
        },
        |combat| combat.suspended_choice.as_ref(),
    );
    choices.insert(
        "player_choices".to_owned(),
        suspended
            .filter(|choice| described_by_the_recording(choice))
            .map_or_else(
                || json!([]),
                |choice| json!([render_player_choice(state, &slot, choice)]),
            ),
    );
    choices.insert(
        "rest".to_owned(),
        // The list the rest site generated, whole: an option that is on the
        // screen to be read but cannot be taken is still on it, which is why
        // the smith is listed with an unupgradable deck. The entries the build
        // takes off the live list as options are spent are kept here, because
        // the rest observation is written from the option-chosen event, which
        // the build raises before it trims the list.
        match state.run.as_ref().and_then(|run| run.active_room.as_ref()) {
            Some(sts2_engine::ActiveRoom::RestSite { options, .. }) => json!({
                // The list the rest site generated stands on show until the
                // player's next act is observed; after that it is what the
                // trim left.
                "options": options
                    .iter()
                    .enumerate()
                    .filter(|(index, option)| {
                        option.standing || state.run.as_ref().is_some_and(|run| {
                            run.rest_option_owed.is_some()
                                && (run.rest_options_owed.is_empty()
                                    || run.rest_options_owed.contains(index))
                        })
                    })
                    // The recorder indexes the visible list, not the backing
                    // list the synchronizer trims. Once an earlier Miniature
                    // Tent option is gone, the survivor moves down to zero.
                    .enumerate()
                    .map(|(index, (_, state))| json!({
                        "index": index,
                        "option_id": state.option.option_id(),
                    }))
                    .collect::<Vec<_>>(),
            }),
            _ => Value::Null,
        },
    );
    choices.insert(
        "merchant".to_owned(),
        match active {
            Some(sts2_engine::ActiveRoom::Shop {
                offers,
                removal_price,
                removal_used,
                ..
            }) => json!({
                "inventories": [{
                    "player": slot,
                    "entries": render_shop_entries(offers, *removal_price, *removal_used),
                }],
            }),
            _ => Value::Null,
        },
    );
    Value::Object(choices)
}

/// What a deck screen offers, which the recording names by deck position.
///
/// The recorded offer is the deck itself, in plain deck order, not the
/// screen's filtered or sorted list: the merchant's removal screen hides the
/// cards it will not remove, yet the offer carries an eternal card at its own
/// deck index and a curse where the deck holds it rather than sorted to the
/// front. What the player may actually pick stays the screen's business; this
/// is the deck the screen was opened over.
fn deck_cards(state: &GameState) -> Vec<Value> {
    state
        .run_player
        .deck
        .iter()
        .enumerate()
        .map(|(index, fingerprint)| {
            json!({
                "deck_index": index,
                "fingerprint": render_card_fingerprint(fingerprint),
            })
        })
        .collect()
}

/// Whether the recording names a screen in `player_choices` at all.
///
/// The list is not every screen that is up. A card reward, the merchant's
/// removal, an event's grid, a bundle, and a generated-card pick carry an
/// `offered` payload and stand in an observation that names them; the smith,
/// the cook, an event picking out of the deck, the weaver's paid removal, and
/// every screen a card body or a potion opens over a combat pile carry none
/// and stand in an observation whose list is empty. The two groups divide on
/// where the cards came from: a set laid out for the screen, which nothing
/// else in the projection carries, against cards the deck and the piles
/// already name. The merchant's removal is the one deck screen in the first
/// group, because what it lays out is a purchase.
fn described_by_the_recording(choice: &sts2_engine::SuspendedChoice) -> bool {
    choice.identity == sts2_engine::ChoiceIdentity::Offered
        || matches!(
            choice.purpose,
            sts2_engine::ChoicePurpose::RemoveDeckCardAtShop { .. }
        )
}

fn render_player_choice(
    state: &GameState,
    slot: &str,
    choice: &sts2_engine::SuspendedChoice,
) -> Value {
    let card = |candidate: &sts2_engine::CardHandle| json!({"index": candidate.index, "fingerprint": render_card_fingerprint(&candidate.fingerprint)});
    let (context, options) = match &choice.purpose {
        sts2_engine::ChoicePurpose::AddRewardCard { .. } => (
            "card_reward",
            json!({
                "cards": choice.candidates.iter().map(card).collect::<Vec<_>>(),
                // Passing on the reward sits after the cards themselves, and
                // whatever a relic puts on the screen sits after that.
                "alternatives": std::iter::once(json!({
                    "index": choice.candidates.len(),
                    "option_id": "skip",
                }))
                .chain(choice.alternatives.iter().enumerate().map(|(offset, alternative)| {
                    json!({
                        "index": choice.candidates.len() + 1 + offset,
                        "option_id": alternative.option_id,
                    })
                }))
                .collect::<Vec<_>>(),
            }),
        ),
        sts2_engine::ChoicePurpose::RemoveDeckCardAtShop { .. } => (
            "merchant_card_removal",
            json!({
                "cards": deck_cards(state),
            }),
        ),
        sts2_engine::ChoicePurpose::AddChosenCardsToDeck => (
            "reward_grid",
            json!({"options": choice.candidates.iter().map(card).collect::<Vec<_>>()}),
        ),
        // The screens the recording does not name are filtered out before
        // this is reached. One that slipped through would be named without an
        // offered shape, so the comparison reports what is missing from it
        // rather than passing it.
        sts2_engine::ChoicePurpose::RestSmith { .. }
        | sts2_engine::ChoicePurpose::RestCook { .. }
        | sts2_engine::ChoicePurpose::RemoveDeckCardsForGold { .. }
        | sts2_engine::ChoicePurpose::RemoveDeckCardForReward { .. }
        | sts2_engine::ChoicePurpose::RunCommands { .. } => {
            return json!({"choice_id": choice.id.get(), "id": format!("{slot}:{}", choice.id.get())});
        }
    };
    json!({
        "choice_id": choice.id.get(),
        "id": format!("{slot}:{}", choice.id.get()),
        "player": slot,
        "offered": {"context": context, "options": options},
    })
}

fn render_reward_set(state: &GameState, set: &sts2_engine::RewardSet) -> Value {
    json!({
        "id": format!("{}:{}", set.player_slot, set.id.get()),
        "set_id": set.id.get(),
        "player": set.player_slot,
        "room": render_reward_room(state),
        "disallow_skipping": set.disallow_skipping,
        "rewards": set
            .rewards
            .iter()
            .map(|item| json!({
                "index": item.index,
                "selected": item.selected,
                "fingerprint": render_reward_fingerprint(&item.fingerprint),
            }))
            .collect::<Vec<_>>(),
    })
}

/// The room a reward set belongs to. A set an event puts up belongs to no room.
fn render_reward_room(state: &GameState) -> Value {
    let Some(run) = &state.run else {
        return Value::Null;
    };
    // The room the rewards belong to is the one the player is standing in,
    // which is the top of the room stack: a fight an event walked into is what
    // its own rewards name, not the event below it.
    let (id, room_type, model) = match &run.pushed_room {
        Some(pushed) => (1, pushed.room_type, Some(pushed.model_id.to_string())),
        None => (
            0,
            run.current_room_type
                .unwrap_or(sts2_engine::MapPointType::Unknown),
            run.current_room_model.as_ref().map(ToString::to_string),
        ),
    };
    let point_type = match room_type {
        sts2_engine::MapPointType::Monster => "monster",
        sts2_engine::MapPointType::Elite => "elite",
        sts2_engine::MapPointType::Boss => "boss",
        sts2_engine::MapPointType::Treasure => "treasure",
        _ => return Value::Null,
    };
    json!({
        "id": id,
        "type": point_type,
        "model_id": model,
    })
}

/// Renders a reward fingerprint exactly as the recorder writes it.
#[must_use]
pub fn render_reward_fingerprint(fingerprint: &sts2_engine::RewardFingerprint) -> Value {
    // The recorder reads a reward off its save-derived description and writes
    // as nothing whatever that description does not answer. An offer it cannot
    // describe therefore carries no pools, no width, no odds and no source at
    // all, and is described only by the cards standing on the screen.
    let written_down = fingerprint.has_save_representation();
    json!({
        "reward_type": fingerprint.reward_type,
        "card_pools": written_down.then(|| fingerprint
            .card_pools
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()),
        "option_count": written_down.then_some(fingerprint.option_count),
        "rarity_odds": written_down.then(|| fingerprint
            .rarity_odds
            .map_or("none", sts2_engine::RarityOdds::as_str)),
        "source": written_down.then(|| fingerprint.source.as_str()),
        "custom_description_encounter_source_id": fingerprint
            .custom_description_encounter_source_id
            .as_ref()
            .map(ToString::to_string),
        "was_gold_stolen_back": written_down.then_some(fingerprint.was_gold_stolen_back),
        "model_id": fingerprint.model_id.as_ref().map(ToString::to_string),
        "gold_amount": written_down.then_some(fingerprint.gold_amount),
        // The recording writes no offered cards as null rather than an empty
        // list, which is how the replay adapter reads it too.
        "offered_cards": (!fingerprint.offered_cards.is_empty()).then(|| {
            fingerprint
                .offered_cards
                .iter()
                .map(render_card_fingerprint)
                .collect::<Vec<_>>()
        }),
        "special_card": fingerprint.special_card.as_ref().map(render_card_fingerprint),
    })
}

/// A merchant's stock, then the card removal it sells as the entry after it.
fn render_shop_entries(
    offers: &[sts2_engine::ShopOffer],
    removal_price: i32,
    removal_used: bool,
) -> Vec<Value> {
    let mut entries: Vec<Value> = offers
        .iter()
        .map(|offer| {
            let (kind, model_id) = match &offer.item {
                sts2_engine::ShopItem::Card(card) => ("card", card.model_id.to_string()),
                sts2_engine::ShopItem::Potion(model) => ("potion", model.to_string()),
                sts2_engine::ShopItem::Relic(model) => ("relic", model.to_string()),
            };
            json!({
                "index": offer.index,
                "type": kind,
                // A bought slot is cleared but keeps the price it sold at.
                "model_id": (!offer.sold).then_some(model_id),
                "cost": offer.price,
                "stocked": !offer.sold,
            })
        })
        .collect();
    // No `stocked`: the recording's flag for this entry is set from the shop
    // UI node rather than from the run, so it lags a settled removal by an
    // observation. Whether the removal is still available is modelled by
    // `removal_used`, and its price by the run's `card_removals_used`.
    let _ = removal_used;
    entries.push(json!({
        "index": offers.len(),
        "type": "card_removal",
        "model_id": Value::Null,
        "cost": removal_price,
    }));
    entries
}

/// Every act the run generated, each keeping its own progress through its
/// lists. Only the act being played holds a map; earlier acts drop theirs.
fn render_acts(run: &RunState) -> Value {
    let ids = |models: &Vec<sts2_core::ModelId>| {
        models.iter().map(ToString::to_string).collect::<Vec<_>>()
    };
    run.acts
        .iter()
        .enumerate()
        .map(|(index, act)| {
            json!({
                "index": index,
                "model_id": act.act_model.to_string(),
                "saved_map": (index == run.current_act)
                    .then(|| render_act_map(&run.maps[index])),
                "rooms": {
                    "ancient": act.ancient.to_string(),
                    "boss": act.boss.to_string(),
                    "second_boss": act.second_boss.as_ref().map(ToString::to_string),
                    "normal_encounters": ids(&act.normal_encounters),
                    "elite_encounters": ids(&act.elite_encounters),
                    "events": ids(&act.events),
                    "normal_encounters_visited": act.normal_encounters_visited,
                    "elite_encounters_visited": act.elite_encounters_visited,
                    "events_visited": act.events_visited,
                    "boss_encounters_visited": act.boss_encounters_visited,
                },
            })
        })
        .collect::<Vec<_>>()
        .into()
}

/// A room's own family, which names an ancient's room after the event in it.
fn room_family(point_type: sts2_engine::MapPointType) -> &'static str {
    match point_type {
        sts2_engine::MapPointType::Ancient => "event",
        other => map_point_type(other),
    }
}

fn render_rooms(run: &RunState) -> Value {
    if run.current_coord.is_none() {
        // Standing on a fresh act's map, with no point walked onto yet.
        return json!([{"id": 0, "stack_depth": 1, "type": "MAP", "model": Value::Null}]);
    }
    let Some(point_type) = run.current_room_type.or_else(|| {
        run.map()
            .points
            .iter()
            .find(|point| Some(point.coord) == run.current_coord)
            .map(|point| point.point_type)
    }) else {
        return json!([]);
    };
    let mut rooms = vec![json!({
        "id": run.current_room_id,
        "stack_depth": 1,
        "type": room_type(point_type),
        "model": run.current_room_model.as_ref().map(ToString::to_string),
    })];
    // A fight an event walked into without leaving stands on top of it, and the
    // stack is what the location is: the room below is still there.
    if let Some(pushed) = &run.pushed_room {
        rooms.push(json!({
            "id": pushed.id,
            "stack_depth": rooms.len() + 1,
            "type": room_type(pushed.room_type),
            "model": pushed.model_id.to_string(),
        }));
    }
    Value::Array(rooms)
}

fn room_type(point_type: sts2_engine::MapPointType) -> &'static str {
    match point_type {
        sts2_engine::MapPointType::Monster => "MONSTER",
        sts2_engine::MapPointType::Elite => "ELITE",
        sts2_engine::MapPointType::Boss => "BOSS",
        sts2_engine::MapPointType::Shop => "SHOP",
        sts2_engine::MapPointType::Treasure => "TREASURE",
        sts2_engine::MapPointType::RestSite => "RESTSITE",
        sts2_engine::MapPointType::Ancient => "EVENT",
        sts2_engine::MapPointType::Unknown => "UNKNOWN",
    }
}

fn render_combat(combat: &CombatState) -> Value {
    json!({
        "encounter_model": combat.encounter_model.to_string(),
        "round": combat.round,
        "side": match combat.side {
            CombatSide::Player => "player",
            CombatSide::Enemy => "enemy",
        },
        "player_actions_disabled": combat.player_actions_disabled,
        // The engine keeps the creature list in the build's own order, which
        // the enemy side's slots decide; a recording lists them by the
        // identifier each was created with.
        "creatures": render_creatures(combat),
        "players": [{
            "slot": combat.player.slot,
            "phase": match combat.player.phase {
                PlayerTurnPhase::None => "none",
                PlayerTurnPhase::Start => "start",
                PlayerTurnPhase::AutoPrePlay => "auto_pre_play",
                PlayerTurnPhase::Play => "play",
                PlayerTurnPhase::AutoPostPlay => "auto_post_play",
                PlayerTurnPhase::End => "end",
            },
            "ready_to_end_turn": combat.player.ready_to_end_turn,
            "energy": combat.player.energy,
            "max_energy": combat.player.max_energy,
            "stars": combat.player.stars,
            "turn": combat.player.turn,
            // `OrbQueue`: how many slots the player has and what is standing
            // in them, front first. `passive` and `evoke` are the orb's own
            // passive and evoke amounts as they stand, truncated to whole
            // numbers, and `properties` holds its saved properties, which are
            // empty for every orb because none saves any.
            "orbs": {
                "capacity": combat.player.orbs.capacity,
                "items": combat
                    .player
                    .orbs
                    .orbs
                    .iter()
                    .enumerate()
                    .map(|(index, orb)| json!({
                        "index": index,
                        "model_id": orb.model_id.to_string(),
                        "passive": orb.passive,
                        "evoke": orb.evoke,
                        "properties": Value::Null,
                    }))
                    .collect::<Vec<_>>(),
            },
            "piles": {
                "draw": render_pile(&combat.player.piles.draw),
                "hand": render_pile(&combat.player.piles.hand),
                "discard": render_pile(&combat.player.piles.discard),
                "exhaust": render_pile(&combat.player.piles.exhaust),
                "play": render_pile(&combat.player.piles.play),
            },
        }],
    })
}

fn render_pile(cards: &[CardInstance]) -> Value {
    Value::Array(
        cards
            .iter()
            .map(|card| {
                json!({
                    "combat_card_id": card.id.get(),
                    "energy_cost": card.energy_cost,
                    "star_cost": card.star_cost,
                    "costs_x": card.costs_x,
                    "fingerprint": render_card_fingerprint(&card.fingerprint),
                })
            })
            .collect(),
    )
}

fn render_creature(creature: &Creature) -> Value {
    json!({
        "combat_id": creature.id.get(),
        "model_id": creature.model_id.to_string(),
        "side": match creature.side {
            CombatSide::Player => "player",
            CombatSide::Enemy => "enemy",
        },
        "current_hp": creature.current_hp,
        "max_hp": creature.max_hp,
        "block": creature.block,
        "alive": creature.alive(),
        // What is down and waiting to come back cannot be touched.
        "hittable": creature.alive(),
        // A pet stands on the player's side and is not a player, so the two
        // fields are a pair: `player` names the player a creature is and
        // `pet_owner` names the player it belongs to, and no creature ever
        // answers both.
        "player": match creature.side {
            CombatSide::Player if !creature.is_pet() => Some("p0"),
            CombatSide::Player | CombatSide::Enemy => None,
        },
        "pet_owner": creature.is_pet().then_some("p0"),
        // Which of the encounter's slots the creature stands in, and nothing
        // at all for a creature the encounter named no slot for.
        "slot_name": creature.slot_name,
        "powers": creature
            .powers
            .iter()
            .enumerate()
            .map(|(index, power)| render_power(index, power))
            .collect::<Vec<_>>(),
        "move": creature.intent.as_ref().map(|intent| json!({
            "move_id": intent.move_id,
            "intents": intent
                .intents
                .iter()
                .map(|line| json!({
                    "type": line.intent_type,
                    "damage": line.damage,
                    "repeats": line.repeats,
                    "total_damage": line.damage.map(|damage| damage
                        * i32::try_from(line.repeats.unwrap_or(1))
                            .expect("repeat count fits game integer")),
                }))
                .collect::<Vec<_>>(),
        })),
    })
}

/// The creature list a recording carries.
///
/// The engine keeps its own list in the build's order, which the enemy side's
/// slots decide; a recording lists creatures by the identifier each was created
/// with.
fn render_creatures(combat: &sts2_engine::CombatState) -> Value {
    let mut creatures: Vec<&sts2_engine::Creature> = combat.creatures.iter().collect();
    creatures.sort_by_key(|creature| creature.id.get());
    Value::Array(creatures.into_iter().map(render_creature).collect())
}

fn render_power(index: usize, power: &PowerInstance) -> Value {
    json!({
        "index": index,
        "model_id": power.model_id.to_string(),
        "amount": power.amount,
        "amount_on_turn_start": power.amount_on_turn_start,
        "applier": power.applier.map(sts2_core::CreatureCombatId::get),
        // Only an instanced power carries a target.
        "target": power.target.map(sts2_core::CreatureCombatId::get),
        "type": match power.power_type {
            sts2_engine::PowerType::Buff => "buff",
            sts2_engine::PowerType::Debuff => "debuff",
        },
        "properties": Value::Null,
    })
}

/// Compares a rendered observation against the recorded one, walking only what
/// the renderer emitted. Numbers that are not whole are compared as `f32`,
/// because that is the width the recorded projection carries them in.
#[must_use]
pub fn compare(expected: &Value, rendered: &Value) -> Vec<FieldDifference> {
    let mut differences = Vec::new();
    walk("", expected, rendered, &mut differences);
    differences
}

fn walk(pointer: &str, expected: &Value, rendered: &Value, differences: &mut Vec<FieldDifference>) {
    match (expected, rendered) {
        (Value::Object(expected), Value::Object(rendered)) => {
            for (key, rendered) in rendered {
                let child = format!("{pointer}/{key}");
                match expected.get(key) {
                    Some(expected) => walk(&child, expected, rendered, differences),
                    None => differences.push(FieldDifference {
                        pointer: child,
                        expected: None,
                        actual: Some(rendered.clone()),
                    }),
                }
            }
        }
        (Value::Array(expected), Value::Array(rendered)) => {
            if expected.len() != rendered.len() {
                differences.push(FieldDifference {
                    pointer: format!("{pointer}/length"),
                    expected: Some(json!(expected.len())),
                    actual: Some(json!(rendered.len())),
                });
                return;
            }
            for (index, (expected, rendered)) in expected.iter().zip(rendered).enumerate() {
                walk(
                    &format!("{pointer}/{index}"),
                    expected,
                    rendered,
                    differences,
                );
            }
        }
        (Value::Number(expected_number), Value::Number(rendered_number))
            if !expected_number.is_i64() || !rendered_number.is_i64() =>
        {
            // The recording carries these at single precision, so that is the
            // width they have to agree at, bit for bit.
            let same = match (expected_number.as_f64(), rendered_number.as_f64()) {
                (Some(expected), Some(rendered)) => {
                    (expected as f32).to_bits() == (rendered as f32).to_bits()
                }
                _ => expected_number == rendered_number,
            };
            if !same {
                differences.push(FieldDifference {
                    pointer: pointer.to_owned(),
                    expected: Some(expected.clone()),
                    actual: Some(rendered.clone()),
                });
            }
        }
        _ => {
            if expected != rendered {
                differences.push(FieldDifference {
                    pointer: pointer.to_owned(),
                    expected: Some(expected.clone()),
                    actual: Some(rendered.clone()),
                });
            }
        }
    }
}

/// Recorded paths the renderer does not emit, so partial verification stays
/// visible rather than reading as a pass.
#[must_use]
pub fn uncovered(expected: &Value, rendered: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    collect_uncovered("", expected, rendered, &mut paths);
    paths.sort();
    paths
}

fn collect_uncovered(pointer: &str, expected: &Value, rendered: &Value, paths: &mut Vec<String>) {
    match (expected, rendered) {
        (Value::Object(expected), Value::Object(rendered)) => {
            for (key, expected) in expected {
                let child = format!("{pointer}/{key}");
                match rendered.get(key) {
                    Some(rendered) => collect_uncovered(&child, expected, rendered, paths),
                    None => paths.push(child),
                }
            }
        }
        (Value::Array(expected), Value::Array(rendered)) => {
            // Every element, not just the first: an array can hold differently
            // shaped entries, and a field missing from only one of them would
            // otherwise go unreported. The index is normalised out so the set
            // stays stable as arrays grow and shrink.
            for (expected, rendered) in expected.iter().zip(rendered) {
                collect_uncovered(&format!("{pointer}/*"), expected, rendered, paths);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{render_act_map, render_card_fingerprint};
    use serde_json::json;
    use sts2_engine::{ActMap, CardFingerprint, MapCoord, MapPoint, MapPointType};

    /// A recorded act map writes the start, the boss, and a Double Boss
    /// run's second boss out beside the grid's points, not among them. The
    /// engine keeps the second boss in its own point list so that the boss can
    /// lead on to it, and the projection has to take it back out the way it
    /// takes out the other two.
    #[test]
    fn a_second_boss_is_projected_beside_the_points_not_among_them() {
        let at = |col: u8, row: u8| MapCoord { col, row };
        let point = |coord: MapCoord, point_type: MapPointType, children: Vec<MapCoord>| MapPoint {
            coord,
            point_type,
            children,
            can_modify: true,
        };
        let map = ActMap {
            width: 7,
            height: 2,
            start: at(3, 0),
            boss: at(3, 2),
            second_boss: Some(at(3, 3)),
            points: vec![
                point(at(3, 0), MapPointType::Ancient, vec![at(3, 1)]),
                point(at(3, 1), MapPointType::Monster, vec![at(3, 2)]),
                point(at(3, 2), MapPointType::Boss, vec![at(3, 3)]),
                point(at(3, 3), MapPointType::Boss, vec![]),
            ],
        };
        let rendered = render_act_map(&map);
        let ids = |value: &serde_json::Value| -> Vec<String> {
            value
                .as_array()
                .expect("a point list")
                .iter()
                .map(|point| point["id"].as_str().expect("an id").to_owned())
                .collect()
        };
        assert_eq!(ids(&rendered["points"]), vec!["3,1".to_owned()]);
        assert_eq!(rendered["boss"]["id"], json!("3,2"));
        assert_eq!(rendered["second_boss"]["id"], json!("3,3"));
        assert_eq!(rendered["second_boss"]["type"], json!("boss"));
        assert_eq!(rendered["second_boss"]["children"], json!([]));

        let single = ActMap {
            second_boss: None,
            points: map.points[..3].to_vec(),
            ..map
        };
        assert_eq!(
            render_act_map(&single)["second_boss"],
            serde_json::Value::Null
        );
    }

    /// A card with nothing written on it projects no properties at all. One
    /// that carries a saved property projects the bucketed shape a saved run
    /// writes.
    #[test]
    fn a_cards_saved_properties_are_projected_only_when_it_has_some() {
        let mut fingerprint =
            CardFingerprint::base("CARD.GUILTY".parse().expect("static model ID"));
        assert_eq!(
            render_card_fingerprint(&fingerprint)["properties"],
            serde_json::Value::Null
        );
        fingerprint
            .properties
            .insert("CombatsSeen".to_owned(), json!(2));
        assert_eq!(
            render_card_fingerprint(&fingerprint)["properties"],
            json!({
                "bools": serde_json::Value::Null,
                "card_arrays": serde_json::Value::Null,
                "cards": serde_json::Value::Null,
                "int_arrays": serde_json::Value::Null,
                "ints": [{"name": "CombatsSeen", "value": 2}],
                "model_ids": serde_json::Value::Null,
                "strings": serde_json::Value::Null,
            })
        );
    }
}
