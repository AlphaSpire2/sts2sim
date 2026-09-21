//! Work that outlives the creature it names.
//!
//! The fight lets a dead creature go, and it lets it go in the middle of
//! settling — so anything still holding that creature's name has to survive
//! finding it gone. Each shape below holds a name past the point the fight
//! let it go.
//!
//! A blow over several targets runs as two loops, and the split is
//! load-bearing: every target takes its damage first, and only then is each
//! settled result handed to the listeners. Between the two, a later target's
//! `BeforeDamageReceived` body can settle the dead — a Thorns paying its
//! owner's attacker back is a blow of its own — sweeping out an earlier target
//! the second loop still owes an announcement to.
//!
//! One queue up, a card play names what the card was aimed at in two effects
//! with the card's own commands queued between them, so a card that kills what
//! it was aimed at leaves the boundary looking at a queued effect naming a
//! creature that is gone.
//!
//! Applying a power is the same shape on the power path: the stat a temporary
//! power lends is worked before the power that lends it is attached, so the
//! loan can settle its own target's death and leave the
//! rest of the application holding a name the fight has let go.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};

const SEED: &str = "SWEEP";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// Two toadpoles, both spiked up, the front one worn down to less than the
/// sweep will land on it. The sweep kills the front one, and the back one's
/// thorns settles that death while the swing is still landing.
fn two_spiked_toadpoles(front_hp: i32) -> Simulator {
    ScenarioBuilder::new(SEED, id("ENCOUNTER.TOADPOLES_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .enemy(1, id("MONSTER.TOADPOLE"), front_hp, 22, "SPIKE_SPIT_MOVE")
        .enemy(2, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
        .power(1, id("POWER.THORNS_POWER"), 2, PowerType::Buff)
        .power(2, id("POWER.THORNS_POWER"), 2, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.BREAKTHROUGH"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds")
}

/// Plays the named card at whichever offered target the caller picks out.
fn play_a_card(
    simulator: &mut Simulator,
    model_id: &str,
    aimed_at: impl Fn(&sts2_engine::TargetHandle) -> bool,
) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, target: Some(target) }
                    if card.fingerprint.model_id == id(model_id) && aimed_at(target)
            )
        })
        .cloned()
        .expect("the card is offered at the target");
    simulator.step(action).expect("the card applies");
}

fn play_breakthrough(simulator: &mut Simulator) {
    let action = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PlayCard { card, .. }
                    if card.fingerprint.model_id == id("CARD.BREAKTHROUGH")
            )
        })
        .cloned()
        .expect("the breakthrough is offered");
    simulator.step(action).expect("the breakthrough applies");
}

/// The second loop writes the blow down on a creature the back toadpole's
/// thorns have already swept out, and the swing settles all the same.
#[test]
fn a_sweep_that_kills_one_of_two_spiked_enemies_still_settles() {
    let mut simulator = two_spiked_toadpoles(7);
    play_breakthrough(&mut simulator);

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    let standing: Vec<_> = combat
        .creatures
        .iter()
        .map(|creature| creature.id)
        .collect();
    assert!(
        !standing.contains(&sts2_core::CreatureCombatId::from(1)),
        "the sweep killed the front toadpole, so the fight let it go"
    );
    let back = combat
        .creatures
        .iter()
        .find(|creature| creature.id == sts2_core::CreatureCombatId::from(2))
        .expect("the back toadpole stands");
    assert_eq!(back.current_hp, 13, "the sweep landed nine on the survivor");
    // One point for the breakthrough's own cost, and two from each toadpole's
    // thorns — the dead one pays its blow back before it falls.
    let player = combat
        .creatures
        .iter()
        .find(|creature| creature.id == combat.player.creature_id)
        .expect("the player stands");
    assert_eq!(player.current_hp, 80 - 1 - 2 - 2);
}

/// The same swing where nothing dies: the announcement writes the blow down on
/// both, which is the history the tolerant write must not have dropped.
#[test]
fn a_sweep_that_kills_neither_writes_the_blow_down_on_both() {
    let mut simulator = two_spiked_toadpoles(22);
    play_breakthrough(&mut simulator);

    let combat = simulator.state().combat.as_ref().expect("the fight stands");
    for slot in [1, 2] {
        let toadpole = combat
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(slot))
            .expect("both toadpoles stand");
        assert_eq!(toadpole.current_hp, 13);
        assert_eq!(
            toadpole.unblocked_blows_taken, 1,
            "the blow is written down on a creature the fight still holds"
        );
    }
}

