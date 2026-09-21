//! Text and JSON command-line front end over the public simulator API.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{BufRead, Write};

use sts2_engine::{Action, DecisionContext, RunResult, Simulator};

mod replay;

pub use replay::{ReplayReport, replay, write_report};

#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    Engine(sts2_engine::EngineError),
    InvalidCommand(String),
    /// A recording could not be parsed, validated, or replayed.
    Replay(String),
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Engine(error) => Display::fmt(error, formatter),
            Self::InvalidCommand(command) => write!(formatter, "invalid command: {command}"),
            Self::Replay(detail) => formatter.write_str(detail),
        }
    }
}

impl Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<sts2_engine::EngineError> for CliError {
    fn from(value: sts2_engine::EngineError) -> Self {
        Self::Engine(value)
    }
}

/// Runs a playable session. Commands are `actions`, `state`, an action number,
/// `play CARD_ID`, `end`, `choose CARD_ID`, `claim REWARD_INDEX`, and `proceed`.
pub fn run<R: BufRead, W: Write>(
    simulator: &mut Simulator,
    mut input: R,
    output: &mut W,
    json: bool,
) -> Result<Option<RunResult>, CliError> {
    writeln!(output, "sts2sim — deterministic pinned-build simulator")?;
    loop {
        render(simulator, output, json)?;
        let DecisionContext::Terminal { result } = simulator.decision() else {
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                return Ok(None);
            }
            let command = line.trim();
            if command == "quit" {
                return Ok(None);
            }
            if command == "state" || command == "actions" || command.is_empty() {
                continue;
            }
            let action = parse_action(command, simulator.legal_actions())?;
            simulator.step(action)?;
            continue;
        };
        writeln!(output, "terminal: {result:?}")?;
        return Ok(Some(*result));
    }
}

/// Plays the session to its terminal with a fixed, deterministic policy.
pub fn run_auto<W: Write>(
    simulator: &mut Simulator,
    output: &mut W,
    json: bool,
) -> Result<RunResult, CliError> {
    loop {
        render(simulator, output, json)?;
        if let DecisionContext::Terminal { result } = simulator.decision() {
            writeln!(output, "terminal: {result:?}")?;
            return Ok(*result);
        }
        let action = auto_action(simulator)
            .ok_or_else(|| CliError::InvalidCommand("no legal action".to_owned()))?;
        writeln!(output, "auto> {}", action_label(&action))?;
        simulator.step(action)?;
    }
}

fn render<W: Write>(simulator: &Simulator, output: &mut W, json: bool) -> Result<(), CliError> {
    if json {
        serde_json::to_writer(&mut *output, &simulator.agent_observation())?;
        writeln!(output)?;
    } else {
        let observation = simulator.agent_observation();
        writeln!(
            output,
            "decision={:?} act={:?} floor={:?} map={:?} hp={:?}/{:?} gold={:?} block={:?} energy={:?}",
            observation.decision,
            observation.current_act,
            observation.floor,
            observation.map_coord,
            observation.current_hp,
            observation.max_hp,
            observation.gold,
            observation.block,
            observation.energy
        )?;
        for (index, action) in simulator.legal_actions().iter().enumerate() {
            writeln!(output, "  {index}: {}", action_label(action))?;
        }
    }
    Ok(())
}

