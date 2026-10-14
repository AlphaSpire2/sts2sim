# Simulator architecture and public boundary

This document is the normative behavioural boundary for the Rust
implementation: the shape of the public API, what a caller may observe and act
on, and the mechanisms that every piece of content is written against. It is a
description of the simulator as it stands, not of how it got there. Where a
rule of the game is stated here, it is stated because the simulator reproduces
it and something in the boundary depends on it.

The simulator is deterministic and headless. It owns the run's RNG streams, the
stable-ID allocators, and a streaming replay layer, and it exposes a
state-machine API: a caller reads a decision context and its legal actions,
steps one action, and reads the transition that came of it.

## Coverage

A run is generated from a seed alone. Which acts a run is generated against is
itself rolled, off a stream built from the seed before the run exists. Map
generation, the relic bag, the room pools, and the Neow offer are reproduced
exactly for single-player runs. Rooms are generated when the player walks onto
them: an unknown map point resolves into a real room family on entry, and a
monster room is built from the act's encounter list and the encounter's own
floor-local RNG. Card rarity and potion-drop odds drift across a run as player
state, so reward and merchant rolls share one moving distribution. The
generated-room manifest is deliberately narrow: an unsupported relic or room
model fails with its model ID and the place in the run it was reached.

Every character's run is stood up from its own loadout, and every card, relic,
potion, power, monster, encounter, event and pool of the pinned build that a
single-player run can reach is registered under the model the build declares
it as. What a run is offered is keyed by the character playing it — the card
pool a reward draws from, the pool a body that conjures "one of the owner's own
cards" reads, the potions a belt is filled with, and the shelf a merchant
stocks — so a model that reaches into a colour other than the one being played
reaches a real one. The multiplayer-only colorless cards are not registered at
all, because a single-player run's pools never hold one. See [Pools](#pools)
and [Events](#events).

A run can also be picked back up. A recording that opens on a load models
exactly the four things the save round trip is observable in: the run's reload
count, every map point becoming modifiable again, a combat in progress coming
back on its first round, and the reward-set and player-choice synchronizers
being rebuilt, so the ids they hand out start over — a rewards screen the save
left owed is rebuilt along with them and takes the first id the fresh
synchronizer gives, and a selection screen the room's entrance put up, standing
or banked by a listener, is asked for again and takes the first choice ids in
the order it was first asked for. Nothing else about a save is modelled,
because nothing else about it is observable through this boundary.

The command surface covers the gameplay operations the registered content
reaches, and the modifier hooks and room resolution cover the hook events and
room families that content reaches. Every point the game stops for a selection
screen stops here too, and picks itself back up where it stood, including the
turn-boundary sites that let several screens stand at once; the one site that
does not is a listed divergence (see
[`fidelity-divergences.md`](fidelity-divergences.md)).

## Deterministic state-machine API

The engine boundary is conceptually:

```text
Simulator::new(RunConfig) -> Result<Simulator, ConfigError>
Simulator::from_portable_state(PortableState) -> Result<Simulator, ImportError>
Simulator::state() -> &GameState                       # privileged
Simulator::agent_observation() -> AgentObservation     # policy-safe
Simulator::decision() -> DecisionContext
Simulator::legal_actions() -> &[Action]
Simulator::step(Action) -> Result<Transition, StepError>
Simulator::step_quietly(&Action) -> Result<(), StepError>  # same walk, no Transition
Simulator::snapshot()/restore() or Clone
Simulator::state_key() -> StateKey
Simulator::observation_key() -> ObservationKey         # policy-safe; folds copy identity and hand order
Simulator::project(version, scope) -> PortableState
Simulator::projection_digest(version, scope) -> Digest
Simulator::combat_setup() -> Option<CombatSetup>          # the fight the run stands in, as a loadout and a door
Simulator::enter_combat_setup(&CombatSetup) -> Result<Simulator, EngineError>
sts2_content::combat_from_setup(&CombatSetup, seed) -> Result<Simulator, EngineError>
```

No operation consults global mutable run state. Immutable pinned content may be
shared, but a simulator owns every mutable run RNG, per-player RNG, ordered
relic grab bag, queue, hook, and allocation counter.

`GameState` is authoritative and privileged. `AgentObservation` is a derived,
information-safe value, not a serializable subset that consumers can cast back
to the engine. `PortableState` is a versioned replay/conformance boundary and is
not an STS2 save format. `Action` is structured data with stable discriminants
and portable target fingerprints; CLI strings are only an adapter.

`CombatSetup` is a fight said in the game's own terms — character, ascension,
act, floor, encounter and room kind, hit points, gold, the deck with upgrades
and enchantments, the relics with the counts they keep, the belt — and nothing
the engine derives: no RNG state, no opening hand, no enemy health rolls, no
map, no grab bags. Any producer can write one; the engine stands it up through
the same room entry and combat start a map step takes, so nothing a fight's
opening does is skipped, and a run projects the fight it stands in back to one.
A relic's saved count is classified on its definition as a *cycle* (wraps at a
threshold; an unsaid count is rolled below it) or a *ledger* (a fact about the
run; an unsaid count is refused), and the content audit refuses a counting
relic left unclassified. A world stood up from a setup carries the analysis
origin: it is never fidelity evidence. `ScenarioBuilder` is the complement —
it says everything, for a rule test that needs turn three of a specific fight.
`sts2sim combat --setup fight.json --seed SEED` plays one.

## Decisions and actions

`DecisionContext` is an enum whose variants carry only the prompt identity and
visible constraints needed to interpret an action. Expected initial families
include map navigation, combat priority, card/index selection, rewards, shop,
event, rest, treasure, room progression, and terminal. Nested choices are first-
class decision contexts, not callback-local UI operations.

A card choice carries how its candidates are named — a combat instance, a deck
position, or a place in a list of cards that stand in no pile — so an index
means one unambiguous thing, and it carries how few cards an answer may name as
well as how many, because a screen that can be backed out of accepts none at all
while one that cannot does not.

Legal actions are a deterministic ordered list. Their equality and encoding may
use visible portable identities such as player slot, visible map coordinate,
combat-card index plus card fingerprint, creature combat ID plus model
fingerprint, deck index plus card fingerprint, reward-set ID plus reward index
and contents, or choice ID. An index/fingerprint mismatch is
`illegal_action`; it is never permission to select the current object at that
index.

The order and presence of legal actions must be identical for authoritative
states that the player cannot distinguish. Action handles are scoped to their
decision context and are invalid after the context changes.

## `step`, automatic processing, and quiescence

`step` accepts exactly one legal player decision. It applies that decision and
runs deterministic automatic work until the earliest of:

1. an unresolved player choice is at the front of gameplay;
2. a normal player decision is available and no automatic work can precede it;
3. the run reaches a terminal result; or
4. a structured error/invariant failure occurs.

The state is **quiescent** at cases 1–3: no queued effect, death/escape cleanup,
hook dispatch, intent selection, room transition, reward construction, or other
automatic gameplay operation is eligible to run before the exposed decision or
terminal result. Delayed work that is semantically scheduled for a later phase
may remain recorded, but its due condition must be explicit and false. Case 1 is
the one place the queue may be non-empty: what a selection screen interrupted
waits there, and its due condition is the answer. Nothing else may, in a combat
or out of one.

Processing order is part of fidelity:

```text
validate action against current DecisionContext and legal actions
prepare an atomic mutation boundary
apply the player decision
repeat:
    finish the current atomic effect
    dispatch hooks in the pinned order
    enqueue consequent effects in the pinned order
    perform mandatory cleanup/transitions when eligible
    if a player choice now blocks the front of gameplay: stop
    if eligible automatic work remains: continue
    otherwise expose the next normal decision or terminal result and stop
commit Transition
```

Validation occurs before mutation. A rejected action leaves the complete state,
queue, stable-ID allocators, event trace, and every RNG counter unchanged. An
unexpected internal failure is not reported as ordinary illegal input; debug
builds retain enough pre-step state to identify partial-mutation defects.

Hooks use an explicit stable ordering key derived from the game's own
semantics, never hash-map iteration or allocation address. A listener added or
removed while a pass is being dispatched is handled by that same ordering rule,
and tests pin it.

## Boundary examples

### Card play without a nested choice

At `CombatPriority`, the caller selects a visible attack and target. `step`
validates energy, ownership, target fingerprint, and other play conditions;
spends energy; moves the card; resolves its effects and hooks; handles deaths;
and runs any mandatory combat/room transition. It returns only when combat
priority is available again, another choice blocks resolution, or the combat/run
is terminal. Animation completion is irrelevant.

### Card play that requests a card choice

The caller plays a card whose effect must select one of three visible cards.
That play is the one decision consumed by `step`. Effects before the selection
resolve, then the queue suspends with a `ChooseCards` decision and stable choice
ID. The selection is a second `step`. It must correlate to the suspended choice,
resume exactly once, and settle the remaining effects before returning.

### End turn

`EndTurn` is one decision. It runs the whole handover: the two halves of the
player's turn end, the enemy side's whole turn, and the opening of the player's
next one. It may stop early only for a genuine nested player choice or terminal
state — not at animation or command boundaries.

The player's turn has a phase, carried per player and projected by the recorder,
and the handover has two flags of its own:

```text
Start        block clears, energy refills, the hand is drawn
AutoPrePlay  start-of-turn auto-play effects
Play         the player acts
AutoPostPlay end-of-turn auto-play effects
End          from the moment the turn is ended until the sides switch
None         between turns, during the enemy's turn, and once combat is over
```

`ending_player_turn_phase_one` covers the end-of-turn listeners and what the
hand does to whoever still holds it. `ending_player_turn_phase_two` covers
putting the hand down and switching sides, and it stays set until the player
next acts, because the handover it belongs to is not over until the next player
turn is standing. Answering a screen the handover itself opened is
not the player acting again: the flag stays up while the handover is still
owed. `player_actions_disabled` is set for the whole handover in both
directions.

A fight is stood up before it is announced. Every creature joins, each runs what
it does as it joins, and only then is the combat marked `in_progress`. A whole
family of commands does nothing at all while the fight is not yet in progress —
block gained or lost, cards drawn, shuffled, exhausted, or auto-played, orbs
channelled — so a monster that asks for thirteen block as it joins is given
none. Its body is transcribed as written rather than edited to leave the ask
out, because the ask is what the game makes and the nothing is what the game
gives.

A turn opens the same way for either side. What everyone carries is noted as
what they started the turn with; `BeforeSideTurnStart` runs; the enemy side
settles on the move it will make on its own next turn; block falls away for the
side now up and is then announced; the player's setup refills energy, fires
`AfterEnergyReset`, draws five, and fires `AfterPlayerTurnStart`; then
`AfterSideTurnStart`, then the auto-pre-play phase. The opening turn of a
combat runs this same machine, which is why a relic that acts at the top of a
fight and one that acts every turn are one listener each rather than two cases.

Block falling away is two loops over the side, not one pass: every creature on
it clears, and only then is each of them announced with `AfterBlockCleared`.
The split shows with two creatures on a side, because a listener answering for
the first one finds the second already stripped and what it hands the second one
stands. The two loops also do not test the same thing. Clearing is skipped for a
player on their first turn — they keep whatever they walked into the fight with,
and the test is on the creature rather than on the side, so anything else
standing with them still clears. The announcement is skipped for nobody: a
player who kept their block is announced anyway.

The player's setup is the one part of that opening the rest of the turn start
does not wait out. A screen opened inside it leaves `AfterSideTurnStart` and the
rest of the turn start to run with the question still standing; what does wait
on the answer is the auto-pre-play phase. So a screen in the turn setup is
answered with the player already in the `Start` phase of their next turn, and
the phase moves on to `Play` behind the answer. Everywhere else in the handover
the caller waits: the phase moves out of `AutoPostPlay` only once that phase's
own question has been answered.

Each stage of the handover is re-enterable at exactly the point it stopped, and
the position is state rather than a Rust call stack, so a snapshot taken with a
screen up resumes identically. The stages are phase one, phase two, the switch
into the enemy turn, the enemy turn, its end, and the switch back, and each of
them has stages of its own: the
auto-post-play phase, the before-turn-end passes, what the hand does to whoever
holds it, and the flush; the enemy turn holds which enemy it is on, and a
monster move holds the move it captured and how far into that move's body it
got. A creature killed earlier in the same enemy turn does not take one, which
is a per-entry check as the loop reaches it rather than a filter on the list it
snapshotted.

The chain asks whether the fight is over at named points, and every one of them
stops it where it stands, because each is a guard on whatever the chain was
about to do next:

```text
phase one   after the before-turn-end passes, before the hand acts
phase one   after the flush listeners, as phase one finishes
phase two   nowhere: it makes no check of its own
enemy turn  as the enemy side's turn opens, before any enemy acts
enemy turn  after each enemy has taken its turn
enemy turn  as the enemy side's turn ends, before the sides switch back
next turn   as the player's own turn opens
```

So a fight won by an end-of-turn listener ends on the same decision the player
ended their turn with: the hand never acts, the sides never switch, and the
rewards are on the table. A fight the player loses ends the same way, at the
same points — the check settles a pending loss first, which is the build's
order. A won fight is finished by whoever asked, which is where a settled queue
finishes one too; a lost one has already taken the combat with it.

One thing about this chain the game does not settle observably.
`ending_player_turn_phase_two` is cleared when the handover that set it is over,
and the player's own turn setup is the one screen the handover does not wait
for. What the flag reads in the observation written for that screen therefore
depends on ordering the game never pins. The engine keeps the rule it applies
everywhere else: the flag stands until the player acts again, and answering the
screen is that act. The turn-setup resumption test pins that reading.

### Illegal stale target

An action captured before a monster dies refers to its old combat ID and model
fingerprint. Applying it after the decision context changes returns
`wrong_decision_context` or `illegal_action`. State and RNG counters remain
bit-for-bit equal to their pre-call values.

## Content mechanism boundaries

Each of the mechanisms below is shared by many models, so its shape is fixed
here rather than being decided by whichever model reaches it first.

### Commands

Content is not a closed set of effect variants. Every card, power, relic,
potion, monster move, and room effect is a run of calls over one finite command
surface: `CreatureCmd`, `DamageCmd`, `CardCmd`, `CardPileCmd`, `CardSelectCmd`,
`PlayerCmd`, `PowerCmd`, `RelicCmd`, `PotionCmd`, `RewardsCmd`, `OstyCmd`,
`ForgeCmd`, and `OrbCmd` — one enum each, one variant per gameplay operation the
game offers a model. An operation whose whole job is presentation — animation
triggers, VFX, audio, card previews, timed waits — has no variant, because none
of it crosses this boundary. Each family's own documentation names what it
leaves out and why.