/// The other half of the same defect, one queue up: a card play names what the
/// card was aimed at in two effects — the play's own announcement before the
/// card's commands and the enchantment's play after them — so a card that
/// kills what it was aimed at and then puts a screen up leaves the boundary
/// staring at a queued effect naming a creature the fight has let go.
///
/// Neow's fury is exactly that card: it swings, and then takes back what the
/// hand has room for. Enchanted, it queues the enchantment behind the screen.
mod a_card_that_kills_what_it_was_aimed_at {
    use super::{card, id};
    use sts2_engine::{
        Action, EnchantmentFingerprint, PileName, ScenarioBuilder, Simulator, TargetHandle,
    };

    /// A beetle worn down to less than the swing, an enchanted Neow's fury in
    /// hand, and a discard pile for the screen to offer back.
    fn fight_against_a_dying_beetle() -> Simulator {
        let mut fury = card("CARD.NEOWS_FURY");
        fury.enchantment = Some(EnchantmentFingerprint::new(id("ENCHANTMENT.SLITHER"), 1));
        ScenarioBuilder::new("FURY", id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            // Two of them: the swing has to kill one and leave the fight
            // standing, or the win clears the queue before the boundary sees it.
            .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 6, 39, "SHRINKER_MOVE")
            .enemy(2, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
            .card(PileName::Hand, 0, fury, 0)
            .card(PileName::Discard, 1, card("CARD.STRIKE_IRONCLAD"), 0)
            .card(PileName::Discard, 2, card("CARD.DEFEND_IRONCLAD"), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    }

    #[test]
    fn still_settles_the_screen_it_puts_up() {
        let mut simulator = fight_against_a_dying_beetle();
        let action = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(
                    action,
                    Action::PlayCard { card, target: Some(TargetHandle { .. }) }
                        if card.fingerprint.model_id == id("CARD.NEOWS_FURY")
                )
            })
            .cloned()
            .expect("the fury is offered at the beetle");
        // The queued enchantment play still names the beetle the swing has
        // just killed, and the boundary stands over it.
        simulator.step(action).expect("the fury applies");

        let combat = simulator.state().combat.as_ref().expect("the fight stands");
        assert!(
            combat
                .creatures
                .iter()
                .all(|creature| creature.id != sts2_core::CreatureCombatId::from(1)),
            "the swing killed the beetle, so the fight let it go"
        );
        assert!(
            combat.suspended_choice.is_some(),
            "the fury's screen went up behind the kill"
        );
    }
}

/// The same shape one frame earlier, in the *first* loop rather than the
/// second: the blow this call is in the middle of settling can sweep out its
/// own target before it lands.
///
/// A blow opens each target's share with `BeforeDamageReceived`, and a Thorns
/// body is a blow of its own. That blow runs a whole cycle on the creature it
/// pays back, hooks and all, and what those set off can come back around: an
/// Inferno on the
/// player answers `AfterDamageReceived` by burning every opponent, which is
/// enough to kill the very creature whose thorns started it — before the swing
/// that provoked the thorns has taken a point off it.
///
/// The build has no such gap, because a corpse stays in the fight: the swing
/// finds no block and no HP left on it and settles to nothing. Here the fight
/// has let it go, so the share has to settle to nothing by finding it gone.
mod a_target_swept_out_by_its_own_retaliation {
    use super::{card, id, play_a_card};
    use sts2_engine::{PileName, PowerType, ScenarioBuilder, Simulator};

