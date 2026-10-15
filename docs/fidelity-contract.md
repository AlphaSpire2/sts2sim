# Fidelity and compatibility contract

This document turns the project goal into a testable contract for the pinned
build. The build identity is pinned as constants in `sts2-core` and restated
for readers outside the build in
[`manifests/compatibility.json`](../manifests/compatibility.json); the two are
checked against each other at load, so a build whose manifest and constants
disagree fails its own tests. If this prose disagrees with either, the code
wins and the disagreement is a defect.

## Supported identity

The only compatibility identity recognised is:

- game version `v0.107.1`;
- game commit `59260271`, the short form the game reports at runtime;
- model database hash `3954186980`.

The recorder reads all three from the running game and writes them into every
trace header, so any recording verifies the pin without other sources. The
compatibility id every artifact carries is derived from them:
`sts2-{version}-{commit}-models-{hash}`.

The supported mode is a new, single-player, standard or custom run with no
gameplay modifiers and no gameplay-affecting mods. A run the player saved,
quit, and loaded again is supported: the recorder continues the same trace
after a `run.resume` record, and replay puts the run back where the build last
saved it, drops the actions played between that save and the quit, and replays
the resumed records on from there. The report counts the reloads and the
rewound actions. Each resume requires the explicit boolean
`data.combat_runtime.ending_player_turn_phase_two` emitted by recorder 0.17.0 and
later. It identifies process state absent from the save. An older recording
without it is still supported when the flag cannot have been up at the quit:
the last recorded fight observation showed it down and no input inside a fight
followed, or no fight was recorded yet. A restarted and a retained process then
both load with it down. Otherwise the missing metadata is reported as
`unsupported_content` at the resume, without claiming a state mismatch or a
complete match. Malformed metadata is invalid input. A true flag is currently
supported only when restoring an active combat; retention through a noncombat
or finished-room checkpoint is unsupported. No later expected observation
substitutes for this input.

A `Mods` header states which mods were loaded, in whichever order the loader
enumerated them. There is no allowlist: any mod may be loaded, and neither ids
nor versions are compared, so a recording made with an empty mod set, with the
recorder stack alone, or with any number of quality-of-life mods beside it is
accepted alike. What refuses a recording is a loaded mod that declares itself
gameplay-affecting — the recording is what replays are diffed against, and
such a mod poisons it. That declaration is the mod's own, carried into the
header from its manifest by the recorder. A mod that touches gameplay without
declaring it is therefore accepted, and its effect surfaces as a divergence
report rather than an `unsupported_mod` refusal.

Other inputs return `unsupported_build`, `unsupported_mode`, or
`unsupported_mod`; they are not normalised to the closest supported input.

## Run identity

`RunConfig` contains all inputs capable of changing generation:

| Field | Required meaning |
| --- | --- |
| `seed` | Canonical STS2 seed string, not a display-only alias |
| `compatibility_id` | Exact version, commit, model hash, and rule set |
| `mode` | `standard` or `custom` |
| `players` | One ordered player slot with a character |
| `ascension` | Exact level, zero through ten |
| `modifiers` | Ordered exact gameplay modifier set; empty in the supported mode |
| `act_one` | The lobby's act-one setting: `random`, `overgrowth`, or `underdocks` |
| `unlock_preset` | The profile name; resolved through the registry when no explicit profile is supplied |
| `unlock_profile` | Optional complete manifest of the unlock state and profile inputs used for generation |

`act_one` is run-defining because act one is rolled rather than fixed: the
setting either names the act outright or leaves the roll standing, and the seed
does not carry which. The build defaults it to `random` and hides the control
that would change it, so a recording of this baseline carries `random`.

The CLI's `run` command takes a seed and an optional ascension and fills every
other field from the baseline: the pinned compatibility id, standard mode, one
player, no modifiers, `random` act one, and the unlock preset `v0107_1_veteran`.
A seed alone makes no parity claim; the baseline is what makes it one.

### Unlock profiles and presets