`OrbCmd` carries six variants for the eight orb operations, because channelling
an orb by model and channelling one by type are the same call, and the evoke
that every public evoke ends in is a shared body rather than something a model
invokes. Beside those six it carries two variants that are not operations at
all: an orb's write to its own local state — `CardCmd::SetLocal` is the
precedent — and the value an evoke ends on, which is what a Tesla Coil reads to
know where the lightning went.

`ForgeCmd` is a single variant carrying an amount. It stands a Sovereign Blade
up in its owner's hand where they hold none outside the exhaust pile, and then
adds the amount to the damage of every blade they hold, the exhausted ones
included. It rolls nothing, opens no screen, and keeps no state of its own —
what a forge is worth lives on the blades — and it announces itself with
`AfterForge` when it is done.

`OstyCmd` is a single variant too, `Summon`, because summoning is the only way a
pet ever reaches the field. The rest of what a pet amounts to is not a command
family at all: it is a creature on the player's side, a link from that creature
back to its owner, and the listener conditions that read it.

### Card selection

`CardSelectCmd` is the card-screen surface, and every screen on it is a
first-class decision boundary rather than a callback. A screen names where its
cards come from, how many it wants, whether it may be backed out of, and what
runs once it has been answered:

```text
a combat pile:  the hand, the draw, discard, exhaust, or play pile
the run deck:   for an upgrade, for a removal, or for anything else
an offer:       cards that stand in no pile at all, laid out to be chosen between
```

Which source a screen has is also what names its cards in an answer, because the
game identifies a chosen card one way per family: a combat card by its combat
instance, a deck card by where it stands in the deck, and an offered card by its
place in the offer. A screen therefore carries that identity alongside
its candidates, and an index means a different thing in each.

Two shortcuts come before any question is asked, and both are the game's. A
screen with nothing to offer hands its caller an empty list; a screen that wants
at least as many cards as it has takes them all, unless its caller demanded
confirmation. Either way the commands after the screen still run, with whatever
was handed back bound to them, which for an empty answer is nothing to act on
rather than nothing happening.

A screen over the draw pile is never presented in pile order, because the pile
order is the draw order. The game sorts it by rarity and then by displayed
name; a name sort is culture-variant, so what crosses this boundary is the
machine-stable order used when a selector answers for the player: rarity, then
model ID, stably, so identical cards keep their relative places. Every other
pile is presented in pile order, and the run deck in deck order.

A multi-card screen with more answers than the action list can lay out is
offered the way the game's own screen is worked: the player clicks cards in and
out of a set one at a time, and the selection completes as soon as the set is
big enough. Past the
enumeration cap each offered action names the selection so far plus one more
card, an action reaching the minimum finishes the answer, the decision carries
the selection standing between the clicks, and the legality gate takes a whole
set in one action — the shape a recorded run answers with — whether or not the
list laid it out.

A rest site's smith and a merchant's card removal are this surface too, not
engine special cases: each is an option that opens a deck screen, and neither
pays out until a card is actually chosen. Backing out of the smith leaves the
rest site standing.

Two of the screens make their own cards rather than being handed any. The
rewards grid is laid out over what a run of card creations came to,
concatenated in the order the calls were made; the choose-a-card screen has a
form that does the same for the three cards it lays out. The bundle screen is
answered with one whole bundle rather than with cards, so its candidates carry
the grouping alongside them and an answer names every card of one group: it is
the one screen whose answer is not a subset of what it offers. The enchant
screen offers the deck cards the enchantment
may go on, in deck order; the amount it carries is what the preview over each
card shows and nothing acts on it, because the body that opened the screen is
what enchants whatever it hands back. It is the one screen with no
manual-confirmation guard on the take-them-all shortcut, and every caller in the
pinned build asks for one count rather than a range, which makes the two the
same thing. The game's stack of automated selectors, which answer screens in
place of a player, has no command variant of its own, because this simulator's
decision boundary is that mechanism. It is a mode instead: while it stands, a
screen answers itself with the first cards it offers rather than being put to
the player, which is the whole of the one selector the game pushes in gameplay.

The transform screen offers the deck's transformable cards, which is everything
that is not a quest card and can leave the deck. What it shows beside each of
them — what that card would become — is a preview built off canonical models
that rolls nothing, so none of it crosses this boundary.

A transform itself is a card leaving its pile and a replacement standing where
it stood. The replacement is either named by the body or rolled out of what the
card's own pool leaves once the game's filter has had its say: a card that is
neither a status nor a curse keeps only the three rolled rarities, a quest card
and the three unrolled rarities read the colorless pool rather than their own,
and a card is never among its own options. Which pool a card reads is settled
where the registry is built, because it depends on nothing that changes during a
run. Two shapes of the call are both here and they differ in the order the deck
takes the replacements: one call over a list takes every original out and then
puts the replacements back in the order their originals stood, while a body's
own loop finishes each card before it looks at the next. Which stream a roll
reads is the caller's and it is fidelity: three relics read the run's niche
stream and one reads the player's own transformations.

A command names its arguments by selector rather than by identity, because the
model issuing it does not know the identity either. A creature selector names
the owner, whoever the play or move was aimed at, everyone on the side opposite
the owner's, one of them rolled on the combat-targets stream, everyone on the
owner's own side, whoever worked the listening power onto its owner, or the
owner's pet. Random targeting rolls once per call, over the living opponents as
they stand at that moment, and reads no dice at all when there are none to hit;
an attack re-derives the candidates before every hit, so a kill between hits
narrows the next roll. There is no adjacency selector, because the pinned build
has no notion of one creature standing beside another. The one thing a slot does
carry is order: an encounter lists its spawns in slot order, each creature
remembers the slot it spawned in, and that order is the order the enemy side
takes its turns in — which matters as soon as one enemy's move depends on what
another already did this turn. It is also the order a random target is rolled
over, so a creature summoned into a slot in front of its summoner changes what
the next boomerang hits. The side is re-sorted into slot order every time a
creature with a slot joins, and a slot the encounter does not name sorts in
front of every one it does. It is not the order anything is rendered in; the
projection sorts creatures by combat ID. The pet selector reads the pet off
whoever issued the command: it names nobody where the owner has none, and it
names the corpse where the pet is down, because the game answers with a dead pet
too — every body that must not act on one checks for itself whether the pet is
missing. A pet is never named by a selector that sweeps a side: `AllOpponents`,
`AllAllies`, `OtherOpponents`, `OtherAllies`, and `EveryCreature` all pass over
it, and so does a potion aimed at a player, which is why an enemy's sweep never
touches the thing standing behind its target.

A card selector names the issuing card, a filtered
pile, one card of a pile rolled on the combat card-selection stream, the
deck-owned card a thief prefers, or whatever a selection screen just handed
back.

Where the game names no applier, the command carries none rather
than defaulting to the owner: that is what leaves a power one power brought with
it attached by nobody, so nothing takes it away by dying.

A value a command reads is an expression rather than a number: a fixed amount,
a calculation base plus a per-unit amount for each of something counted, or a
choice between two values on a state test. That is the shape the game gives a
calculated value, and it is what lets one attack command express a flat six, a
slam worth the
block its owner stands behind, and a strike that scales with how many Strikes
the combat holds. A command works its value out once, before it lands, so every
creature a single command reaches takes the same one; an attack is the
exception, because the build recomputes it per hit — once per hit, against the
single creature that hit found, and against nobody at all when the hit lands on
several at once. How many hits an attack makes is settled before the loop opens
and is not worked out again.

A few control forms complete the surface, standing for what a model body writes
around its calls: a guarded run of commands, a counted repetition, a loop over
cards one at a time, and the bindings a body makes — a card selection resolved
once so that every command after it acts on the same cards rather than rolling a
new selection, what a blow or a whole swing came to, and what a gain of block
was actually worth once the passes had their say. One further call stands for
what a `Modify*` listener *answers with* rather than for anything it does: an
addition, a scale as a
percentage, a cap, or a value outright. Outside a modifier pass it has no value
to work on and does nothing.

A card's or a potion's commands are queued, one entry per command, and settle in
order; a monster move and a listener body run theirs where they stand.

A selection screen stops the body it was opened from where it stands. What is
left of that body becomes one queue entry, and so does what is left of the
listener pass the body was in, and so does each pass the dispatch site still
owes, and so does each frame of the turn machine or the damage path around
them. The frames unwind innermost first and each puts itself one place further
along the front of the queue, so the order they are picked back up in is the
order the game resumes them in: the rest of the body, then
the rest of the pass, then the passes behind it, then the frame that was running
the dispatch, then whatever was already queued. Once the player answers, the
commands after the screen go in front of all of it with the picked cards bound
to them.

What a frame has to remember is exactly as much as resuming it needs: which
step of the body it had reached, and where it stood in whatever loop it was
running. So each frame is a step index rather than a Rust call stack, and the
whole of it is state — an exact-state key covers it, and a snapshot taken with a
screen up resumes to the same place the original does. The frames that carry one
are the turn handover and each stage inside it, the enemy turn and a monster
move within it, an attack between its hits, and a `CreatureCmd::Damage` between
the creatures one call reaches.

Everything between the queue and a body that opens a screen has to be able to
put itself down that way, and only some hook events can pause at all. Some
cannot: the whole `Modify*` family, `AfterSideTurnStart`, `AfterEnergyReset`,
`AfterBlockCleared`, `AfterCardChangedPiles`, `BeforeCardPlayed`,
`BeforePotionUsed`, `AfterCombatVictory` and `AfterApplied` have no way to
suspend, so a body that asks a question there is content being wrong and fails
as an invariant naming the hook event. Where a hook event can pause, its
dispatch site decides what pausing means. Most sites stop the pass on the screen
and make the caller wait; that is the discipline described above, and it covers
`AfterCardPlayed`, `AfterDamageGiven`, `AfterDamageReceived`,
`AfterPlayerTurnStart`, the two auto-play phases, and `AfterFlush`. The
turn-boundary sites — `BeforeSideTurnStart`, the three before-turn-end passes,
`BeforeFlush`, and the two after-turn-end passes — let each listener pause on
its own, carry on to the next listener with the screen still owed, and join the
paused bodies at the end of the site, so several screens can be outstanding at
once. This engine carries five of those six sites; the sixth,
`BeforeSideTurnStart`, is a listed divergence, and a screen reached there fails
as unsupported content naming the discipline rather than as an invariant.
Neither failure carries on with the rest of the frame while
the player is still looking at a screen. Exactly one piece of registered content
reaches the turn-boundary discipline — the power that asks which cards to keep
back before the hand is flushed — so the mechanism buys that one body and
nothing else.

Outside a combat there is no queue to defer into, so what is left of an
out-of-combat body waits on the run in the same shape a combat's queue holds,
and is settled by the answer. Obtaining a relic is the path that reaches it: a
relic obtained at a chest, a merchant, an event, or a reward runs its own
`on_obtained` body, that body can open a deck screen, and what the body does
after it runs once the screen has been answered.

Once a fight is over, what a model already set off still reads its own dice — a
selection still rolls — but every command the game guards against an ending
combat hands nothing over.

### Modifier hooks

Relics, powers, cards, potions, and monster models are one listener surface in
the game, not separate systems. The engine models them the same way: a single
ordered listener sequence that every hook pass walks.

The sequence is part of fidelity, because additive and multiplicative damage
modifiers apply in listener order. There are two of them, and which one a pass
walks is the event's own:

```text
the combat list:  for each creature, allies before enemies, in spawn order:
                    that creature's powers, then
                    for the player: relics -> potion slots -> orbs -> piles
                    for an enemy:   its monster model
the run list:     deck cards, then the combat list
the run list
  with no combat: deck cards -> relics -> potion slots
out of combat:    deck cards -> relics -> potion slots
```

The third is the run's own listeners with no combat list behind them even where
a fight is standing. The reward passes are the sites that use it.

Which list a dispatch site walks is load-bearing: the damage and HP-lost passes,
`AfterCurrentHpChanged`, `AfterDamageReceived`, `BeforePotionUsed`,
`AfterCombatVictory`, and `AfterCardChangedPiles` let the run deck's own cards
speak before anything in the fight does, while the block passes,
`AfterBlockBroken`, `AfterDamageGiven`, `AfterCardPlayed`, and every
turn-boundary site do not.

A card is a listener wherever it stands. Every card of every combat pile is on
the combat list, in pile order — hand, draw, discard, exhaust, play — and every
card of the run deck is in front of the run list.

A `HookEvent` is one *pass*, not a dispatch site. Ending a side's turn runs
three passes over the whole listener list — `BeforeSideTurnEndVeryEarly`, then
`BeforeSideTurnEndEarly`, then `BeforeSideTurnEnd` — so the engine names each
pass and a dispatch site declares its passes as an ordered list. Ordering
therefore lives in the data rather than in whoever wrote the listener. A card
changing piles is a two-pass site of the same kind, run every time a card moves
between piles, is created into one, or leaves the state, and it carries the card
and the pile it came from. Drawing a card is another: an early pass and then the
base one, which is what puts a power that plays every Strike as it is drawn in
front of a power that binds the cards a turn deals.

A fight finishing is two sites rather than one: `AfterCombatEnd` runs first,
with the combat still standing, and `AfterCombatVictory` after it. The game
runs the first for a lost fight too; here a lost fight is a lost run, settled
with the combat dropped, so nothing that listens there could act on the answer.

A damage is one arithmetic run rather than a sequence of rounded steps. The
whole of it is worked in exact decimal — the card's own enchantment first, what
it adds and then what it scales, and only then the additive, multiplicative and
cap passes over every listener — and a whole number is taken of it where the
blow lands and nowhere else. That matters wherever something scales a value
before something else adds to it: a corrupted attack is worth half again as much
and the mover's strength is added to what that came to, fraction and all. The
engine holds the chain in hundredths for that reason; every other value chain is
whole numbers from end to end.

Three of the passes are *chaining* rather than additive: `ModifyEnergyGain`,
`ModifyCardPlayCount`, and the pass over a card's X value hand each listener the
value so far
and take back what it says the value now is, so a listener that turns a gain
away answers with nothing rather than with a negative offset. Two of the three
are followed by a second pass over exactly the listeners that changed the value,
in listener order, which is how a one-two punch is spent for the attack it
doubled.

