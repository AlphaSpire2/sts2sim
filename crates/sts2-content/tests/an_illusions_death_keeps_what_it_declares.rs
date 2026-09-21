//! What an illusion keeps when it goes down.
//!
//! A creature carrying an illusion is not swept out when it dies: it holds its
//! place and comes back. What it holds while it is down is read from the type
//! each power *declares*, not from the type its current amount reads as — and a
//! debuff that only lends a stat and takes it back is kept as well, so that the
//! stat it lent stays with it rather than being handed back to a corpse.
//!
//! So a wail's minus-six strength stays, because strength is a buff however far
//! below zero it has been worked, and the wail's own marker stays with it. A
//! lasting debuff — a vulnerable somebody else put there — is the other side of
//! the guard, and comes off.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};

const SEED: &str = "ILLUSION";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A toadpole standing behind an illusion and a vulnerable, worn down to less
/// than a strike will land on it, with a wail and a strike in hand.
fn an_illusion_about_to_die() -> Simulator {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.TOADPOLES_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.TOADPOLE"), 5, 22, "SPIKE_SPIT_MOVE")
        .enemy(2, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
        .power(1, id("POWER.ILLUSION_POWER"), 1, PowerType::Buff)
        // The illusion marks its owner a minion as it lands. A scenario places
        // the illusion rather than applying it, so the mark is placed too.
        .power(1, id("POWER.MINION_POWER"), 1, PowerType::Buff)
        .power(1, id("POWER.VULNERABLE_POWER"), 2, PowerType::Debuff)
        .card(PileName::Hand, 0, card("CARD.PIERCING_WAIL"), 0)
        .card(PileName::Hand, 1, card("CARD.STRIKE_SILENT"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

fn play(simulator: &mut Simulator, model_id: &str) {
    let wanted = id(model_id);
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, .. } if card.fingerprint.model_id == wanted
            )
        })
        .cloned()
        .unwrap_or_else(|| panic!("{model_id} is offered"));
    simulator.step(action).expect("the card settles");
}

fn creature(simulator: &Simulator, slot: u64) -> &sts2_engine::Creature {
    simulator
        .state()
        .combat
        .as_ref()
        .expect("the fight stands")
        .creatures
        .iter()
        .find(|creature| creature.id == sts2_core::CreatureCombatId::from(slot))
        .expect("the creature is still in the fight")
}

fn powers(simulator: &Simulator, slot: u64) -> Vec<String> {
    let mut held: Vec<String> = creature(simulator, slot)
        .powers
        .iter()
        .map(|power| power.model_id.to_string())
        .collect();
    held.sort();
    held
}

#[test]
fn an_illusion_that_dies_keeps_the_strength_a_wail_took_from_it() {
    let mut simulator = an_illusion_about_to_die();
    play(&mut simulator, "CARD.PIERCING_WAIL");

    assert_eq!(
        creature(&simulator, 1).power_amount("POWER.STRENGTH_POWER"),
        -6,
        "the wail lends the toadpole minus six strength"
    );

    play(&mut simulator, "CARD.STRIKE_SILENT");

    let toadpole = creature(&simulator, 1);
    assert!(
        !toadpole.alive(),
        "the strike puts the illusion down; it holds its place rather than leaving"
    );
    assert_eq!(
        toadpole.power_amount("POWER.STRENGTH_POWER"),
        -6,
        "strength worked below zero is still a buff, so the corpse keeps it"
    );
    assert_eq!(
        powers(&simulator, 1),
        vec![
            "POWER.ILLUSION_POWER".to_owned(),
            "POWER.MINION_POWER".to_owned(),
            "POWER.PIERCING_WAIL_POWER".to_owned(),
            "POWER.STRENGTH_POWER".to_owned(),
        ],
        "the wail's own marker stays with the strength it lent; the vulnerable does not"
    );
}

#[test]
fn an_illusion_that_dies_drops_a_lasting_debuff() {
    let mut simulator = an_illusion_about_to_die();
    assert_eq!(
        creature(&simulator, 1).power_amount("POWER.VULNERABLE_POWER"),
        2,
        "the toadpole starts the fight vulnerable"
    );

    play(&mut simulator, "CARD.STRIKE_SILENT");

    let toadpole = creature(&simulator, 1);
    assert!(!toadpole.alive(), "the strike puts the illusion down");
    assert_eq!(
        toadpole.power_amount("POWER.VULNERABLE_POWER"),
        0,
        "a debuff that lends nothing back comes off the corpse"
    );
    assert_eq!(
        powers(&simulator, 1),
        vec![
            "POWER.ILLUSION_POWER".to_owned(),
            "POWER.MINION_POWER".to_owned(),
        ],
        "only what the illusion answers for is left"
    );
}
