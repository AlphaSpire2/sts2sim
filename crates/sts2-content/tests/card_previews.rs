//! The numbers a card draws on its face, which are not the numbers it prints.
//!
//! Drawing a face walks the card's own declared variables, and each of them
//! answers with the whole modifier pass over its base rather than with the
//! base: a damage var is put through every damage modifier there is, and a
//! block var through every block modifier. So the face moves with the powers
//! on both sides of the fight, and moves again while the card is aimed.

use sts2_engine::{Action, CardFingerprint, PileName, PowerType, ScenarioBuilder, Simulator};
use sts2_rng::RunStream;

const SEED: &str = "CARDPREVIEWS";

fn id(value: &str) -> sts2_core::ModelId {
    value.parse().expect("static model ID")
}

fn card(model_id: &str) -> CardFingerprint {
    let mut fingerprint = CardFingerprint::base(id(model_id));
    fingerprint.floor_added_to_deck = Some(1);
    fingerprint
}

/// A fight against `enemies` beetles with the named hand, and whatever powers
/// the test wants standing: creature 0 is the player, 1 and up the beetles.
fn fight(
    hand: &[&str],
    powers: &[(u64, &str, i32, PowerType)],
    enemies: u64,
) -> (Simulator, Vec<sts2_engine::CardPreview>) {
    let mut scenario = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0);
    for index in 0..enemies {
        scenario = scenario.enemy(
            index + 1,
            id("MONSTER.SHRINKER_BEETLE"),
            39,
            39,
            "SHRINKER_MOVE",
        );
    }
    for (index, model_id) in hand.iter().enumerate() {
        let slot = u64::try_from(index).expect("small index");
        scenario = scenario.card(PileName::Hand, slot, card(model_id), 0);
    }
    for (creature, power, amount, power_type) in powers {
        scenario = scenario.power(*creature, id(power), *amount, *power_type);
    }
    let simulator = scenario
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let previews = simulator.agent_observation().hand_previews;
    (simulator, previews)
}

/// What the card in hand slot `index` shows: its unaimed damage, its block, and
/// the damage against each enemy it can be aimed at.
fn face(
    previews: &[sts2_engine::CardPreview],
    index: usize,
) -> (Option<i32>, Option<i32>, Vec<i32>) {
    let preview = &previews[index];
    (
        preview.damage,
        preview.block,
        preview
            .aimed_damage
            .iter()
            .map(|aimed| aimed.damage)
            .collect(),
    )
}

/// A card with nothing acting on it shows what it prints, and the observation
/// carries one face per hand slot.
#[test]
fn an_unmodified_card_shows_what_it_prints() {
    let (simulator, previews) = fight(&["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"], &[], 1);
    let observation = simulator.agent_observation();
    assert_eq!(observation.hand.len(), observation.hand_previews.len());
    assert_eq!(previews[0].card_id, observation.hand[0].card_id);
    assert_eq!(
        face(&previews, 0),
        (Some(6), None, vec![6]),
        "a Strike prints six and draws six, aimed or not"
    );
    assert_eq!(
        face(&previews, 1),
        (None, Some(5), Vec::new()),
        "a Defend draws its block and takes no target"
    );
}

/// Strength adds its stacks to what its owner deals, and the pass that draws
/// the card is that same pass, so the card
/// itself reads nine before it is aimed at anything.
#[test]
fn strength_shows_on_the_card_before_it_is_aimed() {
    let (_, previews) = fight(
        &["CARD.STRIKE_IRONCLAD", "CARD.DEFEND_IRONCLAD"],
        &[(0, "POWER.STRENGTH_POWER", 3, PowerType::Buff)],
        1,
    );
    assert_eq!(
        face(&previews, 0),
        (Some(9), None, vec![9]),
        "six and three strength is nine on the face"
    );
    assert_eq!(
        face(&previews, 1),
        (None, Some(5), Vec::new()),
        "strength is not dexterity: the block is untouched"
    );
}

/// Dexterity is the block half of the same rule, and a card's block var is
/// drawn through the block modifiers the same way.
#[test]
fn dexterity_shows_on_the_block_a_card_draws() {
    let (_, previews) = fight(
        &["CARD.DEFEND_IRONCLAD"],
        &[(0, "POWER.DEXTERITY_POWER", 2, PowerType::Buff)],
        1,
    );
    assert_eq!(face(&previews, 0), (None, Some(7), Vec::new()));
}