The registered veteran preset is the default for seed-only runs. Replay builds
an explicit `recorded_profile` manifest from
`run.start.data.state.players[0].unlocks`, preserving epochs, encountered models,
and the exact prior-run count. It does not require a registered preset match.
Generated runs retain the complete manifest in `RunConfig.unlock_profile`, so
serialization preserves the actual generation inputs rather than only a name.
Its schema, compatibility identity, and profile name must agree with the config.
Older configurations without that field still resolve their registered name.

Content generation currently requires all 57 pinned epochs (compared as a set)
and a nonzero prior-run count. Other epoch sets and first-run profiles fail with
an explicit unsupported-content diagnostic: their pool filtering and tutorial
rules are not yet implemented. Encounter histories and discovered-act sets may
differ freely from the veteran preset.

In standard mode, each act's rolled boss is replaced by the first unseen boss
in that act's discovery order. This consumes no additional RNG and happens
before Double Boss chooses its second boss. Custom mode keeps the rolled boss.
Non-boss history entries and the exact nonzero run count remain projection data.

Recorder 0.10.1 supplies `run.start.data.profile.discovered_acts`, captured when
the new run is constructed from the same profile used by act selection. An
explicit empty list means no acts were discovered. When `data.profile` is
absent, replay assumes every supported act was discovered, as a documented
legacy fallback. Present but malformed metadata fails rather than invoking the
fallback. An unlocked, undiscovered alternate act is selected without a roll;
the legacy assumption can therefore produce an act-selection divergence for
older recordings of profiles that had not discovered it.

## Meaning of equal state

At a conformance checkpoint, two states are equal only when every fact that is
currently observable or can change future gameplay is equal. Full canonical
state therefore includes, when applicable:

- run location, acts, maps, map history, room stack, odds, pools, grab bags,
  visited content, resources, inventories, and terminal status;
- every card instance and mutation, ordered deck/combat piles, creatures,
  intent and move history, powers, relic counters, potions, orbs, rewards,
  shops, events, rest state, and active choices;
- exact RNG seed/state and counters for every run, player, derived, or
  content-local gameplay stream;
- ordered queued effects, suspended operations, hook-relevant state, and the
  next stable allocation identifiers whenever they can affect the future; and
- unknown extension data when the external format requires preservation.

Rendering nodes, animation progress, VFX, audio, localised text, UI focus,
wall-clock timestamps, metrics, achievements, and platform services are not
gameplay state. A field is not excluded merely because it is hidden from the
player.

## What a run may depend on

A run depends on its own configuration and on nothing else. It may not read the
wall clock, the operating system, the environment, the localisation table, a
randomly seeded hasher, an entropy source, or any profile state beyond the
unlock preset the configuration names. The build has a generator of its own
that is not seeded by the run; this simulator has no such stream, and every
generator here is derived from the run seed. Where the build reaches for
unseeded randomness, the clock, or the profile inside gameplay, each such read
is either presentation, developer tooling, excluded content, the origin of a
seed, or profile state generated outside any run — and so either outside this
simulator's scope or modelled from an authoritative input.

Two dependencies on the platform are named rather than removed, because they
are the build's own arithmetic: the Box-Muller transform behind the Gaussian
samplers calls `ln`, `sin` and `cos`, which are not correctly rounded and may
differ in the last place between one platform's maths library and another; and
seed canonicalisation upper-cases with Unicode full case mapping where the
build's runtime uses simple invariant mapping. Both are pinned by golden
vectors; every other float in gameplay is one of the five operations IEEE 754
requires to be exact.

Projection v1 canonical JSON is the external replay contract. Objects are
key-sorted, documented arrays retain their gameplay order or specified set
normalisation, missing and `null` differ, numeric values follow the projection
rules, and the digest is SHA-256 over the canonical UTF-8 JSON bytes. The
simulator's internal `state_key` is separate and includes all future-relevant
facts even when Projection v1 does not expose them.

## Privileged and player-facing information

`GameState` and full projections are privileged diagnostics. Policies consume
`AgentObservation`, visible history, and legal actions. The convenient public
policy/search interface must not return or clone `GameState`.