fn parse_action(command: &str, legal: &[Action]) -> Result<Action, CliError> {
    if let Ok(index) = command.parse::<usize>() {
        return legal
            .get(index)
            .cloned()
            .ok_or_else(|| CliError::InvalidCommand(command.to_owned()));
    }
    let pieces: Vec<_> = command.split_whitespace().collect();
    let selected = match pieces.as_slice() {
        ["map", col, row] => col.parse::<u8>().ok().zip(row.parse::<u8>().ok()).and_then(
            |(col, row)| {
                legal.iter().find(|action| {
                    matches!(action, Action::ChooseMap { destination }
                        if destination.col == col && destination.row == row)
                })
            },
        ),
        ["event", option_id] => legal.iter().find(|action| {
            matches!(action, Action::ChooseEvent { option_id: candidate, .. }
                if candidate == option_id)
        }),
        ["alternative", option_id] => legal.iter().find(|action| {
            matches!(action, Action::ChooseAlternative { option_id: candidate, .. }
                if candidate == option_id)
        }),
        ["event", index, option_id] => index.parse::<usize>().ok().and_then(|index| {
            legal.iter().find(|action| {
                matches!(action, Action::ChooseEvent { index: at, option_id: candidate }
                    if *at == index && candidate == option_id)
            })
        }),
        ["buy", index] => index.parse::<usize>().ok().and_then(|index| {
            legal.iter().find(|action| {
                matches!(action, Action::BuyShopItem { offer_index, .. }
                    if *offer_index == index)
            })
        }),
        // Removing a card is two steps at a merchant: open the removal, then
        // pick off the card choice it puts up.
        ["remove"] => legal
            .iter()
            .find(|action| matches!(action, Action::BuyCardRemoval)),
        // A rest option is named by its own id. Taking one that opens a screen
        // — the smith, the cook — is two steps as well: take the option, then
        // pick off the deck choice it puts up.
        ["rest", option] => legal.iter().find(|action| {
            matches!(action, Action::RestOption { option: named, .. }
                if named.option_id().eq_ignore_ascii_case(option))
        }),
        ["treasure", model_id] => legal.iter().find(|action| {
            matches!(action, Action::TakeTreasure { relic } if relic.to_string() == *model_id)
        }),
        ["advance"] => legal.iter().find(|action| matches!(action, Action::AdvanceAct)),
        ["end"] => legal
            .iter()
            .find(|action| matches!(action, Action::EndTurn { .. })),
        ["proceed"] => legal
            .iter()
            .find(|action| matches!(action, Action::Proceed)),
        ["play", id] => id.parse::<u64>().ok().and_then(|id| {
            legal.iter().find(|action| {
                matches!(action, Action::PlayCard { card, .. } if card.card_id.get() == id)
            })
        }),
        ["choose", id] => id.parse::<u64>().ok().and_then(|id| {
            legal.iter().find(|action| {
                matches!(action, Action::ChooseCards { cards, .. }
                    if cards.len() == 1 && cards[0].card_id.get() == id)
            })
        }),
        ["claim", index] => index.parse::<usize>().ok().and_then(|index| {
            legal.iter().find(|action| {
                matches!(action, Action::ClaimReward { reward_index, .. }
                    if *reward_index == index)
            })
        }),
        _ => None,
    };
    selected
        .cloned()
        .ok_or_else(|| CliError::InvalidCommand(command.to_owned()))
}

fn auto_action(simulator: &Simulator) -> Option<Action> {
    match simulator.decision() {
        // Whatever is offered first, which for a boss's room is the act change
        // that closes it rather than a proceed.
        DecisionContext::MapNavigation { .. }
        | DecisionContext::Event { .. }
        | DecisionContext::Shop { .. }
        | DecisionContext::RestSite { .. }
        | DecisionContext::Treasure { .. }
        | DecisionContext::CrystalSphere { .. }
        | DecisionContext::RoomProceed
        | DecisionContext::ActTransition { .. } => simulator.legal_actions().first().cloned(),
        // Auto play always takes a card rather than skipping, so a skippable
        // reward cannot loop back to itself.
        DecisionContext::ChooseCards { .. } => simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
            .or_else(|| simulator.legal_actions().first())
            .cloned(),
        DecisionContext::CombatPriority { .. } => {
            for wanted in ["CARD.BULLY", "CARD.BASH", "CARD.STRIKE_IRONCLAD"] {
                if let Some(action) = simulator.legal_actions().iter().find(|action| {
                    matches!(action, Action::PlayCard { card, .. }
                        if card.fingerprint.model_id.to_string() == wanted)
                }) {
                    return Some(action.clone());
                }
            }
            simulator
                .legal_actions()
                .iter()
                .find(|action| matches!(action, Action::EndTurn { .. }))
                .cloned()
        }
        // A boss's reward screen has no proceed of its own: walking on from it
        // is the act change, which is what the build's own proceed button does
        // there.
        DecisionContext::Rewards { .. } => simulator
            .legal_actions()
            .iter()
            .find(|action| matches!(action, Action::ClaimReward { .. }))
            .or_else(|| {
                simulator
                    .legal_actions()
                    .iter()
                    .find(|action| matches!(action, Action::Proceed | Action::AdvanceAct))
            })
            .cloned(),
        DecisionContext::Terminal { .. } => None,
    }
}

