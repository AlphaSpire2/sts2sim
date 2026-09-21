use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "sts2sim",
    version,
    about = "Playable command-line interface for sts2sim; with no command, plays the reference combat"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Play the reference combat, or the fight a combat-setup document describes
    Combat {
        /// Combat-setup JSON file (loadout and encounter) to play instead of
        /// the reference combat
        #[arg(long)]
        setup: Option<PathBuf>,
        /// Run seed the setup's fight is generated on; only used with --setup
        #[arg(long, default_value = "SETUP")]
        seed: String,
        /// Play to the end with a fixed policy, taking no input
        #[arg(long)]
        auto: bool,
        /// Print each state as JSON instead of text
        #[arg(long)]
        json: bool,
    },
    /// Generate and play a seeded run from Neow onward
    Run {
        /// Run seed
        #[arg(long)]
        seed: String,
        /// Ascension level, 0 through 10
        #[arg(long, default_value_t = sts2_engine::BASELINE_ASCENSION, value_parser = clap::value_parser!(u8).range(0..=10))]
        ascension: u8,
        /// Play to the end with a fixed policy, taking no input
        #[arg(long)]
        auto: bool,
        /// Print each state as JSON instead of text
        #[arg(long)]
        json: bool,
    },
    /// Replay a recording against the simulator; exits non-zero on any mismatch
    Replay {
        /// Recording (.sts2pgn trace) to replay
        recording: PathBuf,
        /// Print the report as JSON instead of text
        #[arg(long)]
        json: bool,
        /// List the recorded fields the observation renderer does not emit
        #[arg(long)]
        uncovered: bool,
    },
    /// Print the registered model IDs, one per line
    Registry,
    /// Print a shell completion script
    Completions {
        /// Shell to generate the script for
        shell: clap_complete::Shell,
    },
}

fn main() -> ExitCode {
    match execute() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("sts2sim: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute() -> Result<ExitCode, Box<dyn std::error::Error>> {
    // A bare `sts2sim` still means the reference combat.
    let command = Cli::parse().command.unwrap_or(Command::Combat {
        setup: None,
        seed: "SETUP".to_owned(),
        auto: false,
        json: false,
    });
    let (mut simulator, auto, json) = match command {
        Command::Completions { shell } => {
            use clap::CommandFactory as _;
            clap_complete::generate(shell, &mut Cli::command(), "sts2sim", &mut io::stdout());
            return Ok(ExitCode::SUCCESS);
        }
        Command::Registry => {
            // One model ID per line, in the order the registry hands them
            // over: a stable listing for tooling to read.
            for model_id in sts2_content::registered_model_ids() {
                println!("{model_id}");
            }
            return Ok(ExitCode::SUCCESS);
        }
        Command::Replay {
            recording,
            json,
            uncovered,
        } => return replay(&recording, json, uncovered),
        Command::Combat {
            setup: None,
            auto,
            json,
            ..
        } => (sts2_content::shrinker_beetle_fixture()?, auto, json),
        Command::Combat {
            setup: Some(path),
            seed,
            auto,
            json,
        } => {
            let setup: sts2_engine::CombatSetup =
                serde_json::from_reader(BufReader::new(File::open(&path)?))
                    .map_err(|error| format!("{}: {error}", path.display()))?;
            (sts2_content::combat_from_setup(&setup, &seed)?, auto, json)
        }
        Command::Run {
            seed,
            ascension,
            auto,
            json,
        } => (
            sts2_content::standard_ironclad_run_at(&seed, ascension)?,
            auto,
            json,
        ),
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    if auto {
        sts2_cli::run_auto(&mut simulator, &mut output, json)?;
    } else {
        let stdin = io::stdin();
        sts2_cli::run(
            &mut simulator,
            BufReader::new(stdin.lock()),
            &mut output,
            json,
        )?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Differentially replays a recording and exits non-zero on any mismatch.
fn replay(
    path: &Path,
    json: bool,
    uncovered: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let report = sts2_cli::replay(BufReader::new(File::open(path)?))
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    sts2_cli::write_report(&report, &mut output, json, uncovered)?;
    Ok(if report.matched() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
