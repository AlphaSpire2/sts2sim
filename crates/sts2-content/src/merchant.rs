//! The pinned single-player merchant: what it can stock and what it charges.
//!
//! A merchant's table is the walking player's own: it stocks that player's
//! character pool beside the colorless pool, and its potions come from that
//! player's potion pool. So there is one spec per character rather than one
//! for the run.

use sts2_core::ModelId;
use sts2_engine::{MerchantCard, MerchantSpec};

use crate::card_pool::COLORLESS_POOL;
use crate::id;

fn pool(
    entries: &[(&str, sts2_engine::CardType, sts2_engine::CardRarity, i32)],
) -> Vec<MerchantCard> {
    entries
        .iter()
        .map(|(model_id, card_type, rarity, _)| MerchantCard {
            model_id: id(model_id),
            card_type: *card_type,
            rarity: *rarity,
        })
        .collect()
}

#[must_use]
pub fn standard_merchant(character: &ModelId, ascension: u8) -> MerchantSpec {
    let [common_potions, uncommon_potions, rare_potions] =
        crate::potions::generation_pool_by_rarity(character);
    // What the first card removal costs and how much each further removal
    // adds. Inflation raises both.
    let (removal_base_cost, removal_price_increase) = sts2_engine::card_removal_prices(ascension);
    MerchantSpec {
        character_pool: pool(crate::rewards::character_pool(character).unwrap_or(&[])),
        colorless_pool: pool(&COLORLESS_POOL),
        common_potions,
        uncommon_potions,
        rare_potions,
        card_cost: [50, 75, 150],
        potion_cost: [50, 75, 100],
        relic_cost: [175, 225, 275, 200],
        // Whether a relic may be shelved is a flag the relic definitions carry:
        // the merchant reads the flag rather than a list of its own.
        relics_banned_from_shops: crate::relics::NEVER_IN_SHOPS
            .iter()
            .map(|relic| id(relic))
            .collect(),
        removal_base_cost,
        removal_price_increase,
    }
}