    /// A back toadpole worn down to less than the Inferno's answer, spiked so
    /// that hitting it burns the player, and a front toadpole healthy enough
    /// that the fight stands through all of it.
    fn an_inferno_and_two_toadpoles() -> Simulator {
        ScenarioBuilder::new("BURN", id("ENCOUNTER.TOADPOLES_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .enemy(1, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
            .enemy(2, id("MONSTER.TOADPOLE"), 4, 22, "SPIKE_SPIT_MOVE")
            .power(2, id("POWER.THORNS_POWER"), 2, PowerType::Buff)
            // Six a burn, which the back toadpole's four cannot survive.
            .power(0, id("POWER.INFERNO_POWER"), 6, PowerType::Buff)
            .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    }

    /// The share still being settled reads a creature the burn has swept
    /// out, and the strike settles all the same.
    #[test]
    fn a_strike_whose_target_dies_to_the_burn_its_thorns_provoked_still_settles() {
        let mut simulator = an_inferno_and_two_toadpoles();
        play_a_card(&mut simulator, "CARD.STRIKE_IRONCLAD", |candidate| {
            candidate.combat_id == sts2_core::CreatureCombatId::from(2)
        });

        let combat = simulator.state().combat.as_ref().expect("the fight stands");
        assert!(
            combat
                .creatures
                .iter()
                .all(|creature| creature.id != sts2_core::CreatureCombatId::from(2)),
            "the burn killed the back toadpole, so the fight let it go"
        );
        let front = combat
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(1))
            .expect("the front toadpole stands");
        assert_eq!(
            front.current_hp, 16,
            "the burn took six off the toadpole the strike never touched"
        );
        // Two from the back toadpole's thorns, and nothing else: the strike
        // itself lands on a creature the fight no longer holds, so the burn
        // fires once and is not paid back a second time.
        let player = combat
            .creatures
            .iter()
            .find(|creature| creature.id == combat.player.creature_id)
            .expect("the player stands");
        assert_eq!(player.current_hp, 80 - 2);
    }

    /// The same strike where the target lives through the burn: it takes the
    /// blow, which is the settling the tolerant read must not have skipped.
    #[test]
    fn a_strike_whose_target_survives_the_burn_still_lands() {
        let mut simulator = ScenarioBuilder::new("BURN", id("ENCOUNTER.TOADPOLES_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .enemy(1, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
            .enemy(2, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
            .power(2, id("POWER.THORNS_POWER"), 2, PowerType::Buff)
            .power(0, id("POWER.INFERNO_POWER"), 6, PowerType::Buff)
            .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds");
        play_a_card(&mut simulator, "CARD.STRIKE_IRONCLAD", |candidate| {
            candidate.combat_id == sts2_core::CreatureCombatId::from(2)
        });

        let combat = simulator.state().combat.as_ref().expect("the fight stands");
        let back = combat
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(2))
            .expect("the back toadpole stands");
        assert_eq!(
            back.current_hp,
            22 - 6 - 6,
            "the burn and the strike both landed on it"
        );
        assert_eq!(
            back.unblocked_blows_taken, 2,
            "both blows are written down on a creature the fight still holds"
        );
    }
}

mod a_count_read_off_a_creature_the_fight_let_go {
    use super::{card, id, play_a_card};
    use sts2_engine::{PileName, ScenarioBuilder, Simulator};

    /// A hang is a blow and then a mark whose size reads the target — the
    /// larger of two and the mark already carried. The blow settles first,
    /// and a blow that kills sweeps the target out mid-settlement, so the
    /// queued mark's `Count::TargetPower` reads a creature the fight has let
    /// go. Two toadpoles, so the kill leaves the fight standing.
    fn hangs_against_toadpoles(front_hp: i32) -> Simulator {
        ScenarioBuilder::new("HANG", id("ENCOUNTER.TOADPOLES_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .enemy(1, id("MONSTER.TOADPOLE"), front_hp, 22, "SPIKE_SPIT_MOVE")
            .enemy(2, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
            .card(PileName::Hand, 0, card("CARD.HANG"), 0)
            .card(PileName::Hand, 1, card("CARD.HANG"), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    }

    /// The mark's amount reads a creature the loan has killed. The build's
    /// flagged corpse answers the read through whatever it holds and the mark
    /// then lands on nobody; the removed creature answers nothing the same
    /// way.
    #[test]
    fn a_hang_that_kills_its_target_still_settles() {
        let mut simulator = hangs_against_toadpoles(7);
        play_a_card(&mut simulator, "CARD.HANG", |candidate| {
            candidate.combat_id == sts2_core::CreatureCombatId::from(1)
        });

        let combat = simulator.state().combat.as_ref().expect("the fight stands");
        assert!(
            !combat
                .creatures
                .iter()
                .any(|creature| creature.id == sts2_core::CreatureCombatId::from(1)),
            "the blow killed the front toadpole, so the fight let it go"
        );
        let back = combat
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(2))
            .expect("the back toadpole stands");
        assert_eq!(
            back.power_amount("POWER.HANG_POWER"),
            0,
            "the mark aimed at the dead lands on nobody else"
        );
    }

    /// The same play where the target lives: the mark goes two, then doubles
    /// to four — the live reads the tolerant count must not have zeroed.
    #[test]
    fn a_hang_whose_target_survives_is_marked_two_then_four() {
        let mut simulator = hangs_against_toadpoles(50);
        for expected in [2, 4] {
            play_a_card(&mut simulator, "CARD.HANG", |candidate| {
                candidate.combat_id == sts2_core::CreatureCombatId::from(1)
            });
            let combat = simulator.state().combat.as_ref().expect("the fight stands");
            let front = combat
                .creatures
                .iter()
                .find(|creature| creature.id == sts2_core::CreatureCombatId::from(1))
                .expect("the front toadpole stands");
            assert_eq!(
                front.power_amount("POWER.HANG_POWER"),
                expected,
                "the mark is the larger of two and what the target carried"
            );
        }
    }
}

/// One frame earlier again, and on the power path rather than the damage path:
/// the application settles its own target's death from inside itself.
///
/// A temporary power lends a stat, and the application works the loan before
/// the power it came from is attached. That loan is a
/// whole application of its own, hooks and all, and a Sleight of Flesh answers
/// `AfterPowerAmountChanged` by cutting the enemy the lasting debuff landed on.
/// The cut can kill, and the fight then lets the target go while the outer
/// application is still holding its name.
///
/// The build has no gap here either: the application asks again whether the
/// target can still take powers once the loan has settled, and everything
/// after it is inside that guard.
mod a_temporary_power_whose_loan_kills_its_own_target {
    use super::{card, id};
    use sts2_engine::{Action, PileName, PowerType, ScenarioBuilder, Simulator};

    /// A wail against two toadpoles, with a sleight of flesh worth nine on the
    /// player. The wail is a temporary power, so each toadpole is lent minus
    /// six strength first, and the sleight cuts nine for it.
    fn a_wail_against_toadpoles(front_hp: i32, front_illusion: bool) -> Simulator {
        let mut scenario = ScenarioBuilder::new("WAIL", id("ENCOUNTER.TOADPOLES_WEAK"))
            .player_hp(80, 80)
            .energy(9, 9)
            .turn(1, 1)
            .enemy(1, id("MONSTER.TOADPOLE"), front_hp, 22, "SPIKE_SPIT_MOVE")
            .enemy(2, id("MONSTER.TOADPOLE"), 22, 22, "SPIKE_SPIT_MOVE")
            .power(0, id("POWER.SLEIGHT_OF_FLESH_POWER"), 9, PowerType::Buff);
        if front_illusion {
            scenario = scenario.power(1, id("POWER.ILLUSION_POWER"), 1, PowerType::Buff);
        }
        scenario
            .card(PileName::Hand, 0, card("CARD.PIERCING_WAIL"), 0)
            .build(sts2_content::standard_registry())
            .expect("the scenario builds")
    }

    /// Plays the wail, which names every enemy at once rather than one of them.
    fn play_the_wail(simulator: &mut Simulator) {
        let action = simulator
            .legal_actions()
            .iter()
            .find(|action| {
                matches!(
                    action,
                    Action::PlayCard { card, .. }
                        if card.fingerprint.model_id == id("CARD.PIERCING_WAIL")
                )
            })
            .cloned()
            .expect("the wail is offered");
        simulator.step(action).expect("the wail applies");
    }

    fn creature(simulator: &Simulator, slot: u64) -> Option<&sts2_engine::Creature> {
        simulator
            .state()
            .combat
            .as_ref()
            .expect("the fight stands")
            .creatures
            .iter()
            .find(|creature| creature.id == sts2_core::CreatureCombatId::from(slot))
    }

    /// The outer application reads the side of a creature its own loan has
    /// swept out, and the play settles all the same.
    #[test]
    fn a_wail_whose_loan_kills_the_front_toadpole_still_settles() {
        let mut simulator = a_wail_against_toadpoles(5, false);
        play_the_wail(&mut simulator);

        assert!(
            creature(&simulator, 1).is_none(),
            "the cut killed the front toadpole, so the fight let it go"
        );
        let back = creature(&simulator, 2).expect("the back toadpole stands");
        assert_eq!(back.current_hp, 13, "the sleight cut nine off the survivor");
        assert_eq!(back.power_amount("POWER.STRENGTH_POWER"), -6);
        assert_eq!(back.power_amount("POWER.PIERCING_WAIL_POWER"), 6);
    }

    /// The same wail where nothing dies: both toadpoles are lent the loss and
    /// then take the wail, which is the application the guard must not eat.
    #[test]
    fn a_wail_that_kills_neither_toadpole_lands_on_both() {
        let mut simulator = a_wail_against_toadpoles(22, false);
        play_the_wail(&mut simulator);

        for slot in [1, 2] {
            let toadpole = creature(&simulator, slot).expect("both toadpoles stand");
            assert_eq!(toadpole.current_hp, 13, "the sleight cut nine off each");
            assert_eq!(toadpole.power_amount("POWER.STRENGTH_POWER"), -6);
            assert_eq!(toadpole.power_amount("POWER.PIERCING_WAIL_POWER"), 6);
        }
    }

    /// The half of the same defect that never raised: an Illusion keeps its
    /// dead owner in the fight, so the outer application finds the creature
    /// where it left it and works the wail onto a corpse. The build refuses,
    /// because a reviving Illusion refuses to be hit.
    #[test]
    fn a_wail_whose_loan_kills_a_corpse_holder_does_not_land_on_the_corpse() {
        let mut simulator = a_wail_against_toadpoles(5, true);
        play_the_wail(&mut simulator);

        let front = creature(&simulator, 1).expect("the illusion holds the field");
        assert_eq!(front.current_hp, 0, "the cut killed it");
        assert_eq!(
            front.power_amount("POWER.PIERCING_WAIL_POWER"),
            0,
            "and nothing works a power onto it after that"
        );
    }
}