A listener body can ask whether the play it is answering for is one the player
chose rather than one the game played for them. That is what separates two
relics that both count a turn's plays: one skips the plays nobody chose and the
other counts them all.

A play the player chose queues its whole tail up front — its body, the
enchantment's, the pass over the listeners, and finally the card being put down
— so a screen the body opens simply leaves that tail on the queue. An auto-play
cannot: the game runs the whole of it inside whatever asked for it, so it runs
where it stands. What it still owes when a screen interrupts it —
the iterations of its body it has not run, the pass that announces it, and the
pile the card goes to — is held on a resume frame of its own instead, which is
how a card nobody chose finishes a play a screen stood in the middle of.

Sly is a keyword read at exactly one place: a card discarded while it is Sly
plays itself for free. Sly-ness is captured as each card of a discard is
*queued*, so a card made Sly by the discard's own listeners or by the draw
behind it does not answer for that batch; the plays themselves are deferred
until every card of the batch has been discarded and the paired draw has
happened. The end-of-turn flush of the hand goes through the pile command rather
than the discard one, so nothing Sly fires there.

A card burning up is a dispatch site of its own. `CardCmd::Exhaust` puts the
card in the exhaust pile, writes the burn into the combat history, and then runs
`AfterCardExhausted`. There is deliberately no bulk form of it, because the
listeners for one card have to finish before the next card starts burning. All
three ways a card can burn go through it: a body asking for it, a played card
whose result pile is the exhaust pile, and a card burning up in hand as the turn
ends. Only the last is flagged as caused by the card being ethereal, and that
flag is what a dark embrace defers on rather than pays for.

Each pass walks a list snapshotted before it began and checks every entry for
liveness as it comes up. A listener added during a pass is not in that
snapshot and so does not act until the next one. A pass a selection screen
interrupted carries that same snapshot across the answer, and every entry it had
not reached yet is checked for liveness at the moment it is reached — which is
after the player has answered, so what the screen's own commands did to the
board is what the rest of the pass sees.

What liveness means is not the same for every kind of listener, and the
difference is load-bearing. The per-entry guard asks whether the state still
contains the model, and that question resolves differently per kind:

- a **power** is checked by its *owner*, never by itself — the guard is the
  owner's combat state, and nothing detaches a removed power from its owner. A
  power taken off a creature that is still in the fight therefore keeps
  answering for the rest of the pass it was snapshotted into, and stops from the
  next pass on, when the snapshot no longer lists it;
- a **monster model** is likewise checked by its creature's combat state;
- a **relic**, **potion**, **card**, **affliction**, **enchantment**, and
  **orb** are each checked by themselves as well as by their owner: one removed
  from the state mid-pass stops where it stands. A card is checked for removal
  from the state only, not for which pile it is in, so a card that moved piles
  mid-pass keeps answering;
- every listener also stops when its owner stops listening: a creature that died
  or escaped has been taken out of the combat, and a player who is fully dead is
  no longer active for hooks.

So the rule is not "removal stops a listener": it is "leaving stops a listener,
and everything but a power also stops when it is itself removed". A power that
strips another power mid-pass does not silence it until the next pass.

A listener declares a condition alongside its event, standing in for the test a
listener body would otherwise open with: whether its owner is one of
the creatures whose turn is starting or ending, which side is taking the turn,
whether its owner is on the side currently acting, whether the card just played
was an attack, whether the play was aimed at somebody, and — for the passes
below — whether its owner is dealing or taking the value being worked out. What
the listener does past that test is a run of commands, and the listening model
and the creature carrying it are what those commands' selectors resolve against:
a power decrementing itself, working another power on its own owner, or reading
its own stacks all say so by naming the listening power rather than a model ID.
A test the condition vocabulary does not cover is a guarded run inside the
body.

The events the covered content needs are `BeforeSideTurnStart`,
`AfterBlockCleared`, `AfterEnergyReset`, `BeforeHandDraw`,
`AfterPlayerTurnStart`, `AfterSideTurnStart`, the three auto-play pre-play
passes and the post-play one, the three before-side-turn-end passes,
`BeforeFlush`, `AfterFlush`, the two after-side-turn-end passes,
`BeforeCardPlayed`, `AfterCardPlayed`, `BeforeAttack`, `AfterAttack`,
`BeforePotionUsed`, `AfterDamageGiven`, `AfterDamageReceived`,
`AfterCombatEnd`, `AfterCombatVictory`, `AfterApplied`,
`BeforePowerAmountChanged`, `AfterTakingExtraTurn`, and the `Modify*` family
below.

`BeforeAttack` and `AfterAttack` bracket a whole attack rather than each of its
hits, which is what lets a power bind one swing, add itself to every hit of it,
and be spent once for the lot. What the power binds to is the card that swing
came from, which is the whole of what the refusals behind it read; nothing ever
clears the binding, so a power that survived the swing it bound goes on
answering for that same card, refuses every other, and spends nothing more.
`BeforeHandDraw` runs ahead of the pass that settles how many cards the turn
deals, so a power that stocks the hand stocks it before the count is worked
out. Neither site can pause: an attack has no way to suspend, and the turn
setup around `BeforeHandDraw` is re-enterable only between its steps, so a body
asking a question in either fails loudly rather than coming back to a step that
had already half run. `AfterBlockBroken` and `AfterCurrentHpChanged` are
dispatched as well although no registered body answers them, because the
dispatch condition is the part that has to be right before a body arrives to
test it.

An intent advertises what its move would deal right now, so a weakened monster
shows a smaller number than its move's base: the intent is worked out through
the same passes an actual blow is.

#### The `Modify*` passes

A value nothing has landed yet is offered to the whole listener list before it
does. Each such dispatch is a run of passes over that list:

```text
damage:   ModifyDamageAdditive -> ModifyDamageMultiplicative -> ModifyDamageCap
block:    ModifyBlockAdditive  -> ModifyBlockMultiplicative
HP lost:  ModifyHpLostBeforeOsty -> ...BeforeOstyLate
                                 -> ModifyHpLostAfterOsty -> ...AfterOstyLate
a power:  ModifyPowerAmountGiven -> ModifyPowerAmountReceived
```

An application also announces itself before any of that.
`BeforePowerAmountChanged` runs from the top of both application paths — the
one that stands a fresh instance up and the one that joins the instance already
there — before the given pass, before the received pass, and whether or not the
creature working it is standing in the fight. It is the one pass about a power
application that carries the *card* the application came from, which is why the
card is threaded through an application from end to end: a relic that doubles
what one card of a fight works has to bind that card here and cannot read it
anywhere else.

One further pass is not over a value at all. The result-pile pass chains where a
played card is put down through the listener list, and then walks exactly the
listeners that answered with something else for
`AfterModifyingCardPlayResultPile` — which is how a rebound is spent for the
card it sent back to the draw pile.

The additive pass adds into a running total. The multiplicative pass gathers
every factor and works the whole product out once, so nothing is lost between
two of them: three quarters of half again as much is nine eighths, not one. The
cap pass takes the least limit any listener offered and holds the value to it.
The HP-loss passes chain, each listener seeing what the one before it left.
`ModifyUnblockedDamageTarget` runs between the before- and after-pet HP-loss
pairs, and that seam is where the pet lives: a Die For You
answers there, taking the blow off its owner after the before-pet pass has had
its say and before the after-pet one does. Block has no cap pass at all.

Nothing about a pass reads how the value was worked out. Each listener makes
that test itself, which is why a scale skips unpowered damage and a cap does
not: a shell that admits no hit worth more than nine holds down a relic's flat
nine as readily as a strike. That test is the listener's declared condition —
whether its owner is dealing or taking the damage, whether the block is going to
its owner or came off its owner's card, whether the power being worked is being
worked onto its owner — and past that test the body is a run of commands like
any other listener's.

`ModifyPowerAmountReceived` is the one pass with a second half: the listeners
that said something about the amount are walked again for
`AfterModifyingPowerAmountReceived`, in listener order. That is how an absorber
is spent — it says the debuff lands for nothing, and the second pass takes a
stack off it for having said so.

Which listeners a second half reaches depends on the pass, and there are two
answers rather than one. The damage, block and result-pile passes gather a
listener that *answered* with anything but the identity: a non-zero addition, a
factor that is not one, a cap that actually bit, a pile that is not the one
offered. The HP-lost passes, `ModifyGoldGained`, `ModifyCardPlayCount` and
`ModifyOrbPassiveTriggerCount` gather one that left the *value* somewhere else
than it found it. The difference
is a body that writes the value it already held: a Buffer writing nothing over a
blow block already stopped whole answered, but changed nothing, so it is not
spent for it.

Every instance of damage walks one path, the one `CreatureCmd::Damage` walks:
the modifier passes, then block unless the damage is unblockable, then the
HP-loss passes over what block did not stop, and then the settled result is
offered to the listeners. Block a power grants its own owner is unpowered and no
listener scales it.

One call reaching several creatures walks that path as two loops rather than
one: every creature takes its damage first, and only once all of them have is
each settled result offered to the listeners. The split is load-bearing, because
a body answering for the first creature sees the second already hit. A creature
that was already dead when the call reached it takes no part. What died of the
call is settled at the end of it, which is why an attack's next hit sees a
narrowed field.

What the result carries is how much block took, how much block was left
standing, what the blow cost past block, whether block was allowed to stop it
at all, and whether it found its target standing and was worth at least what
they had left — enough to answer both what a listener body asks of it and what
the loop that dispatches them asks. Four passes read it, in this order, and
each has a dispatch condition of its own read off the result rather than off
the listeners:

```text
AfterBlockBroken       block took some of the blow and had nothing left after
AfterCurrentHpChanged  the blow cost the target HP
AfterDamageGiven       every result there is, whatever it came to
AfterDamageReceived    unless the blow killed the target and it is still dead
```

Nothing narrower than that belongs in the dispatch. A body that only answers a
blow which cost its own owner something says so in its own condition, because
the loop offers it a blow block stopped whole either way. The one skip the loop
does make is the killing blow: what the blow killed is answered for by the death
rather than by the fourth pass, and the test asks about the creature as well as
the result, so a target something stood back up in between is offered it after
all.

All four fire off any damage, not only off an attack, which is why a power that
reacts to a blow of its owner's being stopped is written as an
`AfterDamageGiven` body testing the result rather than as an event of its own.
Being fully blocked is not the same as costing nothing: a blow worth nothing
against a creature standing behind block counts, one against a creature with no
block does not, and damage block was never allowed to stop never does.

`AfterBlockBroken` and `AfterCurrentHpChanged` carry only the creature it
happened to. The game offers an HP delta alongside; nothing registered reads it,
so it is not carried. `AfterCurrentHpChanged` is dispatched from every path this
engine has that moves a creature's HP — the damage loop, a heal, and setting HP
outright — on the same tests the game makes at each: what was asked for rather
than what the ceiling left room for.

Timed debuffs tick down as the enemy side's turn ends, on whichever side is
carrying them: each of them listens for `AfterSideTurnEnd` and counts itself
down. A debuff that lands on a player-side creature sets a skip marker that the
next tick consumes, which is why it lasts through the turn it landed on. The
marker is set only when the debuff stands up a new instance, and from the
power's declared type rather than from what the amount makes of it.

### Powers

A power is a set of listener bodies plus what it declares about itself. What it
declares is what the game settles by declaration rather than by behaviour: its
type, how it carries its amount, whether a second application
joins the instance already there, whether it may be worked below zero, whether
its owner counts as a secondary enemy, whether its owner leaves the combat on
death, and whether it is one of the wrappers that lends a stat.

How it carries its amount is `Counter` or `Single` — shown or hidden. That
distinction is what makes a negative amount read as a debuff: only a counter
power that may go negative does. What a power counts as is therefore a function
of the amount, not a fixed label — a debuff worked backwards reads as a buff,
which is why nothing turns it away.

Whether a second application stacks is the instance type. With no instance type
a second application joins the instance the creature already carries. An
instanced power stands up its own instance every time, which is what a power
that carries a stolen card or counts down on its own timer needs. One instanced
per applier joins only what the same applier left, and stands up its own for
anybody else. The search for an instance to stack onto happens first, before any
listener sees the amount, so an absorber turns away a stacking application and a
fresh one alike.

An instance is a listener in its own right, and two of them are two listeners.
A creature carrying two instances of one instanced power answers every pass
twice, each answer read off its own instance's amount rather than off the sum;
a command that names the listening power acts on the instance that issued it
rather than on the first one of the model the owner holds; and a removal takes
one instance off, not every instance of the model. That is what lets two bombs
tick down independently and go off on different turns.

Applying a power runs the amount past `ModifyPowerAmountReceived`, then attaches
it if anything is left. A power falls away when it is worked to nothing: at
exactly nothing if it may go negative, and at nothing or below if it may not.
Decrementing is an application of one the other way, which is why it removes by
the same rule. Counting a duration down is that same decrement, unless the skip
marker is standing, in which case the marker is spent instead.

A temporary power is a wrapper naming the stat it stands for and which way it
moves it. It works that stat onto its owner as it
lands and again for whatever a later application adds, and at the end of a turn
its owner took part in it takes itself off and gives the same amount back. A
potion that lends strength for a turn therefore applies only the wrapper; the
strength is the wrapper's doing.

What a power does past that is listener bodies. A power that brings another
with it says so in `AfterApplied`, which is dispatched to the power itself
rather than walked over the listener list. Being surrounded is a
`ModifyDamageMultiplicative` body that reads which way its owner is facing out
of its own variable and tests whether the dealer carries the marker for the
other side, and a `BeforeCardPlayed` body that turns its owner to face whatever
they just struck at — before the blow is worked out, which is why striking at
the one behind you never earns the harder hit.

### Relics

Relics are instances with per-run state, not bare model IDs: a relic that counts
cards played carries its counter in the player state and is part of the state
key. Relics belong to the player, not to the run, so a combat scenario can hold
them without a run around it.

A relic listens on the same surface a power does, the `Modify*` passes included:
one that pays out on upgraded cards joins the additive damage pass with a body
guarded on the card the damage came from, rather than being a number the damage
path reads off the relic.

It also has one body of its own that is not a listener on that surface: what it
does as it is obtained, which runs once the relic is the player's. That is
where a relic that upgrades, removes, or transforms a card asks its question,
and it is the only body that ordinarily runs outside a combat, which is why an
out-of-combat run has to be able to put itself down. The body runs on its
owner's creature where a fight is standing and on the run's own state where one
is not.

