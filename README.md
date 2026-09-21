# sts2sim

A deterministic, headless simulator for a pinned build of Slay the Spire 2,
written in Rust. It reimplements the game's rules — it does not drive, wrap, or
load the game. Given the same run configuration and the same decisions, it aims
to produce the same legal choices, the same RNG consumption, and the same
gameplay state as the real build, and to say so out loud when it cannot.

The supported build is `v0.107.1`, commit `59260271`, model hash `3954186980`,
as locked in `manifests/compatibility.json`. Anything else fails rather than
approximating.

See `AGENTS.md` for the project layout, `docs/architecture.md` for the
simulator boundary, and `docs/fidelity-contract.md` for what "the same" means.

## Build and test

```sh
cargo build
cargo test --workspace
```

The suite replays no recordings; see below for how conformance is checked.

## Installing

Nothing here is published to a registry, so the binary comes from a checkout:

```sh
cargo install --path crates/sts2-cli
```

That builds `sts2sim` in release and leaves it in `~/.cargo/bin`, which a
rustup install already has on `PATH`. Every command below then drops its
prefix — `sts2sim replay run.sts2pgn` is `cargo run -p sts2-cli -- replay
run.sts2pgn` without the rebuild check in front of it. The install is a
copy rather than a link to the working tree, so rerun it after pulling;
`cargo uninstall sts2-cli` takes it back off.

### Shell completions

`sts2sim completions SHELL` prints a completion script for `bash`, `elvish`,
`fish`, `powershell`, or `zsh`. For bash, write it where bash-completion looks,
under the command's own name:

```sh
mkdir -p ~/.local/share/bash-completion/completions
sts2sim completions bash > ~/.local/share/bash-completion/completions/sts2sim
```

Shells started after that load it on demand; the one already open needs
`source ~/.local/share/bash-completion/completions/sts2sim` once. Without
bash-completion installed, `eval "$(sts2sim completions bash)"` in `~/.bashrc`
does the same job at the cost of running `sts2sim` on every shell start.

The script is generated from the argument parser, so it completes subcommands
and flags and nothing else: recording paths, seeds, and model IDs are still
typed out in full. It is also a snapshot of the CLI it was generated from —
regenerate it after an upgrade, the same as reinstalling.

## Playing from the command line

The CLI is a thin front end: every choice it offers comes from the engine's
`legal_actions`, and it never invents one of its own.

```sh
cargo run -p sts2-cli -- combat            # the reference combat, from turn one
cargo run -p sts2-cli -- combat --setup fight.json --seed N3Z9EE2CXB   # any fight a combat-setup document describes
cargo run -p sts2-cli -- run --seed N3Z9EE2CXB   # a seeded run from Neow onward
cargo run -p sts2-cli -- run --seed N3Z9EE2CXB --ascension 10   # the same run, ascended
```

Each prompt prints the current decision, a one-line state summary, and the
numbered legal actions:

```text
decision=CombatPriority { player_slot: "p0", turn: 1 } hp=Some(80)/Some(80) block=Some(0) energy=Some(3)
  0: play 0 CARD.DEFEND_IRONCLAD -> self
  1: play 1 CARD.STRIKE_IRONCLAD -> 1
  2: play 4 CARD.BULLY -> 1
  3: potion 0 POTION.BLOCK_POTION -> 0
  4: end (turn 1)
```

Type the number of the action you want, or name it:

| Command | Does |
| --- | --- |
| `4` | takes legal action 4 — always available, and the simplest way to play |
| `play CARD_ID` | plays a card by its instance ID, the number after `play` |
| `end` | ends the turn |
| `choose CARD_ID` | picks a card on an open card choice |
| `claim INDEX` | claims a combat reward |
| `map COL ROW` | walks to a map point |
| `event OPTION_ID` | takes an event option |
| `buy INDEX` / `remove` | buys a merchant slot, or opens the card removal |
| `rest heal` / `rest smith` | picks a rest-site option |
| `treasure MODEL_ID` | takes a treasure relic |
| `proceed` / `advance` | leaves a room, or enters the next act |
| `state` / `actions` | reprints the current decision |
| `quit` | leaves |

Two flags apply to both commands. `--auto` plays the whole thing with a fixed
policy and no input, which is the quickest way to see a run end to end. `--json`
prints machine-readable state instead of text. A third, `--ascension`, applies
to `run` alone: it takes a level zero through ten, and a run that names none is
generated at zero.

## Running a replay differential test

A recording of a real run — an `.sts2pgn` trace of the pinned build — is the
oracle this simulator is checked against. Recordings are not part of this
repository: the trace format is not stable, so they are kept in a collection
of their own. Replaying one applies every action it records and compares every
settled observation against the simulator's own state, stopping at the first
field that differs:

```sh
cargo run -p sts2-cli -- replay run.sts2pgn
```

```text
seed          N3Z9EE2CXB
records       1102 read, 373 actions applied
observations  355 compared, no differences
terminal      recorded DEFEAT, simulated DEFEAT
uncovered     10 recorded fields the renderer does not emit (--uncovered lists them)

MATCH: every compared observation agreed
```

The exit code is zero on a match and non-zero on any divergence, so it drops
straight into a script. On a divergence the report names the differing field,
its recorded and simulated values, and the last observation that agreed.

A trace that was saved and quit mid-run replays as it is: its `Result` still
reads `*`, so it is a prefix that constrains replay only through its recorded
checkpoints and makes no terminal claim. A trace the recorder continued after a
reload carries a `run.resume` record, and replay treats it the way the build
did: the run is put back where the build last saved it — the map point it
walked onto, the fight it won, or the ancient it finished with — the actions
played between that save and the quit are dropped, and the resumed session's
records replay on from there. The report says how many reloads there were and
how many actions they rewound (`resumed` in the text report, `resumes` and
`rewound_actions` in JSON). A second-session file from recorder 0.6.0, whose
`run.start` is the restored save rather than a seeded start, is not a run of
its own and still needs its prefix replayed first.

`--uncovered` lists the recorded fields the observation renderer does not emit
yet — deliberate gaps, not silent ones. `--json` prints the whole report as
JSON.

This command is how conformance is checked. The test suite proves rules in
isolation and never replays a recording, because a recording is authoritative
for one build only and the game is still changing: when the pinned build moves,
so does every recording. Conformance for a build is a curated collection of
recordings of that build, each replayed with this command, with the exit code
as the verdict.

## Licence

Copyright © 2026 Eric Matysek.

sts2sim is free software: you can redistribute it and/or modify it under the
terms of the GNU Affero General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option) any
later version. See [`LICENSE`](LICENSE). It is distributed without any
warranty; the licence has the details.

sts2sim is an independent reimplementation of the rules of Slay the Spire 2.
It is not affiliated with, endorsed by, or supported by Mega Crit. It ships no
game code, assets, or text, and it uses the game's own identifiers only to
describe the behaviour it reproduces. Slay the Spire is a trademark of Mega
Crit.