| Fact | Canonical `GameState` | `AgentObservation` | Notes |
| --- | --- | --- | --- |
| Current HP, gold, block, energy | yes | yes | Visible at the boundary |
| Hand and visible card mutations | yes | yes | Preserve visible ordering |
| Draw-pile size | yes | yes | Size is visible |
| Hidden draw-pile order | yes | no | Known top cards may be represented explicitly |
| Current monster intent | yes | yes | Only after the game reveals it |
| Future monster move selection | yes/cached if applicable | no | Must not affect action IDs or ordering |
| Revealed map and visited rooms | yes | yes | Preserve what the player can remember |
| Unrevealed rooms/outcomes | yes | no | Include only a visible unknown marker |
| Current offered reward/shop/event choices | yes | yes | Offer-local stable handles are allowed |
| Future rewards, shops, events, relic order | yes | no | No predictive identifiers |
| Seed and RNG state/counters | yes | no | Diagnostic output only |
| Internal allocation IDs | yes if future-relevant | no by default | Expose only a non-leaking portable handle |
| Pending choice prompt and legal targets | yes | yes | Ordering must follow visible game rules |
| Automatic effect queue internals | yes | no | A normal decision is exposed only at quiescence |

Examples:

1. Two combat states with the same visible hand and draw-pile size but different
   hidden draw order must produce equal observations and equal legal-action
   encodings.
2. If the game reveals the next card, the observation may include that card and
   every sampled hidden state must preserve it at the known position.
3. A map action may identify a visible coordinate such as `(2,3)`. It may not
   carry an internal room object ID that distinguishes two visually identical
   unrevealed outcomes.
4. Full replay diagnostics may report RNG counters after a divergence. The same
   counters must never appear in policy features or alter legal-action ordering.

Search starts from observation plus visible history and samples a possible
hidden world with a separate analysis seed. It must not clone or perturb the
authoritative simulator. Scripted scenario randomness is likewise a separate
test facility and is never evidence of gameplay RNG parity.

## Known divergences

Gameplay victory is terminal after the final required boss's combat settles,
including combat-end effects; Double Boss requires the second boss. The
Architect and all subsequent presentation are outside simulation and
conformance. New recordings finish at the gameplay boundary. Older traces'
post-victory tails are excluded only after the final combat observation is
checked, and the excluded record count is reported.

[`fidelity-divergences.md`](fidelity-divergences.md) is the standing list of
places this simulator departs from the build, each with the evidence for
whether the departure can reach a conformance claim. A divergence not on that
list is a bug.

## Errors and fidelity claims

`sts2-replay` distinguishes trace identity from verification strength. `Profile`
is `recording` or `script`; absent headers use the legacy producer convention.
Conflicting profile/producer declarations and unknown profiles are rejected.
Both profiles undergo the same build and supported-run compatibility checks,
with profile-specific required headers. Scripts need a `Producer`, `Mode CUSTOM`,
and terminal `*`, but not recorder identity, run ID, or a mod list.

Action scripts can be replayed to check action legality and identity fingerprints.
An illegal action establishes a disagreement with the script's expectations,
not which implementation is wrong. Success means actions accepted, not agreement
with game observations. Observation records in scripts cannot confer authority.
Script replay carries an explicit warning and `action_consistency` verification
level in both human and machine reports. Recordings use `recorded_state` and
report actual comparison coverage; a recording with zero comparisons is also
unverified. The shared outcome distinguishes input errors and unsupported content
from gameplay divergence. Report consumers must use this evidence alongside any
success exit code when making conformance claims.

Compatibility is checked before gameplay interpretation. Stable error classes
are:

- `unsupported_build`, `unsupported_mode`, `unsupported_mod`, and
  `unsupported_content` for inputs outside the declared implementation;
- `invalid_config` and `invalid_replay` for malformed or internally
  inconsistent inputs;
- `wrong_decision_context` and `illegal_action` for well-formed actions that
  cannot be applied now;
- `state_divergence` for a conformance mismatch; and
- `internal_invariant` for impossible simulator state.