Obtaining is the whole of `RelicCmd::Obtain`: the relic joins the inventory at
the end or at a named place, leaves both grab bags unless it is stackable — the
one thing that keeps a relic in the bags after it has been taken — writes down
the floor it was taken on, is set up for this player, and then its own
`on_obtained` body runs. Three commands undo it and they are not the same
thing. `Remove` takes the relic out of the inventory. `Replace` removes one and obtains
another at the index the first stood at, so the inventory's order survives.
`Melt` leaves the relic in the inventory and takes everything else away: it is
off the listener list, so nothing it declares runs, and it can no longer be
traded. Every one of the three ends by announcing the removal to the relic
itself, which no relic of the pinned build answers.

Two things a relic is worth are read off the model rather than written beside
it. What a shop charges is its rarity — 175, 225, 275, 200 for the four rolled
ones, and a price out of reach for the three a shop never stocks — and a shop
additionally refuses the five relics whose `allowed_in_shops` is false. Whether
an event may take a relic back is `is_tradable`, which is asked of the instance
the player is holding rather than of the model: not a relic whose limited uses
are spent, not one that does its work the moment it is picked up, not a melted
one, not one that brings a pet, and not a starter, event, or ancient relic.

`is_used_up` is the dynamic half of that, and ten relics answer it for
themselves. Every one of those answers is the same shape said against what the
instance keeps — a count that has run down, a count that has run up, or the
relic's own boolean — so it is declared as that rather than written as a body. A
relic given a number of uses is holding that number the moment it is taken, and
spends it down itself.

A held relic carries per-run state of its own: a counter, a second counter
beside it, a boolean latch, the cards and models a setup saved, and whether it
has been melted. All of it is part of the state key, because all of it changes
what the relic does next. A relic that needs several booleans keeps them as bits
of the one count it has.

A handful of what a relic declares is a predicate the engine asks rather than a
body the relic runs, because the game settles it by declaration too: whether its
owner's block is cleared for the turn and how much of it survives, whether the
energy refill replaces what is standing or adds to it, whether a combat room is
made to pay a potion, whether a card of a type joining the deck or standing on a
merchant's shelf is upgraded on the way, which pool stands behind the
character's own on a card reward, whether any potion reaches the belt at all,
whether a draw its owner's own turn makes is refused, how many cards its owner
may play in a turn, and what a card costs. The last of those runs as a pair of
passes over a cost: everything the early pass says stands before anything the
late pass says, whichever listener said it, and a relic may name the count it
has to be standing on for its rule to answer at all.

Whether a relic may be offered at all is the same kind of predicate, asked
before a bag hands anything out: a relic whose answer is no leaves every bucket
at the next pull rather than at the moment it stopped. Three shapes of that
answer are declared — the relic that stops being offered from the act-three
chest on, and the two that read the number of players, which this baseline
answers as the single-player run it supports.

Encounters do not heal; relics do.

Every relic of the four rolled rarities, of the event pool, and of the ancient
pool is registered, less the relics of the two stand-in characters no run
plays. The ancient pool loses none of them to those characters: no relic of a
character's own pool is ancient, so which relics an ancient may deal is the
same list whichever character is walking the run. Every one of them has its
body but one: that relic's own availability predicate is the number of players,
so no single-player
bag, offer or ancient can hand it over, and the cards it would deal are the
multiplayer-only ones a single-player pool never carries. One relic, a paper
phrog, has no listener body at all: Vulnerable reads the relic off whoever is
swinging and raises its own multiplier, so the relic is transcribed on the
power.

