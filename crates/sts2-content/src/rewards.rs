//! What a character's own pool is worth to a generator.
//!
//! A room's card offers are drawn from the walking player's own card pool and
//! its potion offers from that player's own potion pool, so every generated
//! offer belongs to whoever is playing. The registry keys these by character
//! for exactly that reason.

use sts2_core::ModelId;
use sts2_engine::{CardRarity, GeneratedRewardSpec};

use crate::card_pool::{DEFECT_POOL, IRONCLAD_POOL, NECROBINDER_POOL, REGENT_POOL, SILENT_POOL};

use crate::id;

/// The characters this baseline registers pools for, in the order the build
/// declares them.
pub(crate) const IMPLEMENTED_CHARACTERS: [&str; 5] = [
    "CHARACTER.IRONCLAD",
    "CHARACTER.SILENT",
    "CHARACTER.REGENT",
    "CHARACTER.NECROBINDER",
    "CHARACTER.DEFECT",
];

/// A character's pool, as the tables carry it.
pub(crate) fn character_pool(
    character: &ModelId,
) -> Option<&'static [(&'static str, sts2_engine::CardType, CardRarity, i32)]> {
    match character.to_string().as_str() {
        "CHARACTER.IRONCLAD" => Some(&IRONCLAD_POOL),
        "CHARACTER.SILENT" => Some(&SILENT_POOL),
        "CHARACTER.NECROBINDER" => Some(&NECROBINDER_POOL),
        "CHARACTER.REGENT" => Some(&REGENT_POOL),
        "CHARACTER.DEFECT" => Some(&DEFECT_POOL),
        _ => None,
    }
}

/// The pool a character's own generated cards are named by.
pub(crate) fn character_card_pool_id(character: &ModelId) -> Option<ModelId> {
    match character.to_string().as_str() {
        "CHARACTER.IRONCLAD" => Some(id("CARD_POOL.IRONCLAD_CARD_POOL")),
        "CHARACTER.SILENT" => Some(id("CARD_POOL.SILENT_CARD_POOL")),
        "CHARACTER.NECROBINDER" => Some(id("CARD_POOL.NECROBINDER_CARD_POOL")),
        "CHARACTER.REGENT" => Some(id("CARD_POOL.REGENT_CARD_POOL")),
        "CHARACTER.DEFECT" => Some(id("CARD_POOL.DEFECT_CARD_POOL")),
        _ => None,
    }
}

/// The reward pools are the character card pool split by rarity, keeping the
/// pool's declaration order because reward draws are positional.
fn cards_of(character: &ModelId, rarity: CardRarity) -> Vec<ModelId> {
    character_pool(character)
        .unwrap_or(&[])
        .iter()
        .filter(|(_, _, card_rarity, _)| *card_rarity == rarity)
        .map(|(model_id, _, _, _)| id(model_id))
        .collect()
}

/// The character's common cards, which is what an event that lays out cards
/// draws from.
pub(crate) fn common_cards(character: &ModelId) -> Vec<ModelId> {
    cards_of(character, CardRarity::Common)
}

pub(crate) fn reward_spec(character: &ModelId) -> GeneratedRewardSpec {
    let [common_potions, uncommon_potions, rare_potions] =
        crate::potions::generation_pool_by_rarity(character);
    GeneratedRewardSpec {
        common_cards: cards_of(character, CardRarity::Common),
        uncommon_cards: cards_of(character, CardRarity::Uncommon),
        rare_cards: cards_of(character, CardRarity::Rare),
        common_potions,
        uncommon_potions,
        rare_potions,
        card_option_count: 3,
        card_pools: character_card_pool_id(character).into_iter().collect(),
    }
}

/// A character's card pool as a generator reads it: what each card is and what
/// rarity it counts as.
pub(crate) fn pool_cards(character: &ModelId) -> Vec<sts2_engine::PoolCard> {
    character_pool(character)
        .unwrap_or(&[])
        .iter()
        .map(
            |(model_id, card_type, rarity, energy_cost)| sts2_engine::PoolCard {
                model_id: id(model_id),
                card_type: *card_type,
                rarity: *rarity,
                energy_cost: *energy_cost,
                costs_x: crate::card_pool::costs_x(model_id),
                can_be_generated_in_combat: crate::card_pool::can_be_generated_in_combat(model_id),
                can_be_generated_by_modifiers: true,
            },
        )
        .collect()
}