/// Weak takes a quarter off what its owner deals. It tests the dealer and not
/// the target, so it shows on the unaimed
/// face too.
#[test]
fn weak_shows_on_the_card_before_it_is_aimed() {
    let (_, previews) = fight(
        &["CARD.STRIKE_IRONCLAD"],
        &[(0, "POWER.WEAK_POWER", 2, PowerType::Debuff)],
        1,
    );
    assert_eq!(
        face(&previews, 0),
        (Some(4), None, vec![4]),
        "three quarters of six is four and a half, and the face draws four"
    );
}

/// Vulnerable tests the creature *taking* the blow, so it counts only where a
/// target is named: the unaimed pass names no target, and every listener that
/// asks about one stays silent. That is the whole per-target difference — the
/// card reads six in the hand and nine while it is pointed at the shaken
/// beetle.
#[test]
fn a_vulnerable_target_shows_only_while_the_card_is_aimed_at_it() {
    let (simulator, previews) = fight(
        &["CARD.STRIKE_IRONCLAD"],
        &[(1, "POWER.VULNERABLE_POWER", 2, PowerType::Debuff)],
        2,
    );
    assert_eq!(
        face(&previews, 0),
        (Some(6), None, vec![9, 6]),
        "half again against the one that is shaken, and the print against the other"
    );
    // The aimed numbers name the creatures they belong to, in the order the
    // combat holds them, and the plays that can be made name the same two.
    let aimed: Vec<_> = previews[0]
        .aimed_damage
        .iter()
        .map(|aimed| aimed.combat_id)
        .collect();
    let targeted: Vec<_> = simulator
        .legal_actions()
        .iter()
        .filter_map(|action| match action {
            Action::PlayCard {
                target: Some(target),
                ..
            } => Some(target.combat_id),
            _ => None,
        })
        .collect();
    assert_eq!(aimed, targeted);
}

/// A card that lands on all of them is held up against every hittable enemy at
/// once, and the shared number is drawn only where every one of them comes to
/// the same one: with all the enemies shaken the face shows the extra damage,
/// and with one shaken and another not it shows none of it.
#[test]
fn an_all_enemies_card_draws_the_shared_number_only_where_the_enemies_agree() {
    let shaken = (1, "POWER.VULNERABLE_POWER", 2, PowerType::Debuff);
    let (_, one) = fight(&["CARD.BREAKTHROUGH"], &[shaken], 2);
    assert_eq!(
        (one[0].damage, one[0].multi_target_damage),
        (Some(9), Some(9)),
        "one beetle shaken and one not: the card falls back to the unaimed pass"
    );

    let (_, both) = fight(
        &["CARD.BREAKTHROUGH"],
        &[shaken, (2, "POWER.VULNERABLE_POWER", 2, PowerType::Debuff)],
        2,
    );
    assert_eq!(
        (both[0].damage, both[0].multi_target_damage),
        (Some(9), Some(13)),
        "both shaken: half again of nine, drawn on the card"
    );
    assert!(
        both[0].aimed_damage.is_empty(),
        "an all-enemies card takes no target selection"
    );
    // The card declares a damage var of nine beside an HP-loss var of one:
    // the nine is the card's damage, and the point it costs its owner is not.
    assert_eq!(both[0].block, None);
}