fn action_label(action: &Action) -> String {
    match action {
        Action::ChooseMap { destination } => {
            format!("map {} {}", destination.col, destination.row)
        }
        Action::ChooseEvent { index, option_id } => format!("event {index} {option_id}"),
        Action::ChooseAlternative { option_id, .. } => format!("alternative {option_id}"),
        Action::UsePotion {
            slot,
            model_id,
            target,
        } => match target {
            Some(target) => format!("potion {slot} {model_id} -> {}", target.combat_id.get()),
            None => format!("potion {slot} {model_id} -> self"),
        },
        Action::DiscardPotion { slot, model_id } => format!("discard {slot} {model_id}"),
        Action::BuyShopItem {
            offer_index,
            fingerprint,
        } => format!("buy {offer_index} {fingerprint:?}"),
        Action::BuyCardRemoval => "remove".to_owned(),
        Action::RestOption { option, .. } => {
            format!("rest {}", option.option_id().to_lowercase())
        }
        Action::TakeTreasure { relic } => format!("treasure {relic}"),
        Action::UncoverCrystalSphere { x, y, tool } => {
            format!("divine {x} {y} {tool:?}")
        }
        Action::AdvanceAct => "advance".to_owned(),
        Action::PlayCard { card, target } => format!(
            "play {} {} -> {}",
            card.card_id.get(),
            card.fingerprint.model_id,
            target.as_ref().map_or_else(
                || "self".to_owned(),
                |target| target.combat_id.get().to_string()
            )
        ),
        Action::EndTurn { turn } => format!("end (turn {turn})"),
        Action::ChooseCards { cards, .. } => format!(
            "choose {}",
            cards
                .iter()
                .map(|card| card.card_id.get().to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
        Action::ClaimReward {
            reward_index,
            fingerprint,
            ..
        } => {
            format!("claim {reward_index} {}", fingerprint.reward_type)
        }
        Action::Proceed => "proceed".to_owned(),
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Io(std::io::Error::other(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_mode_completes_combat_and_rewards() {
        let mut simulator = sts2_content::shrinker_beetle_fixture().unwrap();
        let mut output = Vec::new();
        let result = run_auto(&mut simulator, &mut output, false).unwrap();
        assert_eq!(result, RunResult::Victory);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("terminal: Victory")
        );
    }

    /// A won run is reported the same way a lost one is: the session ends on
    /// the terminal the simulator settled on, and nothing above the simulator
    /// decides which one it is.
    #[test]
    fn auto_mode_walks_the_last_boss_through_the_architect_to_a_victory() {
        let mut simulator = sts2_content::final_boss_fixture().unwrap();
        let mut output = Vec::new();
        let result = run_auto(&mut simulator, &mut output, false).unwrap();
        assert_eq!(result, RunResult::Victory);
        let session = String::from_utf8(output).unwrap();
        assert!(session.contains("auto> advance"));
        assert!(session.contains("terminal: Victory"));
    }

    #[test]
    fn stable_card_id_commands_drive_the_fixture_line() {
        let mut simulator = sts2_content::shrinker_beetle_fixture().unwrap();
        let commands = b"play 4\nplay 3\nplay 1\nend\nplay 6\nplay 7\nend\nplay 6\nplay 4\nclaim 0\nclaim 1\nclaim 2\nchoose 11\nproceed\n";
        let mut output = Vec::new();
        let result = run(&mut simulator, &commands[..], &mut output, false).unwrap();
        assert_eq!(result, Some(RunResult::Victory));
    }
}