Two of the surfaces relics reach are shared mechanism rather than anything of
theirs. A turn that does not hand over is asked for at the top of the switch
from the player's side to the enemy's: while anything answers yes the side stays
the player's, the round does not wind on, the enemy never acts, and
`AfterTakingExtraTurn` runs for whoever bought it. And exactly one relic turns
the automated card selector on in gameplay, for the burst it plays its owner's
opening turn with; see [Card selection](#card-selection).

### The passes an act's map is worked through

Laying an act's map out is followed by two passes over the run's own listeners —
the deck's cards first and the relics behind them, with no combat list behind
them. The first pass chains the whole map, so a listener that answers replaces
it outright; the second may annotate the map it is handed but may not return a
different one.

This baseline lays every act's map out up front, each on a stream of its own, so
the dice are the same wherever the generation happens. What is not the same is
what the run is holding when the passes run, so the passes run where the game
runs them: as the act is entered, and again whenever something asks for a map to
be generated afresh — which one relic does the moment it is taken.

Two maps are laid out from scratch rather than by bending the act's own, and
both belong to the content: the hourglass every path converges through one
centred treasure point of, which reads a stream of its own, and the single
column of sixteen fixed points, which reads no dice at all. What a listener does
with the marks is its own state: a relic saves the coordinates it marked, so a
map generated again in the same act carries the same marks, and a saved
coordinate that is no longer the kind of point it was re-rolls the whole set.

A relic can also widen the map. Free travel is an OR fold over the run's
listeners, and while anything answers yes the points a player may walk
to are every point of the row above rather than the children of the point they
stand on. A step onto a point that was not a child is the step the relic paid
for, and it spends one of its uses — so a relic with three of them is three
steps off the graph and then a relic that does nothing.

Three things a relic can be are its own state rather than a body. A relic can
keep cards away from the deck and hand them back one at a time, which is a list
of whole cards saved on the instance. A relic can be *wax*, which is a saved
boolean and nothing else: what reads it is the body of the relic that handed the
wax one over and melts it again. And a relic can turn a death away — the veto
pass `CreatureCmd::Kill` makes is two walks of the whole listener list rather
than one, the early one and then the late one, so a potion that answers the
first is spent before a relic that answers the second is asked at all.

Whether a chest holds anything is asked as a treasure room fills itself, which
happens inside the room's entry and after the room has been announced. A
listener answering no empties the chest outright: no relic is pulled, no gold is
paid, and neither is rolled for. Because the announcement comes first, a relic
counting the treasure rooms it has seen has already counted the one it is
standing in.

Neither an event relic nor an ancient one is ever rolled — no bag holds either —
so each reaches a player only through the event or the ancient that gives it.
Every one of those rooms is registered, so every such relic is reachable: Neow's
twenty-seven single-player options, the nine ancients, and the events that hand
one over are all pages a run can walk into.

### Enchantments

An enchantment is a model that lives on a card, and it is the second thing a
card carries that listens: it stands on the combat listener list immediately
behind the card it is on, and on the run list immediately behind that card's
deck version. Its liveness guard is the card's, so it stops
answering when the card leaves the state or stops carrying it.

Five of its events are not passes over the listener list at all. The damage and
block dispatches each open by asking the card's own enchantment directly — what
it adds lands on the value, what it scales lands on the whole of it — and only
then walk the listeners, so an enchantment's scale applies before any listener
has added anything. The play count asks the fifth, `EnchantPlayCount`, the same
way. All five are named as events of their own — `EnchantDamageAdditive`,
`EnchantDamageMultiplicative`, `EnchantBlockAdditive`,
`EnchantBlockMultiplicative`, `EnchantPlayCount` — so that an enchantment body
is written in the same command surface as every other body, but the dispatch is
a direct ask rather than a walk.

What an enchantment may go on is one shared base test plus whatever the model
adds: no status, curse, or quest card; no unplayable card standing in the run
deck; and no card that already carries an enchantment, because nothing in the
pinned build is stackable. The extras are a card type, an attack-or-skill pair,
a tag, whether the card gains block, the card's own Exhaust keyword, a basic
Strike or Defend, and a playable card whose cost is not X.

What an enchantment does to the card it sits on — a keyword put on or taken off,
a cost made free — is derived from the enchantment wherever the keyword or the
cost is asked for, rather than written into the card once. The game rebuilds a
card's values on the same occasions, after a downgrade and after a save is
loaded, so the two come to the same thing and the derived form cannot fall out
of step. Upgrading a cost clamps it at nothing, which is why a card an
enchantment made free stays free through an upgrade.

An enchantment's own play body runs once per play of the card, after the card's
body and before the listeners on the play. That is one queue entry per iteration
of the play loop, and it stands only where the card is carrying an enchantment
as the play opens, which is asked inside the loop; nothing in the pinned build
puts an enchantment on the very card whose body is running.

An enchantment carries an amount, which is saved state and is projected beside
the card — the recorded fingerprint's `enchantment` is an object of the model,
the amount, and the enchantment's own saved properties, not a bare model ID. No
pinned enchantment declares a saved property. What it writes on itself for the
fight — the damage a Momentum has worked up, the mark a Glam is spent — is
private to the fight rather than saved state, so it belongs to the combat copy
and starts over with each fight; the amount does not, and a Goopy writes its
growth through to the run-deck card so that it outlives the fight.

### Potions

A potion definition carries its target type, where it may be drunk, whether an
in-combat roll is allowed to conjure it, and what drinking it does, as a run of
commands: the same vocabulary a card is written in, not a parallel one. Using a
potion is `Action::UsePotion`, carrying the slot index, the potion model, and a
target when the potion takes one. A potion that targets a player names that
creature explicitly rather than being untargeted, because multiplayer targets
another player the same way. Using a potion clears its slot before its body
runs, and drinking one consumes no randomness of its own.

Drinking or discarding a potion is an action on the player's action queue, and
where the belt is offered follows from where that queue would run it: the
queue holds an action back only behind one of the player's own that is waiting
for an answer. Out of a fight no screen is such an action — a reward set, an
event, a room, a merchant, and an obtained relic's screen all stand beside the
queue — so `Action::DiscardPotion`, and `Action::UsePotion` for a potion that
may be drunk out of a fight, are offered at every decision but the run's end,
a card screen included, and run at once beside whatever screen stands. Inside
a fight every screen belongs to the card, potion, or listener that opened it,
which waits at the front of the queue; a potion asked for there runs only once
that action has finished, so the belt is offered at the next decision rather
than beside the screen. An event whose definition sets `locks_potions` holds
the belt from the moment it begins until it finishes: its page names potions
by slot, and nothing is drunk or thrown away in between.

One potion is not drunk at all outside a fight: it is thrown, and what it is
thrown at is a merchant, who is not a creature. That is a `TargetType`
computed rather than declared, and it comes with a usability check of the same
shape — the potion is offered only where there is a merchant standing there to
throw it at, a real one or the fake one an event stands up. A real merchant pays
a hundred gold for it; the fake one turns on the player for it, pushing a fight
that offers his rug and every relic still stocked on the shelf he was holding.

Potions held in a slot are hook listeners while held, sitting behind the relics
in the ordered sequence, and a slot that is empty holds nothing to listen. A
potion is checked for liveness by its own slot, so one drunk or discarded
mid-pass stops where it stands. The one thing any of them answers is the veto
pass `CreatureCmd::Kill` makes before it lets a creature go down: a Fairy in a
Bottle answers no for its owner, and the body that follows the prevented death
drinks it where it stands rather than queueing anything, because the game
re-checks the creature immediately afterwards. That is also why it is automatic:
the belt never offers it, and what drinks it is the death it turned away.

Every potion roll is the same roll, and the callers differ only in the stream
they read and the options they hand it. The options are the player's character
pool followed by the shared pool, in each pool's own declaration order; one
float picks the rarity (a tenth rare, a quarter more uncommon, the rest common)
and one index picks a potion of that rarity out of what is left, with each
potion drawn taken out of the options for the draws that follow. That last part
is why a body asking for several potions at once is one call rather than a
repeated one: a batch never rolls the same potion twice, and rolling them one
at a time is a different run of dice. A body that fills the belt a slot at a
time is the other shape, and it stops at the first procure that is refused
rather than rolling on. A card, a relic, or a potion that rolls one reads
`combat_potion_generation`; a combat reward reads the rewards stream and a shop
its own. The in-combat form additionally drops the potions whose
`can_be_generated_in_combat` is false — the ones that heal or resurrect — and
drops them from that roll alone. The three rarity lists a reward and a shop
index into are derived from the same pool rather than written down beside it.

A full belt does not stop a potion being offered. Selecting the reward where
there is no room simply fails, so the reward is on the screen, can be clicked,
and quietly does nothing: it is not taken, the belt is unchanged, and the screen
stays open. A merchant's shelf and an event's answer a click the same way: the
purchase reports a no-space failure before any gold is spent, and everything
that would have followed it — the shelf, the sale — is skipped, so the offer
stands and the price is still unpaid. Making room is a separate action — the
belt slot is emptied and nothing the potion does runs — and it is available in
every decision the run offers except the terminal, which is where the game's own
belt sits.

Drinking one out of a fight is a non-combat action, and the only thing that
stops a non-combat action is being in a combat: the queue skips one while a
fight is standing and nothing else. A screen does not. A card reward waiting to
be answered is an action gathering a choice at the front of the queue, and a
drink enqueued behind it runs all the same. What the belt may offer out here
is narrower
than in a fight rather than wider: a `CombatOnly` potion has no fight to be
drunk in, and a potion aimed at a player names the run's own player creature,
whose combat ID is unset.

### Pools

A pool is a model of its own here, as it is in the game. Every card pool, every
potion pool and every relic pool of the pinned build is registered with the
membership its own model declares, in the order that model declares it, and
every consumer reads that one registered list: a reward and a merchant index
into a card pool, a transform rolls against one, a grab bag is filled from a
relic pool, and a potion roll walks a character's own potion pool and then the
shared one. Nothing keeps a second copy of a membership beside the registry, so
a pool cannot disagree with itself.

The order is not presentation. A draw is positional — a random take shuffles the
whole filtered list before it takes anything, a rarity list is the pool filtered
in place, and a grab bag is the pool bucketed in the order it is walked — so a
pool written down in the wrong order is a different run of dice from the first
roll. Each pool's membership is pinned by a snapshot test that names every model
in it in the game's own order.

What a pool model declares beyond its membership is one rule and some
presentation. The rule is `is_colorless`, which four of the twelve card pools
answer yes to — the colorless pool, the event pool, the token pool and the
deprecated one — and which is the whole of what a filter asking for a colourless
card tests. That filter reads the card's `visual_pool`, which is the card's own
pool except for the ten event cards that name a colour instead, and which is not
the pool a transform rolls against: a quest card stands in a
pool that is not colourless and still transforms against the colourless one.
Everything else a pool model carries — a title, a card frame, a deck-entry
colour, an energy icon — is outside this boundary.

A character pool is registered as a single-player run with every epoch revealed
reads it. Unlocking prunes by epoch and by player count, so a character's card
pool is registered as the eighty a rolled rarity can offer:
the basics, the ancients and the multiplayer-only cards it also holds are
registered as cards and named as standing in the pool, but are not in the list a
rarity indexes, because every reader filters exactly those out first. A
character's potion pool answers with nothing at all until its epoch is revealed,
and this baseline reveals it.

Three pools hold what nothing rolls. The event relic pool is the hundred and
forty only an event hands over; the event and token potion pools are what a page
or a body deals; the fallback relic pool is the circlet a refinement gives where
the starter it was pointed at has no refined form. Their membership is what says
which pool each model stands in, and no bag or roll is filled from them. The
deprecated pools hold the one stand-in a save reads back where the model it
names is gone. The game's two placeholder pools are not registered at all: one
of them is what the two unplayable stand-in characters carry, and neither holds
a model a run can reach.

Three relics stand in two pools at once — a lasting candy, a razor tooth and
sparkling rouge are in the shared pool and the event pool both — and asking a
relic which pool it is in answers with whichever is reached first. Only the
energy icon reads that, and both grab bags still hold all three. No card and no
potion of the build stands in two pools.

### Acts

Which acts a run has is rolled before the run exists. Each act model declares
which act of the run it can be, and the acts sharing an index are that index's
candidates: act one has two, the default and an alternate gated behind an
epoch, while act two and act three have one each. One act per index is rolled
off `act_selection`, a stream built from the seed string alone and read only
here — the lobby builds it, rolls with it, and drops it, so it is not one of
the run's own streams and never appears in a run's counters.

Three rules ride on the roll, and each of them moves the stream or does not:

- an index rolls even when it has a single candidate, so act two and act three
  each cost a draw whose answer could not have been anything else;
- an unlocked alternate act the local profile has never seen is taken outright
  rather than rolled for, and taking it stops that index's scan, so the index
  reads no dice at all and every later draw moves up one; and
- the lobby's act-one setting is applied after all three rolls and reads no dice
  either. It replaces whatever index 0 settled on, or leaves it standing.

A lobby that left the character to chance resolves it immediately afterwards,
one draw per such player off the same stream, over the game's whole character
list rather than an unlocked subset. So the act list, the profile's discovered
acts, and the act-one setting are all run-defining inputs that a seed does not
carry: the setting is `act_one` on `RunConfig`, and discovered acts are a
profile input of the explicit manifest. Recorder 0.10.1 supplies them in
`run.start.data.profile.discovered_acts`, separate from the unlock projection.
Older recordings omit that metadata and assume all supported acts discovered.

An act the room tables here do not cover, or a rolled character this baseline
has no loadout for, stops as unsupported content naming the model rather than
falling back to the default act or the configured character.

What the run is then stood up from is the character's own: the starting health,
the gold, the starting deck, and the starter relic, with the character's relic
pool added to the shared ones in the player's own grab bag. The bag is filled
and shuffled before any act is drawn, so the character is a run-defining input
in the same sense the seed is.

An act's rooms are drawn from two bags into one list. The weak encounters come
first from a bag of their own, the regular ones after them from theirs, and each
draw is tested against the last entry of the list they share — so the first
regular encounter of an act will not repeat the last weak one, nor anything
tagged like it. A draw that nothing in the bag can satisfy takes anything, and a
draw that has candidates re-rolls until one of them comes up, which is dice the
list's own contents decide.

Every act's map is generated up front, each on a stream of its own, so any act
can be generated at any time. What an act's map is shaped by — how long it is,
how many rest sites and unknown points it carries — is the act's own.

Walking on from a boss ends the act. Between acts the player stands nowhere: the
next act opens with its map, its start point the only place to walk to, its room
odds back at base, and its encounter and event lists back at the front. That
start point is the act's ancient, and an ancient restores whoever walks in — by
everything the player is missing, or four fifths of it at Weary Traveler.

### Ascension

A run is played at one ascension level, zero through ten, and the level is a
`RunConfig` field because it is a generation input the seed does not carry.
Eleven is not a harder run this simulator declines to generate: the game's own
cap is ten, so anything above it is a malformed configuration. A run above the
baseline is not the registered baseline preset any more, so naming both is
refused too.

The eleven names are an ordered enum, and the order is the rule: a run has a
named level whenever its numeric level is at least that name's, so each name is
the level at which its effect starts applying and every level above it keeps
that effect. Two questions are asked of the level everywhere: whether a run has
a level at all, and which of a pair of numbers to use, the ascended one or the
baseline. A model that reads the second is transcribed with both of its numbers
whatever level the run is at.

Where a level is read decides where it is threaded. The content registry is
built at the run's ascension, because the question is asked of the run rather
than of the model: the monsters' sizes and damage and the merchant's removal
price are declarations that branch, so they are built at the level rather than
patched afterwards. Everything else is read at the site the game reads it — as
the map
is laid out, as the run is stood up, as an ancient is walked into, as a purse or
a card offer is rolled, as a shop is stocked.

The ten rules:

- **Swarming Elites** multiplies the map's elite count by one and six tenths and
  rounds it: eight points where the baseline has five. It changes nothing else
  about how a map is generated, and no dice move. Three more elites still have
  to stand somewhere, so a map that cannot hold both loses rest sites to them —
  the repair pass tops the types back up in the order shop, elite, rest site,
  unknown, and the elites are repaired first. It does not replace the treasure
  row with elites: no run turns that on, and the Warden ascension it exists for
  is not one of this build's levels.
- **Weary Traveler** leaves an ancient healing four fifths of what the player is
  missing rather than all of it, truncated. Neow is the one ancient that zeroes
  the player before healing, so a Weary Traveler run opens on four fifths of the
  bar rather than whole.
- **Poverty** takes a quarter off a combat purse and off a chest's gold, each
  truncated back to a whole number. A purse is rolled out of the
  scaled range rather than scaled after the roll, so it is not the baseline
  purse times three quarters. The one encounter that overrides both gold
  properties declares its purse outright and is never scaled.
- **Tight Belt** takes one of the three potion slots a run opens with.
- **Ascender's Bane** appends its curse to the starting deck, written on the
  first floor like everything else the run opens with.
- **Inflation** raises the shop's card-removal price and what each removal adds
  to it.
- **Scarcity** halves the rare threshold of every source that rolls one, halves
  the drift a roll that came up short adds to the next one, and halves how much
  each act already walked raises an offered card's chance of arriving upgraded.
  It also raises three common odds, and nothing reads them: a rarity roll asks
  for the rare threshold and the uncommon one above it and lets common be the
  remainder, so those numbers are transcribed beside the odds rather than in
  them.
- **Tough Enemies** and **Deadly Enemies** are the two per-monster levels, and
  every monster that branches carries both of its numbers.
- **Double Boss** draws the last act a second boss out of the ones it did not
  roll — one draw on the up-front stream, immediately after that act's own
  rooms, and only for the last act — and lays out the point that fight stands on
  a row behind the first boss, reachable only through it. The act's boss room
  fields the second encounter once a boss of that act has already been fought.
  Proceeding past the first boss opens the map rather than asking for the next
  act, because the fight behind it is still a point to walk onto; the second one
  ends the act. The last act's bosses pay nothing either way.

None of this moves a stream at the baseline, and the level a run carries is what
the projection renders for it.

### Events

An event room is generated on entry from the act's pre-generated event list at
the index the run has reached, exactly as a monster room is generated from the
encounter list. Walking into the room is what counts an event as visited, and an
ancient counts as one. Both counters advance on the room the run resolved to and
not on the point that led there, so an unknown point that rolled a monster room
moves the encounter index and leaves the event index alone.

An event outlives its room. The synchronizer that made the event model holds it
until the next event begins and clears the list, so an event whose body never
finished is still there — and still projected, with the page it was
left standing on — after its room has been walked out of, sometimes a whole act
later. An event that did finish is standing on a page with no options, which is
no choice at all. What the projection names is therefore the event the run last
began rather than the room it is in.

Each event owns a floor-independent local RNG seeded from the run seed, the
player slot, and the event's entry name. That RNG rolls the event's variables
when the room is entered, and never touches a run stream. The variables are
settled before the page is built, so an event that declares them settles all of
them first, in the order it declares them; an amount an option pays or a price
an option is measured against then names the var it reads back. A var is rolled
whether or not any option goes on to read it, which is what a punch-off's purse
is. A price the room rolled is what locks an option, too: a luminous choir
discounts its tribute as the room is entered and lays the option out locked
where the player cannot meet the discounted number, while the test for whether
the event may be offered at all reads the undiscounted one, because the act
asks the canonical event before any room exists. The event's RNG is one stream
for the whole visit rather than one per read: it is made as the event is set
up, so the page's own generation leaves the counter where an option's body
picks it up, and the active room carries that counter across the choices that
follow. An event that rolls again inside an option — a tablet upgrading one
card of the deck for every decipher — reads the same stream on, not from zero.
An option is a small vocabulary of page-level outcomes: lose HP, gain gold, add
a card or curse, obtain the next relic from the front of the bag, lay generated
cards out, offer generated rewards, or fail as unsupported. Every one of those
that is a single call over the command surface issues it rather than reaching
into run state on its own. What has no creature to act on outside a combat —
the HP an event takes and the healing an ancient gives — comes off the run's
own totals instead.

An event page is a description plus an ordered option list. Choosing an option
either finishes the event or replaces the page, so a multi-page event is a
sequence of option lists in the same active room. A replacement page can be one
the body rolled for — a Trial's three verdicts — or one it shuffled and cut: a
tinker's table cuts three card types down to two and then that type's three
riders down to two, each cut an unstable shuffle over the list the body wrote.
A page can also be built out of the run rather than declared: a relic trader
pairs the player's own tradable relics, sorted and shuffled on the event's
stream, against three pulled off the front of the bag; a doll room sorts and
shuffles three relics onto a second page; Wongo's pulls the rare in its window
as the page is built; and an endless conveyor rolls one weighted dish onto the
belt, puts the next one up behind every grab, and forces its own dish every
fifth plate. An amount on a page is either
written down or rolled on the event's own RNG as the room is entered. An option
that lays cards out suspends a choice: those cards are drawn evenly from a pool,
offered as they are, and whatever is picked joins the deck. An option can also
carry an alternative to the offer itself — sacrificing every card laid out
rather than picking one — which is an option on the choice screen and not a
second page.

An event can also own a shelf. One event stocks its own inventory as the room is
entered rather than laying out a page at all: it shuffles nine relics on its own
stream, takes six, prices each on the shops stream, and returns no options. That
shelf is bought off the way a merchant's is, and the two things it is not are
both guarded on the room being a merchant room: nothing prices it
through the merchant pass and nothing refills what is bought off it. Walking on
is what leaves the page, so the shelf stands beside the map as one decision.
Projection v1 describes a merchant room's inventories and nothing else, so the
shelf is state this simulator keeps and a recording cannot see.

An event can also stand a screen of its own up that is not a card selection at
all. The Crystal Sphere's fog board is one: the option that opens it builds an
eleven-by-eleven grid on the active room, and the divinations spent clearing it
are actions taken against the event rather than against a page. Its page stays
standing under the board, and the event does not finish until both the board
and the rewards screen the last divination puts up have been cleared, so the
room is left when that screen is dismissed. The board is exact state — which
cells are clear, where the fifteen items sit, how many divinations are left,
and which items have been revealed. Projection comparisons leave
`/special_event` uncovered, because the renderer does not emit that field. Each
recorded click still names its cell and tool and therefore drives the exact
board action. A preceding `event.crystal_sphere.set_tool` changes only the
selected screen button; the adapter settles and ignores it because the click
repeats the tool.

An event can fight without leaving. Entering a combat from inside an event
pushes it onto a room stack rather than replacing what is standing: the event's
page stays up under it and stays projected, and the room the run stands in is
two rooms deep. The map point's history gains a second room, but only the room
at the bottom of the stack marks its type visited, so a nested fight moves no
act counter and takes nothing out of the act's encounter list — the event names
its own encounter. The fight can also be handed rewards the option built, which
are offered with the fight's own in the order the reward screen sorts them.

The room the player is standing in is the top of that stack, and it is what a
body asking about the room reads: a relic that pays a second card offer for a
monster room answers for the fight, not for the event below it, and the reward
screen names the fight as the room its rewards came from. Whether the event
resumes afterwards is the room's own flag. Where it is false, proceeding past
the rewards opens the map and leaves both rooms standing until the next point
is walked onto, which pops the whole stack. Where it is true, proceeding
resumes the event instead: the page that was standing under the fight picks its
body back up, and the fight is still standing while it does, which is what lets
a battleworn dummy read whether the fight was won or its clock ran out. The
body that resumes is closed by the same rules an option's own body is — a
screen it opens holds the page open, and nothing else finishes it.

An event that stands a fight up can also write on the encounter first. A
battleworn dummy's setting is what its encounter branches on when it makes its
monsters, so the option names the setting and the encounter fields the battle
friend it names; an encounter generated with no setting at all refuses to stand
up, and the game treats that as an error.

An event laid out as a fight makes its creatures earlier still. A combat-layout
event builds its combat state as its room is entered, right after its page is
built, so the encounter's monsters and their rolled health are spent there; the
fight an option then walks into stands those same creatures up rather than
rolling new ones.

Choosing an option runs its body at most once: an option that has been chosen
refuses to run again, which every option of the pinned build takes as its
default, so a page clicked twice does its work once and the second click does
nothing at all. Nothing stops a *different* option of the same page from being
taken while the first one's screen transition is still in flight, and what the
game does then depends on which transition lands first, which is not a question
the model layer answers: a recording that takes two options off one page is not
replayable, and says so rather than picking one. The same fade is why a
recording can hold observations in which a pushed room is not standing yet, and
those are not reproducible either.

### Room resolution and generation

Map point types other than unknown map to one room type each. An unknown point
rolls its room type on the unknown-map-point stream against run-level odds that
drift: monster, elite, treasure, and shop carry base weights, anything unrolled
is an event, the rolled type resets to its base weight, every other type
accumulates its base weight again, and all of them reset between acts. Elite is
disabled at the supported baseline. A type the point is not allowed to become
takes no part: it is neither rollable nor accumulates, which is what keeps a
shop off a point that follows a shop or whose every path onward leads to one.

A card in the deck can answer that question too. A quest card carried into the
act it names replaces the whole table with the single room type it wants, and
names the event the act's next pull becomes. The roll is still read — the value
is taken before the table is walked — and a table with nothing left on it leaves
the answer where it started, so the drift does not move either.

An encounter generates its own monsters off a floor-local RNG seeded from the
run's seed, the floor, and the encounter's own entry. What it generates is one
of a handful of shapes: the same monsters every time, the same monsters with
the openers the encounter writes on them, one roll rotating a list of openers
around its spawns, a coin between two arrangements, a branch rolled and then
rolled into, draws without replacement out of a pool, and the numbers a spawn
is given before it stands up. One encounter is none of those: a fight an event
stands up can have its composition written on it by that event rather than
rolled, and no map point can ever be such an encounter.

Every room the run stands up takes the next room id, and that counter is reset
when a map location is entered rather than when a room is left.
Almost every room is the first at its point and so is room zero; a room the run
stands up without moving — the fight an event pushes onto its own room — takes the next id along, and the
projection carries it.

Obtaining a relic takes it out of both the player's grab bag and the shared
one, so no later pull can offer what the player already holds. Every pull also
prunes first: a relic whose spawn condition the run has walked past is dropped
from both bags before anything is drawn, which is how a bag of sixteen relics
that only exist to be found early empties itself once the act-three treasure
chest is behind the run. The prune happens on the pull rather than on the floor
that invalidates them, and it changes what a later pull lands on, so it is not
an optimisation.

A merchant's shelf is five character cards with one of them on sale, two
colorless cards, three relics, three potions, and a card removal, stocked in
that order and in that interleaving of the rewards and shops streams. What each
entry charges is the price it rolled worked through `ModifyMerchantPrice`
and truncated: that pass is asked every time a price is read, so a relic
bought off this very shelf changes what the rest of it costs, and the pass is
recomputed at every boundary here for the same reason. Two relics answer it and
both answer multiplicatively, which is why a shelf held under both is worth two
fifths rather than three tenths off. The pass is asked only where the room
standing is a merchant's, so a fake merchant's shelf is untouched by it.

What a slot does once it has been bought off is cleared or refilled, and a pass
over the run's listeners decides which. The one relic that answers it answers
with an owner test and nothing else, so it is a declared property of the relic
rather than a listener body. None of that happens until the purchase itself has
finished: the gold, the item, and any screen the item's own arrival opens all
settle before the shelf is touched or `AfterItemPurchased` is announced, so a
purchase whose item asks a question parks the rest of the sale behind the
answer, and the shelf and the sale's listeners settle when the answer comes
back. A refilled card slot re-populates from the pool it was built for, which
re-rolls its rarity, its pick, its upgrade, and its price, and loses the sale
it may have been on; a relic slot rolls a fresh rarity and pulls from the back
of the bag past what the shelf still holds; a potion slot rolls a fresh potion
against an empty blacklist: the shelf's own potions are gathered up and then
not passed on, so nothing is actually excluded. Both of those are guarded on
the room standing being a merchant's, so a shelf an event owns is priced by
nothing and refills after nothing: what is bought off it is cleared. It is
bought off the same way all the same, and walking on is what leaves it, so the
shelf stands beside the map as one decision the way a merchant's does.

A relic can buy the lot. `AfterRoomEntered` fires from a merchant's room like
any other, and one relic answers it by running the whole shelf through the same
purchase a click does, with the cost ignored — the character's cards, the
colourless ones, the relics, the potions, and last the card removal, which it
asks for with no way to back out. Each of the four lists is walked as it stands,
so a slot that refills itself behind a purchase is not bought twice, and no gold
moves at all, so `AfterItemPurchased` is announced with nothing spent.

A treasure room grants gold rolled on the rewards stream and offers one relic: a
rarity rolled on the treasure-room stream, then a pull from the front of the
shared grab bag. A merchant pulls from the back of the player bag, so the two
never compete for the same end of a bucket. The chest's gold is not a claimable
reward, and taking the relic leaves the player standing in the room until they
proceed. Straight after the room's own gold comes one special case: a quest card
of the deck that marked this very point as the act's map was laid out pays what
it promised, on top of the room's own, and leaves the deck for it.

A rest site's option list is generated as the room is entered and before the
room is announced, and it is content data: the heal and the smith always, the
mend where the run has more than one player, and then whatever the run's own
listeners add — a deck card's option before a relic's, because that is the
order the run's listener list walks. The list belongs to the room, so a relic
picked up after the player sat down changes nothing until the next rest site.
What each option does is not a model in the game either, so it is transcribed
here: rest, upgrade, cook two cards away for maximum HP, clone
every card an enchantment marked, dig a relic out of the bag, hatch an egg,
kindle a candle, lift a girya. Two of them ask for cards, and both are deck
selection screens rather than an option per card.

An option that is on the list but not enabled is shown and cannot be taken: the
smith with nothing upgradable, the cook with fewer than two cards the deck may
lose. Taking one runs its body, and the body's answer decides what happens next.
An answer of no leaves everything exactly as it stood — that is backing out of
the smith's screen, and it is why nothing is spent and the room stays shut. An
answer of yes writes the choice into the run's history and then clears the whole
list, unless a listener says the rest of it may stand, in which case only the
option taken comes off. Either way taking an option is what enables the proceed:
a rest site cannot be walked out of until one has been taken, and it is left
through the map rather than by finishing.

The heal is 30% of maximum HP, and resting claims a reward set of its own
whether or not anything puts something on it: the heal runs, its own pass runs,
and then `TryModifyRestSiteHealRewards` builds a list a listener may add to. A
set with nothing on it is counted and not shown, and one with something on it
holds the rest site open until it has been taken.

The recorded projection of a rest site lags the list by one step. The list
itself is trimmed the moment an option is taken, because that is what decides
whether another one may be; but the recorder writes its rest observation from
the option-chosen event, which the build raises before the trim, and it holds
that value until the next thing the player does is observed. So the observation
an option-chosen event produces still carries the option that was just taken,
and the live list — trimmed, or empty where nothing was left standing — is what
the observation after the player's next act reads back. An option is not
finished while a screen it opened is still standing either, so an option that
put a reward set up keeps the whole list on show until the set has been
answered. The lag is a projection fact and nothing else: which options are
still standing is state all the same, because it is what the room offers next.

Elite and boss rooms are combat rooms with different reward parameters, not a
different room family: their gold ranges and card rarity odds differ, and an
elite additionally offers a relic pulled from the front of the player's bag.
Rewards generate in one order and are shown in another, sorted gold, potion,
relic, then card.

Every reward set is shown in the game's own order rather than the order a body
put things on it: gold, then potion, then relic, then what a thief handed back,
then a card offer. That sorting runs for a set a relic or an event built as much
as for a room's own.

A card reward on a screen is answered or it is not; a screen dismissed with
cards still on it leaves the reward unanswered rather than completing it, which
is why a skipped offer can be reopened while resolving the current rewards.
Proceeding to map navigation finalizes those decisions: the map offers neither
reward claims nor another Proceed. The dismissed combat reward set remains only
for replay observations until the next room is entered. Replay consumes a
redundant map-open `room.proceed` without a simulation step, but explicitly
rejects collecting dismissed rewards as unsupported. This boundary avoids
agent decisions spent closing and reopening UI; see
[`fidelity-divergences.md`](fidelity-divergences.md#reward-decisions-are-final-before-map-navigation).

`RewardsCmd::OfferCustom` returns the moment the set is marked skipped, so
everything behind it — the event finishing among them — runs from there: the
state after a rewards screen is dismissed is the state the recorder's
observations in front of the skip already show.

A set the player may *not* skip holds its offering body the way a
card-selection screen does. Offering such a set does not come back until the
set is done with, so what the body wrote
after the offer happens after the last reward has been taken: Neow's bones deal
two relics and only then charge the curse they cost.

A relic's acquisition can open a card-selection screen while its reward set
remains active. The claim's continuation runs before the body waiting for the
set. A completion frame preserves that boundary across snapshots and marks the
relic selected only after its acquisition finishes. Precarious Shears therefore
removes the chosen cards and deals its damage before Neow's Bones can charge
the curse, regardless of which relic is claimed first.

An acquisition can also offer a nested reward set. The parent set retains its
identity, selected rewards, and waiting continuations below the new set. Only
the top set accepts actions, while replay observations include all outstanding
sets in opening order. Taking or skipping the nested set restores its parent
and finishes the acquisition before completing the parent claim. Closing a
nested set cannot finish the room or release the parent's waiting body. This
state survives snapshots: Kaleidoscope's card offers finish before Neow's Bones
can complete its relic set and charge its curse.

A room's reward set is not settled when the room has rolled it.
`TryModifyRewards` runs two passes over it and then a third over exactly the
listeners that put something on the list, and what they added is rolled behind
everything that was already there, in the order it was added. Those passes walk
the run's listeners with no combat state at all — the deck's cards, the relics,
the potion slots — because a reward belongs to the room rather than to the
fight that filled it. A body can also offer the list it has just built as a set
of its own, which is what a relic that hands over potions or card offers as it
is taken does.

A card offer is likewise not settled when it has been rolled. Which pools it
draws from is asked first, so a relic can put another pool behind the
character's own; the cards are rolled against those pools, with a rolled rarity
the pool cannot answer walking up to one it can, wrapping at rare; and then two
passes run over the offered cards, followed by a pass over whoever changed one.
That is where a card of a type is upgraded, and where another option is added.

Rolling those cards is one call, and it is the same call wherever a card is
made against a pool: a room's own reward, an event's, a relic's, and the cards a
body lays out to be picked between. What the call is given is the pools, a
filter over them, an odds type, a count, a source, and the flags that decide
which of the passes over it may speak. Each card it makes is blacklisted for the
draws after it, so one call never offers the same card twice, and each is
followed by its own upgrade roll unless the flags spare it — the roll draws its
float before it asks whether the card can be upgraded at all, so sparing it is
what saves the draw rather than the card being unupgradable. Uniform odds roll
no rarity at all: the draw is one index over everything the filter left that is
neither basic nor ancient, which is what lets a body name a single rarity and
get a card for a single draw. Only a creation whose source is an encounter reads
the run's drifting rarity odds, and it moves them on what the roll said rather
than on what the pool could answer.

A card reward keeps the options it was built with, because building the offer
again is the whole of a reroll: the button is on the screen while the reward
carries one, taking it throws the offer away and rolls another with those very
options, and it is offered once. A reward's reroll options are the same record
as its own, so the second offer rolls what the first would have.

### Piles, shuffles, and card identity

A card in a combat carries an identity of its own, allocated in shuffled pile
order as the combat opens. A reshuffle takes the discard pile only, puts it in
canonical order first so the result does not depend on the order the pile
happened to be in, and then walks a Fisher-Yates shuffle over it under the
shuffle stream. That canonical ordering is the unstable sort the game's platform
library performs: two cards with the same fingerprint come out in whichever
order that sort leaves them, and which physical card lands where follows from
that. The sort is reproduced in `sts2-engine/src/netsort.rs`.

Every sorted-then-shuffled selection over the deck works the same way, and the
comparison is the game's own: the model, then the upgrade level, and nothing
else. Two cards that compare equal are left wherever that sort puts them, so
which of five identical strikes a whetstone upgrades is decided by the sort's
walk. A tie-break of the simulator's own would make the sort stable and pick a
different card.

### Interrupted machines

A monster's move machine can be interrupted: an illusion that dies holds the
field on a revive move, and a creature that is stunned stands through a turn.
Either way the machine owes that move before it moves on, and picks itself back
up at the state it was interrupted on. What leaves a rock bug stunned is a power
of its own answering `AfterDamageGiven` and finding that block stopped the whole
blow it just dealt.

A stun names which state the machine picks back up in one of three ways, and the
choice is resolved where the stun is worked rather than where it wears off: the
caller names a state outright, or names none and gets the move the creature last
performed, or asks for the one the machine would have gone on to — which is what
a Flutter names as the blow that empties it knocks its owner out of the air.
Only move states are logged, so the state a machine would have gone on to is a
move's own follow-up with no roll behind it.

A stunned turn is not always an empty one. A stun carries a body, and almost
every caller hands over one that does nothing; a Slumber hands over the
waking that takes the beetle's plating off. The body is held on the creature,
because the stun writes it and the turn after performs it.

A creature keeps two records of its machine, because a move is *entered* a turn
before it is *performed*: the machine settles on the next state as the current
move finishes, so what a repeat weight reads — how many times the creature has
stood on a state — is not what a monster keeping "how many times have I done
this" as a count of its own reads. A body counting its own work reads the second
of those, and inside the move's own body it has already counted this one.

Numbers a monster keeps for itself are written on the creature. A pressure gun's
current damage and the pressure a giant is holding as it goes down are private
to the monster in the game; here they are named values on the creature, read by
a command, by an amount, and by the intent that shows what the next move will
do.
Nothing projects them. An encounter can write one of them onto a spawn before
the fight stands up, and roll it as it does: how much is knocked off a punch
construct's health before the punch-off begins is such a number, rolled per
construct on the encounter's own RNG in spawn order.

A random branch weighs each of its states by the repeat rule and the cooldown
it was registered with, and then by its own weight. That weight is not always a
constant: a branch may weigh itself by what the fight looks like, so a branch
whose weight depends on the fight carries the question it asks and the weight
it answers with when the question answers no. The questions are the machine's
own vocabulary — a number the creature wrote down for itself, a teammate's
telegraphed move, a free slot in the encounter — and a rat asking whether it
may call for backup asks all three at once. The question is asked only for a
branch the repeat and cooldown rules have not already zeroed out of the roll,
which is where the game asks it.

Every stat a monster branches on ascension for carries both of its numbers, and
the registry is built at the run's own level, so a branch is folded once when
the models are built rather than asked at every read.

### The pet

The Necrobinder walks with an Osty, and it is a creature rather than a
mechanism: the pet is a creature on the player's own side whose `pet_owner`
names the player behind it. Everything the pet does is that link being read.

`OstyCmd::Summon` is the whole of the summoning surface, and it does three
different things depending on what is standing. Nothing at all for an amount of
zero — the call returns before the listeners behind it run. A maximum raised by
the amount and a heal for exactly that much, where a pet is already up. And
where none is, or where the one there is down, a creature is made or the corpse
is picked back up, its maximum is *set* to the amount rather than raised by it
— setting a maximum holds the current health down to the new maximum and never
lifts it — and the heal that follows is what brings a revived pet back to full.
A pet made from nothing is given a Die For You, applied by nobody and marking
nobody.

Die For You is what makes a pet worth having. It answers
`ModifyUnblockedDamageTarget`, the pass that runs between the
before-pet and after-pet HP-loss pairs, and it moves a powered attack aimed at
its owner onto itself: not an unpowered one, not one aimed at anybody else, and
not while it is down. What lands on the pet is worked out in the owner's terms
in two places. The blow is stopped by the owner's block rather than the pet's —
the creature whose block is spent is the target's owner where it has one and the
target itself where it does not — and whatever the blow was worth beyond what
the pet had left
tramples through onto the owner behind it. The trample runs only the after-pet
half of the HP-loss chain: the before-pet half has already had its say on this
blow and is not asked twice.

A pet is on the player's side, so the enemy turn never reaches it and no sweep
names it. It is also not a target: `AllOpponents`, `AllAllies`,
`OtherOpponents`, `OtherAllies`, and `EveryCreature` all pass over it, and a
potion aimed at a player finds the player. When it dies its corpse stays in the
fight, held there by the power it carries, and a later summon revives that same
creature. When the player dies the pet is killed with them, its own death
settled the same way any other is, before the player's own is.

The pet swings through `DamageCmd::AttackFromPet`, which makes the pet the
dealer and leaves the card the source. Every card body that does it guards on
the pet standing — the game's own guard is that test plus the shake it plays,
which is presentation — so a play with no pet issues nothing at all. A blow the
pet lands is what a Calcify adds to and what a bone flute answers, both of them
asking the same question: is the creature dealing this the listener's own pet.

The projection carries the pet as a creature like any other, with two
differences the renderer has to make. Its `pet_owner` names the player slot
behind it, and it carries no `player` of its own — only a player-side creature
that is not a pet is a player. And the room history's list of monsters seen in a
fight puts the pets after the enemies, which is the order the game's own walk
produces.

### Orbs

The Defect walks with an orb queue, and it is neither a creature nor a pile: it
is a list on the player's combat state with a capacity, and orbs stand in it in
the order they were channelled. Every orb model is one definition — a passive
value, an evoke value, which of the two turn boundaries it answers, and a body
for each — and every instance carries its own two values, because what raises
them raises the instance rather than the model.

The queue's capacity is `base_orb_slots` plus whatever a fight has added, held
to ten. Slots added and slots removed are the same command with the sign
flipped, and taking slots away evokes the orbs that no longer fit, from the back
of the queue forward. A channel into a full queue evokes the front orb first and
then puts the new one at the back; a channel into a queue with no slots at all
puts one slot under it first, which the game guards on both the base count and
the capacity being nothing.

An orb's two values are read rather than stored: `ModifyOrbValue` runs over the
listeners every time either is asked for, which is how Focus raises every orb of
its owner's at once and how a gold-plated cable reads only the front one. The
floor at nothing is Focus's own, not the pass's: a Focus below zero can take a
Lightning orb's passive down but never below nothing, because Focus's own body
takes the greater of nothing and what it worked out.

The turn pass is its own re-enterable step. `OrbTrigger::BeforeTurnEnd` runs as
its owner's turn ends and `OrbTrigger::AfterTurnStart` as it opens, and each
walks the queue one orb at a time: for every orb,
`ModifyOrbPassiveTriggerCount` says how many times it fires, and then it fires
that many times — the count pass runs for every orb whether or not that orb
answers this boundary, so a queue of Plasma orbs still reads the cables' dice
at the turn's end. Whether the owner is still in a combat is asked twice per
orb, once before the count and once after it, so a fight that ends mid-pass
stops where it stopped. The whole of it is a frame — which trigger, which orbs,
how far along, how many fires are left — so a body that opens a screen puts the
pass down and picks it back up.

An orb body runs in a context of its own rather than in the context of whatever
played the card: the listening model is the orb, the owner is the player, and
the target is what the passive was handed. That is why a Lightning orb evoked by
a Dualcast aimed at its owner still strikes an enemy. What the evoke ends on is
carried too, which is how a Tesla Coil and a Thunder find what the lightning
reached.

Orbs are listeners. The combat list walks the player's relics, then the potion
slots, then the orbs, then the piles, so an orb answers a hook after a relic and
before a card. And a projection carries them: the
player's `base_orb_slots` and, inside the combat block, the queue's capacity and
one entry per orb with its model, its settled passive, and its settled evoke.

### Deaths, minions, and illusions

A creature that dies leaves the combat, and any power it worked on another
creature goes with it. Whether it leaves is a power's own declaration: an
illusion says its owner is not removed, so it stays on the field, keeps its
buffs, sheds what was worked on it, and waits on a revive move that heals it
whole on its turn. A power can bring another power with it as it lands, attached
to the owner by nobody, which is how an illusion marks itself a minion. Being a
minion is likewise a power's own declaration rather than an engine special case.
A minion does not keep a combat alive, and one still standing when its side
loses goes down with it. Winning strips the player of everything the fight gave
them; what died holding a power keeps it.

A creature can also leave a fight without dying. An escape takes it off the
field with the HP it had, detaches its powers without running anything a removal
would run, and settles no death: nothing counts a kill and no death listener
answers. The fight writes down what got away, because a monster room pays the
share of its purse that matches the share of the encounter's monsters that were
put down — and the two counts are counted differently, one per creature that
left against one per monster *model* the encounter ever laid down, which is the
game's own asymmetry. A purse held to nothing is not offered at all, and
nothing is rolled for it either. Only a monster room's purse is held to it: an
elite and a boss pay theirs whole however many of their monsters walked away.

### Cards a deck can hold

A card joining the run deck is stamped with the floor it joined on, and the
stamp is an overwrite rather than a default: `CardPileCmd::Add` writes the run's
current floor over whatever the card was carrying. So
a copy of a card that has been in the deck since the first floor joins on the
floor the copy was made, and a card that leaves the deck and comes back is dated
by its return.

A card carries what it counts as when something sorts a deck by rarity, whether
it can be played at all, whether it burns up if it is still in hand when the
turn ends, and what it does to whoever is still holding it then. A card that
acts at the end of the turn leaves the hand *before* it acts: the turn end puts
it in the play pile, runs its body from there, and then puts it down — burning
it if it is ethereal and discarding it otherwise, whether or not its owner
keeps the rest of the hand. Both of those moves are announced, so such a card
changes piles twice, and the hand its own body sees is the hand without it.

What a card costs is worked out on demand rather than stored: the form it is in
has a base cost, so an upgrade can make it cheaper; a temporary cost written on
the instance overrides that, and clears when its turn or its combat is over or
when the card is played; then a pass over the listeners runs, which is how a
power that makes its owner's attacks dearer or its skills free answers for
every card at once rather than editing each of them; and last a delta a body
has added for the rest of the turn stands over the answer, which is how a card
gets cheaper for every attack played while it is held. The whole of it is
written back onto the instance at every boundary and wherever a body sets a
cost, because the recording carries the answer and not the question.

A cost of X is not a cost at all: an energy cost of X is canonically zero
whatever the model declares, so nothing ever refuses the play for cost, and the
play then spends every point of energy its owner has left and writes what it
spent onto the card as the X its body reads back. At no energy such a card is a
play that does nothing and still goes to the discard. Nothing moves that zero,
either: a cost of X is answered with the base before a single modifier is
looked at, so a temporary cost written on the instance and a power that makes
every card of its owner's dearer both leave it standing at nothing.

Energy itself is the same shape one level up. A player's maximum energy is not
stored but worked out, running `ModifyMaxEnergy` over the player's own maximum
on every read, so a power that raises it shows up the moment it
lands rather than at the next turn's reset. The run carries the base; what the
fight holds is the answer, worked out again at every boundary the way a card's
cost is.

A star cost is the second half of what a card costs, and it is worked out the
same way, at the same site, and written back onto the instance beside the energy
one. Three things about it are not energy's, and each is load-bearing. Nothing
is `-1` rather than zero: a card that costs no stars carries the sentinel, and a
cost below zero is answered before the modifier walk, so no listener can give a
star cost to a card that has none. There
is one pass — `ModifyStarCost` — rather than the energy pass's two, and the
only models that answer it are the two that answer the energy one with the same
body, which is why `CardCostRule` says for itself whether it is one of them. And
a card whose star cost is X answers with its owner's whole pool rather than
running the pass at all, so it is never refused for stars and the play spends
everything.

The star pool is not energy one level up. It has no maximum, no pass over a
maximum, and no reset: nothing refills it as a turn opens, so what a fight has
put into it stands until it is spent, and it starts every fight at nothing only
because the whole combat state is thrown away and built fresh. A play spends its
stars before the card reaches the play pile, and each half announces itself —
`AfterEnergySpent` for every play whatever it paid, and `AfterStarsSpent` only
where stars actually came off. What it spent is written onto the card for every
play rather than only for an X-cost one, which is where a body reads it back. A
gain announces itself with `AfterStarsGained`, and the pool floors at nothing.

A play is refused for stars the way it is refused for energy, so a card costing
more stars than its owner holds is not among the legal actions at all. The pool
is a projection field of its own, which is what a recording compares.

The numbers on a card's face are worked out the same way and at the same site.
A card's dynamic vars are walked, and each of them answers with the whole hook
pass over its base rather than with the base: a damage var through the damage
passes in preview mode, a block var through the block passes. So a Strike in a
strengthened hand reads nine, and reads more again while it is aimed at
something Vulnerable — the target-testing listeners are silent for the unaimed
draw, which names no target at all. A card that lands on all or a random one of
them is drawn in multi-creature targeting mode instead: the pass runs against
every hittable enemy off the one unaimed base, and the shared number is drawn
only where all of them come to the same one, falling back to the unaimed pass
where they differ. The base each pass runs over is the copy's own live base
value, which is what a body grows: a Rampage that has been played once draws
fourteen rather than the nine it prints, because its play body ends by adding
its increase to that base. What an enchantment adds is not in the base — the
pass is handed the bare base and the enchantment speaks first inside it. Those
passes run for a card standing in the hand or the play pile, with the run-level
listeners left out everywhere else, so a card waiting in a pile draws its
print. What the boundary republishes is the hand's faces — the unaimed damage
and block, and the damage against each enemy the play can name — and they ride
the observation rather than the state, because they are worked out from it.
Drawing a face moves nothing: both passes answer with a value and nothing else,
and the list of listeners the block pass hands back is thrown away by the
preview rather than walked.

Which var a card has is read off its body rather than declared beside it: the
number is written into the body once, as the first blow it strikes or the first
block it works up, and the walk descends into every run of commands the body
binds — a guarded arm, a loop, the body a draw or a selection binds — to find
it. A status is the one card whose body is the one it runs at the end of a turn
spent in hand, and an unblockable blow on the body's own owner is an HP-loss
var rather than a damage var. Three cards of the pinned build cannot be read
that way; their faces are a listed divergence (see
[`fidelity-divergences.md`](fidelity-divergences.md)).

Three keywords change where a card is rather than what it does. An *innate* card
is moved to the top of the draw pile before the opening hand of the fight is
dealt, one card at a time in pile order, and the opening draw is raised to at
least the number of innate cards and then capped at what a hand can hold, which
is ten. That cap is the draw's in general: `CardPileCmd::Draw` stops at a full
hand, and a listener can refuse a whole draw call before a card moves — but only
a call the turn's own opening hand draw did not make. A *retained* card is not
put down when the hand is flushed at the end of the turn: the flush asks once
for the whole hand — a pyramid or a power keeps all of it — and then asks each
card whether it retains itself this turn, so a Retain card keeps itself out of a
hand that is otherwise put down. An *ethereal* card burns up
there instead. An upgrade can add any of the three, and can take Exhaust or
Ethereal back off, so what a card's keywords are is read off the form it is in.

Whether a card may be played at all is three questions, not one. The first is a
condition the card carries — a Clash asks that every card in the hand is an
attack — and failing it is the card's own logic blocking the play. A pass over
the listeners lets anything else refuse it, and a card in the same hand answers
as
readily as a power its owner carries: an Enthralled makes itself the only card
its holder may choose, and a Normality allows three plays a turn. Both of those
bodies gate on the refusing card standing in the hand, and both let through a
play the player did not choose, because the test lives on the path a chosen play
takes.

Where a played card is put down is settled the moment the card reaches the play
pile — the result-pile pass runs there, ahead of `BeforeCardPlayed` and ahead of
the card's own body — and then the
listeners that changed the answer are walked again for it. So a power a card's
own body applies has no say over where that card goes, and a body counting the
turn's plays does not count the one it is answering for, because the history
entry for that play is written after the pile has been settled. The answer is
settled early but acted on late, and the move is an ordinary `CardPileCmd::Add`:
a card sent back to a hand that is holding ten by the time the body is done
lands in the discard instead, at the position the answer asked for. So a Feral
returning a free attack to the top of a full hand puts it on top of the discard.

A card play announces itself once per iteration rather than once per play. How
many iterations a play makes is settled before the loop opens and
`BeforeCardPlayed` runs inside it, so a play doubled by an Echo Form is paid
for twice by whatever answers that hook, and the history entry for a started
play is written per iteration too — which is what a body counting its owner's
first plays of the turn reads.

"Every card of the fight" is the piles concatenated in one fixed order: hand,
draw, discard, exhaust, play. Every body that
walks "every card of the fight" walks it in that order, and a body that moves
several cards at once moves them in the order its own caller handed them over
rather than in the order the pile holds them.

Every body that conjures a card out of "the owner's own pool" names the pool of
the character playing, not a colour: a discovery, a power potion, a crossbow, a
vexing puzzlebox, a metamorphosis, and the rest all read the pool of whoever is
walking the run. A pool named outright — the colorless one, most often — is the
exception rather than the rule, so the command surface carries the distinction
as a pool reference rather than as a model id. It matters for more than which
cards come up: a random take shuffles the whole filtered list before it takes
what it was asked for, so a pool one card longer is a different run of dice
from the very first roll.

A card conjured mid-fight is nobody's property: a thief cannot steal it, because
it is not in the deck to take. A copy made of another card mid-fight is not
either — cloning a card marks the copy with what it came from, and the clone
announcement behind it clears the deck version — and that mark is what a card
whose entered-combat listener pays it for the fight's history asks before it
acts, because a copy is owed nothing for what happened before it existed.

The fight's own cards are copies of the deck's, deep-cloned as the combat opens
and thrown away with it, so what a card grows about itself mid-fight lasts the
fight. That is a card's plain fields — the hand a Regret was held in, the
damage a Rampage has grown to — and nothing projects them. What outlives the
fight is a card's `saved_properties`: the values a card model declares as
saved, which belong to the card rather than to the combat and which the
recording projects alongside the model ID and the upgrade level. They are read
off the live model and are saved unconditionally, so a card of a model that
declares one carries it from the moment it is made rather than from the first
body that writes it: a fresh genetic algorithm is written down with a block of
one and a growth of none. The engine has one factory that makes a card out of a
model, and the places a card is handed over — joining the deck, made straight
into a pile, transformed where it stands, or laid out on a screen — write the
declared fields onto whatever reaches them from content or from a fixture
instead. A body names its own card whether it is standing in the deck or in a
pile, so a curse counting the fights a run has seen counts them on the deck
card and asks first that it is the deck's card rather than the fight's copy.

A body can also *switch* on its own saved property. A Mad Science is whatever a
tinker's table wrote onto it: the type it acts as and the rider hung off that
are two saved properties, written down as the ordinals the game gives them, and
the card's body is a run of guarded branches over them.

A card's own *type* can be one of those properties. A definition that says so
names the property, and everything that asks a copy what it is asks the copy
rather than the model: what its play is aimed at, whether an auto-play rolls a
target for it at all, which pile the play puts it down in — a power leaves the
state where an attack and a skill are put down — whether an enchantment that
wants a type of card may go on it, and every filter, count and listener
condition that reads a type. Two more of the model's properties follow the same
answer, because the game computes them from it: its target type, which is its
owner unless the type is an attack, and whether it gains block, which is the
type being a skill. A property holding none of the three playable ordinals is no
type at all, which the game treats as an error wherever it is read; here it
reads as the type the card was made with, which is what an untinkered card does
nothing as.

What rarity a card carries is the pool it stands in, for the pools whose cards
are rolled by rarity: a character's own and the colorless one. Every other
family is one rarity — status, curse, token, quest, event — and the ancients are
scattered through the character pools and the event pool alike, so each of those
cards declares its own.

A pool can be registered without the cards in it being playable. A pool entry is
metadata — what the card is, what it costs, what it counts as — and that is
everything a generator, a reward offer, and a rarity filter read off a card
nobody plays, which is what lets a run be offered another character's colour.
A card's rarity is asked of its pool where it has no body to ask, and playing
one whose body is not transcribed stops the run by name.

The curse pool is a pool of its own, and what a body drawing a curse out of it
sees is not the whole of it: a card declares whether a modifier may generate it,
and eight of the eighteen curses answer no. What is left is ordered by model ID
— not by the pool's own order — and drawn on the run's niche stream without
replacement, so two curses charged at once are never the same curse.

### A fight that is over

Killing the last enemy ends the fight where it stands, but what a card already
set off still runs: the listeners on it fire and it finishes reading its own
dice. What is blocked is the commands those listeners would issue, so a relic
that counts attacks still counts one and simply does not pay out.

The two questions a body asks about that are worked out fresh rather than kept
as flags: a fight is *in progress* from the moment it is announced —
after every creature has joined, before the first before-combat listener — until
the first line of its ending; and it is *ending* while it is in progress and
either a loss is waiting to be processed or nothing primary is left standing
that a listener is holding it open for. So a command that opens with the ending
test refuses from inside the very death that emptied the enemy side: a relic
paying an energy for every enemy that falls pays nothing for the last one. And a
listener guarded on the fight being in progress does not answer the victory heal
a starter relic lands after the fight has ended, which is the difference between
a relic that reads its owner's health during a fight and one that reads it
between fights.

Which listeners fire at all is a property of the dispatch site rather than of
the fight. Walking the combat's listener list yields nothing once the fight is
over or ending, so almost every combat-scope pass reaches nobody from that
point on. Four sites read the combat's listener list directly instead, because
they fire from the very thing that ended the fight and have to finish resolving
it — `AfterCardPlayed` completes the play that caused the kill,
`AfterBlockBroken` fires from the killing hit, `AfterDamageGiven` answers for
the blow itself, and a creature arriving while a fight is ending can decide
whether it really ends. Run-scope passes are not guarded at all, because they
walk the run's list. So the guard is per dispatch site, and a counter that
ticks on the killing blow ticks.

Whether the fight is over is asked at the points the game asks it — after a
settled action's queue, and at each of the checks along the turn handover — and
the answer is the same question every time: nothing primary is left standing on
the enemy side, a minion not counting. A card auto-played inside another play is
the one path with no queue entry of its own to be asked about: it is played
whole, right there, so a kill inside one is reported back out through the
command run to the action that started it. A fight ended that way finishes where
every other fight finishes, on the action's own settle. What is owed by the
frames the fight interrupted is dropped rather than run out.

What the end of a fight does not touch is the turn handover itself. Ending a
combat puts every player's phase back to none and lets their actions be taken
again, and does not clear the flags that say a player's turn is ending. Whether
they are still up when the dust settles is decided by how the chain that set
them comes apart, and a win and a loss come apart differently.

A won fight comes apart the ordinary way. Every check along the chain simply
stops where it stands once the fight is no longer in progress, so the switch
into the enemy side stops and clears
`ending_player_turn_phase_two` on the way out; phase one is cleared the same
way, and a fight won by an end-of-turn listener never queued the enemy turn at
all. A lost fight does not come apart at all: the game-over screen goes up from
inside the very kill that ended it, everything paused is abandoned, and the
flags stay exactly as they stood — which is why the projection of a defeat can
show a player still standing in the middle of a turn ending.

The flag is racing the recorder's snapshot in the game itself, so a defeat can
be recorded either way. This engine takes the settled reading; a recording
that caught the flag up is a listed divergence (see
[`fidelity-divergences.md`](fidelity-divergences.md)).

A blow settles its own dead. Killing what the blow killed is the last thing
`CreatureCmd::Damage` does, so a listener body that kills with a plain damage
command
inside a turn-handover pass buries what it killed there and then, rather than
leaving the corpses standing until whatever the call happened to be nested
inside gets around to a cleanup. A blow that stopped to ask a question settles
them where it is picked back up.

### The run ending

A run ends when the player goes down, and where that happens is not where the
combat notices it. The death is recorded as owed and the run is lost at the
next boundary rebuild, which is the only place a terminal can be reached:
whatever the killing body still had queued is dropped, the player's powers go
with the fight, the combat is dropped, and the state settles on a terminal
result with no decision behind it. That ordering matters because the
recording's last observation is of the ended run, and it shows a player with
nothing on them.

It is deliberately not the combat's own defeat check: a player can also go down
outside a fight — paying HP to an event, or to a card played at a merchant —
and the same owed-loss path answers for both. The run's terminal is the
simulator's answer to `run.finish`, never something a replay forces.

Victory is reached when combat against the last required boss finishes. The
combat's end and victory hooks, health changes, reward bookkeeping, and owed
turn completion settle before the terminal result is exposed. On Double Boss,
the first boss remains a normal completed room; only the second ends the run.
Earlier-act bosses still lead to the next act.

The terminal state retains the settled final combat and its room, player state,
map history, and RNG counters for comparison. No further action is legal.
The Architect is outside the simulator: there is no event, encounter, creature,
dialogue, departure action, or presentation RNG consumption after victory.

New recordings close with `run.finish VICTORY` and a final observation at this
same gameplay boundary. For older recordings/scripts, replay compares the last
combat observation before the post-victory departure, then excludes the
presentation tail and reports `post_victory_records`. Trace structure and
terminal results are still validated; gameplay actions after victory fail.
Presentation observations never repair state or conceal an earlier divergence.

## Projection rendering and observation matching

The replay crate owns `TraceProfile` resolution, compatibility checks for both
recordings and scripts, and `ReplayVerification` evidence and outcome reporting.
`validate_compatibility` returns the resolved profile; consumers pass it to
`ReplayAdapter::for_profile` instead of inspecting producer headers themselves.
The simulator CLI and AlphaSpire use the same implementation. AlphaSpire retains
its policy evaluation, search, and generated decisions. Scripts run without
authoritative observation comparison and report action consistency only.

The replay crate renders engine state into the recorded Projection v1 shape and
compares the two, so a replay can be checked against every recorded observation
rather than only against the actions it accepts. The renderer emits only what
the engine owns; the comparison walks the rendered value, and a recorded field
the renderer never emits is reported as uncovered rather than silently passing.
Non-integer numbers are compared at single precision, which is the width the
recording carries them at.

A relic renders its saved properties: every value its model declares as saved,
bucketed by type and filled independently, so a relic that keeps both a count
and a boolean writes both. Saved properties are saved unconditionally, so a
declared boolean is written from the moment the relic is obtained and reads
`false` until something sets it — which is why the relics whose once-a-fight
latch is a saved property are the ones that carry a name for it in the
projection, and the ones whose latch is private to the relic carry none.

The list of choices a projection carries is not every screen that is up. What it
names are the screens whose options are a set laid out for them — a card reward,
the merchant's removal, an event's grid, a bundle, a generated-card pick — each
of which carries an offered payload the projection could not be read off
anything else. Every recorded answer to a screen over a combat pile or the run
deck carries no such payload, and each of those stands in an observation whose
list is empty: the piles and the deck already name those cards. The merchant's
removal is the one deck screen in the first group, because what it lays out is a
purchase.

A single action is observed several times as it settles, so the adapter buffers
observations and checks the last one before the next initiating record: the
settled state. Observations recorded before the first applied action describe a
run being built, which is a state the simulator never holds. So do the snapshots
the recorder takes while a turn is being handed back — the ones whose context is
`combat.turn.end` or `combat.enemy_turn.start` — and those are passed over
rather than compared, because the handover is one indivisible step here and
there is no state of ours standing halfway through it. A fight's entrance is
passed over the same way: the recorder writes a `room.enter` observation of the
combat before its first turn has started and then the turn start, with no input
between them unless the player acted during the entrance itself — a potion
drunk during a boss's scene — and the game takes such an input and holds it
until the turn is standing, so its effect lands after the hand is drawn.
Entering the room and opening the fight are one step here, and the entrance
snapshot is a state the simulator never holds; a `room.enter` observation of
any other room is the settled state and is compared. How many of each a
recording holds is counted, so a change that starts skipping more is visible. On
a difference, replay stops with a state-divergence error carrying the first
differing field, its expected and simulated values, and the last matching
checkpoint.

On resume, the adapter restores its save point and applies the recorded
`combat_runtime.ending_player_turn_phase_two` input. A true flag on an active
restored combat remains set through its first player turn until another handover
finishes; false takes the ordinary reload path and clears any flag retained by a
previous reload. The last pre-load turn checkpoint does not select this behavior:
a restarted game process and a retained process can have different flags after
identical recorded actions. Missing input is unsupported, malformed input is an
error, and a retained flag without an active restored combat is unsupported.
Subsequent observations still compare the flag normally.

A replay reports, for the recording it ran, how many observations it compared,
the exact set of recorded fields the renderer does not emit, and how the run
ended. What goes uncovered is metrics, mechanics no registered content uses, and
keys whose recorded value is null. A recording this baseline cannot replay to
its end is reported by the record it stops at and the model it names, which
makes an unfinished recording a measurement rather than an absence. A recording
built on an unsupported content profile (a first run or a different epoch set),
or in an unsupported mode, is refused. Other encounter and act-discovery
histories are taken directly from the recording's generation inputs; a
registered preset match is not required. Standard mode applies boss discovery
after room rolls and before the second-boss draw; custom mode skips it.

Because a recording is authoritative for exactly one build of a game that is
still changing, conformance runs outside the test suite: a curated collection of
the pinned build's recordings is replayed with the CLI. The suite itself pins
behaviour with scenario tests, which state the rule they check rather than
depending on a captured trace.

## Transition and diagnostics

A successful `Transition` contains the accepted action, the next decision or
terminal result, compact ordered semantic events, optional diagnostic RNG
counters before/after, and an optional state/projection digest. Semantic events
are diagnostics and consumer signals; replay must not feed them back as extra
decisions.

The `Transition` is diagnosis, not state: `step_quietly` takes the same
legality gate, application, and boundary rebuild, and returns nothing — no
checkpoints, no per-step state key, no RNG bookends, and no rollback clone.
A refused action still moves nothing (the gate runs before any mutation);
an application that escapes mid-way poisons the simulator until `restore`
puts a whole state back, which is the trade a search client wants: it
discards or restores an errored world anyway, and the rollback clone is
most of what a diagnosed step costs. It exists for search and
self-play clients that step orders of magnitude more often than they
diagnose; the states it lands on are identical to `step`'s, and a
conformance replay keeps the chronicled path.

The internal exact-state key covers all future-relevant hidden and visible
state, including RNG and live queue state. The external projection digest covers
only the selected Projection v1 scope. An observation/belief key is a third,
information-set-specific contract and cannot be substituted for either.

First-divergence diagnostics report fixture/build identity, record sequence,
the initiating action, last matching checkpoint, expected/actual digests,
JSON-Pointer field differences, per-stream RNG counter deltas, queue/choice
context, and compatibility warnings. Normal conformance stops; an explicitly
labeled diagnostic resynchronization mode may continue but forfeits a lockstep
claim.

## Replay ownership

The replay adapter owns record-role interpretation above the simulator:

- initiating and `choice.*` records call `step`;
- resolve and lifecycle records validate transitions already produced;
- `state.observe` projects and compares without mutation; and
- `run.finish` validates output and never forces termination.

Multiple observations can refer to one settled input. The adapter remembers
that settlement and never applies the input twice. Portable import is allowed
only at run start or an explicitly selected settled checkpoint and is labeled
continuation conformance.

## State invariants

Every ordinary decision boundary must satisfy at least:

- stable instance IDs are unique in their domain and all references resolve;
- each card instance has exactly one owner/location and cannot occupy two piles;
- creature targets are live and belong to the current combat unless a pinned
  rule explicitly permits otherwise;
- choice and reward IDs are unique and correlate with their initiating action;
- ordered gameplay collections do not depend on unordered iteration;
- RNG counters and state agree with every consumed value; and
- the engine is quiescent—an eligible automatic effect is never exposed as an
  ordinary player decision.

The information classification and compatibility/error rules in
[`fidelity-contract.md`](fidelity-contract.md) are part of this API.

### Public selection effects

Observation 10 carries producer-authored choice semantics on active and deferred
selections and public macro opportunities. The descriptor travels with suspended
state and canonical decision keys independently of execution commands. See the
[public choice contract](choice-semantics.md) for its vocabulary and coverage checks.
