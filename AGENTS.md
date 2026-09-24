# AGENTS.md

## Project goal

Build a deterministic, headless, single-player simulator for Slay the Spire 2
(STS2) in Rust. The simulator should reproduce gameplay-relevant behavior for a
pinned game build without launching STS2, Godot, or game assets. It is intended
to support replay/conformance testing first and, later, Python bindings,
reinforcement learning, and Monte-Carlo Tree Search.

Exact fidelity matters: identical run configuration and player decisions should
produce the same legal choices, RNG consumption, gameplay state, and terminal
result as the supported STS2 build. Unsupported builds or modes should fail
explicitly rather than silently approximate behavior.

See `docs/fidelity-contract.md` for the compatibility and fidelity contract
and `docs/architecture.md` for the current simulator boundary.

## Documentation strategy

Files under `docs/` are authoritative design documents describing the current
intended system. Update them in place when the design changes; do not append
implementation diaries, milestone announcements, or historical progress notes
to them. Progress reports, completed-phase summaries, and other historical
status records are not kept in this repository.

## Project layout

The simulator has the deterministic kernel, the engine substrate, seeded run
generation, map traversal, and the whole content inventory of the pinned
build: all five characters are playable, all four acts are generated and
walked, and ascension 1 through 10 is a run input with every rule
implemented.

```text
sts2sim/
├── Cargo.toml                  # Rust workspace and shared policy
├── crates/
│   ├── sts2-core/              # Stable IDs, compatibility and unlock manifests
│   ├── sts2-engine/            # State machine, commands, snapshots, and actions
│   ├── sts2-content/           # The pinned build's content, model by model
│   ├── sts2-rng/               # Exact deterministic RNG and named streams
│   ├── sts2-replay/            # Format v1/v2, gzip, projections, and replay adapter
│   └── sts2-cli/               # Playable text/JSON run and replay interface
├── AGENTS.md                    # This overview for contributors and agents
├── docs/
│   ├── architecture.md          # Current simulator API and state-machine design
│   ├── fidelity-contract.md     # Current compatibility and information contract
│   └── fidelity-divergences.md  # Departures from the build, and what each one costs
└── manifests/                   # Compatibility and unlock-preset locks
```

`.github/workflows/ci.yml` runs what a fresh clone can run: the workspace
tests, the formatter, clippy and the docs, on three operating systems.

## Recordings

Recordings of the real game — `.sts2pgn` traces — are the differential-replay
oracle, and none of them is part of this repository: the trace format is not
stable, so they are kept in a collection of their own. **No recording is ever
added to this repository**, and `.gitignore` refuses the extension.

The test suite never replays a recording. A recording is authoritative only
for the exact build, configuration, unlock state, and recorded behavior it
carries, and the game is still changing: a test pinned to one would have to be
rewritten at every patch. Conformance for a build is checked outside the suite
instead, by running `sts2sim replay` over a curated collection of recordings
of that build. Every rule in the suite is proven by a scenario test that states
the rule in its own terms. A build behaviour this simulator does not reproduce is
recorded in `docs/fidelity-divergences.md` with the recording that would have
caught it named rather than captured.