An error includes a stable code, decision/location context, and structured
details. Illegal player input cannot consume RNG or partially mutate state.
Unknown content never falls back to a generic card, relic, event, monster, or
effect.

`new(RunConfig)` can establish generation conformance. Importing a portable
settled checkpoint can establish continuation conformance only. Neither replay
observations nor later checkpoints may repair ordinary simulator state.

## Recordings

Recordings are kept outside this repository, in a collection of their own, and
the test suite never replays one; conformance for a build is the collection
replayed with the CLI, each recording's exit code being its verdict. A
recording is authoritative the same narrow way every time: for the exact
build, configuration, unlock preset, actions, and observations it carries, and
for nothing beyond them. One Ironclad ascension-0 defeat is not a claim of
character, action, content, act, ascension, or terminal-path coverage.

A recording is either finished or a prefix. A finished run opens at
`run.start` and closes on a `run.finish` whose result equals both the `Result`
header and the terminal line. A prefix stops without finishing: the recorder
writes `Result "*"` when it opens the file and rewrites it only at a clean
finalisation, so a run whose recording was never finalised keeps `*` and has
no terminal line. Both supported format versions allow prefixes. A prefix is
authoritative for exactly the records it carries — every settled observation
in it is as binding as one in a finished run — and it asserts nothing about
how the run ended. Replaying one is a prefix conformance claim: only its
recorded checkpoints constrain replay, and a simulated terminal that follows
from settling the final recorded input is neither asserted nor rejected.

The reader accepts STS2PGN FormatVersion 1 and 2 with Projection version 1.
Plain and gzip input use the same streaming replay path; gzip is detected by
its magic bytes, including concatenated members, and decompression errors are
reported when encountered. The CLI does not create decompressed copies.

FormatVersion 2 is a storage encoding. A `state.delta` references the immediately
previous recorded observation's sequence. Its object-key `set` and `remove`
operations reconstruct a complete `state.observe`, retaining the original
sequence, correlation, context, digest, and unknown fields. Arrays are replaced
whole; null and removal remain distinct. Invalid bases, paths, envelopes, or
full-state digests are errors. A decoding error ends the parser rather than
allowing later records to use an invalid baseline.

The baseline is cleared by `run.start`, `run.resume`, `room.enter`, and `act.enter`.
The following observation must be full, as must observations whose context names
one of those boundaries. This baseline always belongs to the recording; it
never initializes or repairs simulated state. Resume continues to restore the
simulator's own saved snapshot and count rewound actions. Comparison covers the
whole expanded observation, including fields absent from the delta patch.

Canonical numeric tokens are retained without passing through floating point.
Version-two digest verification matches the recorder's UTF-16 ordinal property
ordering and System.Text.Json string escaping. Existing version-one input and
action-only decision scripts remain supported. Only an incomplete JSON payload
on the final unterminated line is ignored as an interrupted append; complete
malformed records and invalid deltas are still errors.

The CLI freezes simulation at the first divergence but continues decoding the
recording to EOF. Its JSON report's `recorded_end` contains floor, current/max HP,
deck size and relic count from the final decoded observation, or null if that
observation lacks usable stats (including an observation-free recording). This
summary describes recorded state and never changes simulation. `records` counts
the entire decoded stream; action, comparison and resume counters stop at the
first divergence. A malformed tail or invalid delta still fails the replay.

A recording's local profile is a run-defining input. Replay constructs an
explicit profile from its recorded unlocks and optional discovery metadata,
subject to the content limits and legacy fallback described above. Registered
presets under [`manifests/unlock-presets/`](../manifests/unlock-presets) remain
defaults for callers that supply only a profile name.

### Selection semantics

Supported selection producers expose complete public effect metadata under
observation version 10. Metadata does not execute commands, consume RNG, infer
private outcomes, or change selection legality. Unknown descriptors are retained
for legacy snapshots. The registry inventory and existing behavioral suites
jointly verify coverage and execution; [choice semantics](choice-semantics.md)
defines the public interface.