/// The modifier passes are run over a card only while it stands in the hand or
/// the play pile, so a card waiting in the draw or discard pile draws its
/// print and carries no face here.
#[test]
fn only_the_hand_carries_a_face() {
    let simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "SHRINKER_MOVE")
        .power(0, id("POWER.STRENGTH_POWER"), 3, PowerType::Buff)
        .card(PileName::Hand, 0, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Draw, 1, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Discard, 2, card("CARD.STRIKE_IRONCLAD"), 0)
        .card(PileName::Exhaust, 3, card("CARD.STRIKE_IRONCLAD"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let observation = simulator.agent_observation();
    assert_eq!(observation.hand_previews.len(), 1);
    assert_eq!(observation.hand_previews[0].damage, Some(9));
    assert_eq!(observation.draw_pile.len(), 1);
    assert_eq!(observation.discard.len(), 1);
    assert_eq!(observation.exhaust.len(), 1);
}

/// The face is a read. Every pass it runs answers with a value and does
/// nothing else — the block pass also names which powers moved the number, and
/// the face throws that list away rather than acting on it — so asking for the
/// observation moves neither the state nor a stream.
#[test]
fn asking_for_a_face_moves_nothing() {
    let (mut simulator, _) = fight(
        &[
            "CARD.STRIKE_IRONCLAD",
            "CARD.DEFEND_IRONCLAD",
            "CARD.BREAKTHROUGH",
            "CARD.BASH",
        ],
        &[
            (0, "POWER.STRENGTH_POWER", 3, PowerType::Buff),
            (0, "POWER.WEAK_POWER", 1, PowerType::Debuff),
            (1, "POWER.VULNERABLE_POWER", 2, PowerType::Debuff),
        ],
        2,
    );
    let mut untouched = simulator.clone();

    let state_before = simulator.state_key().expect("a state key");
    let rng_before = simulator.state().rng.to_serializable();
    let first = simulator.agent_observation();
    let first_key = simulator.observation_key().expect("an observation key");
    let second = simulator.agent_observation();
    let second_key = simulator.observation_key().expect("an observation key");

    assert_eq!(
        first, second,
        "the face is the same both times it is asked for"
    );
    assert_eq!(first_key, second_key);
    assert!(
        first
            .hand_previews
            .iter()
            .any(|preview| preview.damage.is_some()),
        "the state under test has faces to draw"
    );
    assert_eq!(simulator.state_key().expect("a state key"), state_before);
    assert_eq!(simulator.state().rng.to_serializable(), rng_before);

    // And the run carries on identically: a simulator that was asked for its
    // observation lands where one that never was lands, streams included.
    let turn = Action::EndTurn { turn: 1 };
    simulator.step(turn.clone()).expect("the turn ends");
    untouched.step(turn).expect("the turn ends");
    assert_eq!(
        simulator.state_key().expect("a state key"),
        untouched.state_key().expect("a state key")
    );
    assert_eq!(
        simulator.state().rng.to_serializable(),
        untouched.state().rng.to_serializable()
    );
}

/// A card that grows its own damage var draws what it stands at, not what it
/// prints. A Rampage swings whatever its damage var stands at and then grows
/// that var by its increase, so the copy that has been played once is a ten
/// that has become a fifteen, and the face is drawn off that same grown
/// base.
#[test]
fn a_card_that_grows_its_own_damage_draws_what_it_has_grown_to() {
    let mut simulator = ScenarioBuilder::new(SEED, id("ENCOUNTER.SHRINKER_BEETLE_WEAK"))
        .player_hp(80, 80)
        .energy(9, 9)
        .turn(1, 1)
        .rng_counter(RunStream::MonsterAi, 0)
        // The beetle stands on its plain attack: a shrink landing on the
        // player would move both faces and hide what is being measured.
        .enemy(1, id("MONSTER.SHRINKER_BEETLE"), 39, 39, "CHOMP_MOVE")
        .card(PileName::Hand, 0, card("CARD.RAMPAGE"), 0)
        .card(PileName::Draw, 1, card("CARD.RAMPAGE"), 0)
        .build(sts2_content::standard_registry())
        .expect("the scenario builds");
    let printed = simulator.agent_observation().hand_previews[0].damage;
    assert_eq!(printed, Some(10), "a rampage prints ten");

    let played = simulator
        .legal_actions()
        .iter()
        .find(|action| {
            matches!(action, Action::PlayCard { card, .. }
                if card.fingerprint.model_id == id("CARD.RAMPAGE"))
        })
        .cloned()
        .expect("the rampage is playable");
    simulator.step(played).expect("the play applies");
    simulator
        .step(Action::EndTurn { turn: 1 })
        .expect("the turn ends");

    // The played copy comes back through the reshuffle, beside the copy that
    // was waiting in the draw pile and has never been played.
    let observation = simulator.agent_observation();
    let mut faces: Vec<i32> = observation
        .hand_previews
        .iter()
        .filter_map(|preview| preview.damage)
        .collect();
    faces.sort_unstable();
    assert_eq!(faces.len(), 2, "both copies are in hand");
    assert_eq!(
        faces[1] - faces[0],
        5,
        "the played copy stands five above the untouched one: the increase is five"
    );
    assert_eq!(
        faces,
        vec![10, 15],
        "ten on the copy that has never been played, fifteen on the one that has"
    );
}

/// A card's number is written into its body once, but a body puts it wherever
/// it likes, and the face draws it all the same. Every number here is the
/// card's own declared variable in the pinned build.
#[test]
fn a_number_a_body_buries_is_still_drawn() {
    // A damage var of four, behind a test on the owner's own power.
    assert_eq!(face(&fight(&["CARD.SHIV"], &[], 1).1, 0).0, Some(4));
    // A damage var of twenty-six, swung by a body that then reads what the
    // swing came to.
    assert_eq!(face(&fight(&["CARD.SUNDER"], &[], 1).1, 0).0, Some(26));
    // A damage var of eight, dealt by a body that splashes the overkill.
    assert_eq!(face(&fight(&["CARD.OMNISLICE"], &[], 1).1, 0).0, Some(8));
    // A damage var of ten, inside the loop that buys itself another pass for
    // every kill.
    assert_eq!(
        face(&fight(&["CARD.ECHOING_SLASH"], &[], 1).1, 0).0,
        Some(10)
    );
    // A pet's own damage var of six, whose pass asks about the pet rather
    // than the player and which a body guards on the pet standing.
    assert_eq!(face(&fight(&["CARD.POKE"], &[], 1).1, 0).0, Some(6));

    // A block var of four, worked up by a body that then reads the gain.
    assert_eq!(
        face(&fight(&["CARD.DODGE_AND_ROLL"], &[], 1).1, 0).1,
        Some(4)
    );
    // A block var of six, inside a counted loop.
    assert_eq!(face(&fight(&["CARD.DEATHS_DOOR"], &[], 1).1, 0).1, Some(6));
    // A block var of five, inside the loop over the cards it burns.
    assert_eq!(face(&fight(&["CARD.SECOND_WIND"], &[], 1).1, 0).1, Some(5));
    // A Glitterstream declares two block vars: an unnamed eleven, and a five
    // named for the block it gives next turn. The face draws the unnamed one.
    assert_eq!(
        face(&fight(&["CARD.GLITTERSTREAM"], &[], 1).1, 0).1,
        Some(11)
    );
}

/// A body's number is the var it declares, and a body that spends something
/// else declares nothing for the face to draw.
#[test]
fn a_number_a_card_never_declared_is_not_drawn() {
    // A damage var of seven and no block var: the block a
    // Fisticuffs works up is worth the blow it has already struck, so its face
    // draws seven and no block at all.
    let (_, fisticuffs) = fight(&["CARD.FISTICUFFS"], &[], 1);
    assert_eq!(face(&fisticuffs, 0).0, Some(7));
    assert_eq!(face(&fisticuffs, 0).1, None);
    // An HP-loss var of thirteen and no damage var: what a Bad Luck takes
    // off its
    // holder at the end of a turn is HP loss, which the face draws under its
    // own name rather than as damage.
    assert_eq!(
        face(&fight(&["CARD.BAD_LUCK"], &[], 1).1, 0),
        (None, None, Vec::new())
    );
}

/// A status is unplayable and swings all the same: its whole body is the one it
/// runs at the end of a turn spent in hand, and its damage var of two is what
/// the face draws. That var is unpowered, which is why a strengthened holder's
/// Burn still reads two.
#[test]
fn a_status_draws_what_it_burns_its_holder_for() {
    let (_, plain) = fight(&["CARD.BURN", "CARD.TOXIC"], &[], 1);
    assert_eq!(face(&plain, 0), (Some(2), None, Vec::new()));
    assert_eq!(face(&plain, 1), (Some(5), None, Vec::new()));

    let (_, strong) = fight(
        &["CARD.BURN", "CARD.STRIKE_IRONCLAD"],
        &[(0, "POWER.STRENGTH_POWER", 3, PowerType::Buff)],
        1,
    );
    assert_eq!(
        face(&strong, 0).0,
        Some(2),
        "nothing scales an unpowered var"
    );
    assert_eq!(
        face(&strong, 1).0,
        Some(9),
        "and the strike beside it is scaled"
    );
}
