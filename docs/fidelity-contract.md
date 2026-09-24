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
rewound actions.

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
| `unlock_preset` | The registered unlock preset the run is generated against |

`act_one` is run-defining because act one is rolled rather than fixed: the
setting either names the act outright or leaves the roll standing, and the seed
does not carry which. The build defaults it to `random` and hides the control
that would change it, so a recording of this baseline carries `random`.

The CLI's `run` command takes a seed and an optional ascension and fills every
other field from the baseline: the pinned compatibility id, standard mode, one
player, no modifiers, `random` act one, and the unlock preset `v0107_1_veteran`.
A seed alone makes no parity claim; the baseline is what makes it one.

### Unlock presets

An unlock preset is the one registered generation environment of the pinned
build: a profile past its first run, all 57 patch epochs revealed, every
boss-discovery encounter seen. Its `unlocks` block states exactly what
generation reads and no more, each to exactly the depth generation reads it:

- the revealed epochs, exactly, since pools, alternate acts, and ancients
  filter through them;
- the twelve boss-discovery encounters, since the only thing the seen-encounter
  list changes in a run is which boss an act rolls: a boss the profile has
  never seen is taken, in the act's discovery order, ahead of the rolled one;
- the prior-run count, which generation only ever asks one question of —
  whether the profile has finished a run before — so the preset's count is
  simply nonzero.

Every profile meeting that gate generates identically and resolves to this one
preset; profiles that differ past it — by count past a first run, or by
encounters no discovery list names — are one configuration. A replay carries
the recording's own count and encounter list through for the projection.

A preset also carries the profile inputs generation reads that a recording does
not project. That is `profile.discovered_acts`: an unlocked alternate act
the profile has never seen is taken without a roll, so which acts a profile has
already seen decides both the act list and where the act-selection stream
stands. The veteran preset lists both opening acts as discovered. That is
derived rather than recorded: a profile that had not discovered the Underdocks
could only ever open there, and a run played on this environment opens in
whichever act its act-selection roll names. A mismatch here cannot be detected
when a preset is matched; it surfaces as an act-selection divergence at the
first replayed record.

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

[`fidelity-divergences.md`](fidelity-divergences.md) is the standing list of
places this simulator departs from the build, each with the evidence for
whether the departure can reach a conformance claim. A divergence not on that
list is a bug.

## Errors and fidelity claims

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

A recording's local profile is a run-defining input, so each one resolves to a
registered unlock preset under
[`manifests/unlock-presets/`](../manifests/unlock-presets), matched on the
generation inputs the preset section above names.
