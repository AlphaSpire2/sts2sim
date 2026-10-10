# Public choice semantics

Observation version 10 exposes `ChoiceSemantics` on `ChooseCards`. This is
producer-owned metadata about what selecting a card does. It does not expose
command queues, source continuations, private execution context, or rolled
replacement identities. Consumers must not infer it from source card IDs.

`coverage` is `known`, `partial`, or `unknown`. Unknown has no operations;
known has at least one. Legacy saved choices without this field deserialize
as unknown. `origin` distinguishes combat cards, deck cards, and offered cards;
the engine resolves it when opening the screen. Scope independently identifies
whether a change affects combat or the run deck.

Operations are ordered and retain kind, subject (selected card, copy,
replacement, or player), scope, destination, amount, count, timing, duration,
applicability condition, public model/keyword, and inheritance flags. Counts
are per selected card except storage returns, where they specify cards per
trigger. A zero count is an explicit no-op. `not_applicable` and `unknown`
destinations are different. Named transformations specify their public model;
rolled transformations disclose only that the result is rolled.

Supported kinds are upgrade, exhaust, discard, remove, gain, move, copy,
transform, enchant, play, grant keyword, add/set cost, add replay, store, and
apply power. Conditions include writable energy cost, membership in draw or
discard, and a selected model used as a public option label. Cost duration
and star-cost participation are explicit. The operation describes each card's
effect; command scheduling and ordering among different selected cards remain
execution concerns.

Examples that require more than one operation:

- Transfigure increases writable energy cost and independently increases replay.
- Astrolabe transforms selected cards and upgrades the replacements.
- Liquid Memories makes the selected card temporarily free, then moves it to hand.
- Pael's Tooth stores selected cards, then upgrades and returns cards after combats.
- Pael's Growth describes enchantment amount 4; its amount-1 preview is a
  separate presentation field and is not used to infer the effect.
- Curse of Knowledge's cards label power effects on the player; they are not gains.

Every `CardSelectCmd` constructor carries explicit annotations. Generic helpers
receive them from callers. The registry inventory test walks all registered
cards (including upgrades), powers, relics, potions, monsters, and events for
all characters at ascensions 0 and 10. Missing or invalid supported annotations
fail the test. The current inventory uses at most three ordered operations.
Direct engine producers share the deck gain/removal/upgrade constructors.

Suspended choices and deferred hook screens retain descriptors through snapshots
and resumption. Canonical decision keys include semantics. Victory cancels a
pending choice and its continuation, including its descriptor.

Shop observations expose a removal opportunity with price and eligible deck
indices. Reward observations associate an optional opportunity with each visible
reward, including eligible removal candidates. These previews use public
producer metadata, not probes of unrevealed outcomes. Candidate indices refer
to the run deck for deck origins; offered cards are associated with their reward.
