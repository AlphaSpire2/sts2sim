# Known fidelity divergences

Places where this simulator does not reproduce the pinned build, and what each
one costs.

[`fidelity-contract.md`](fidelity-contract.md) states what parity means. This
document is the standing list of exceptions to it. Two kinds of entry qualify:
a divergence taken **deliberately**, whose safety is **argued from evidence**
rather than assumed; and a build behaviour this simulator does not model,
named together with the recording or playback that would catch it. What does
not qualify is a safety claim nobody has evidence for — that is a bug waiting
for the recording that finds it, and it belongs in the code, not here.

Every entry answers four questions: what the build does, what this simulator
does instead, whether the difference can reach a conformance claim, and what
would change that.

## The shop's action list omits what the player cannot afford

**The build.** A shop item the player cannot afford is clickable. The click is
accepted, the shop answers that there is not enough gold, the item stays on
the shelf, and the screen plays a rejection. Nothing about the run changes.

**This simulator.** `legal_actions()` omits the offer entirely, and applying
one anyway is refused as an illegal action.

**Why it cannot reach a conformance claim.** The recorder never emits an action
for such a click. It records a `shop.purchase` only for an item that is
stocked and that the player can pay for, so an unaffordable click produces no
record and no trace can contain one. A divergence no trace can express cannot
fail a replay.

The simulator is also stricter, not looser: it refuses an input the build
tolerates. A recording the build produced still replays; only an input the
build would have shrugged off is rejected, and nothing produces that input.

**What would change this.** If the recorder ever captured failed purchase
attempts, this becomes a replay hazard immediately and the filter has to go.
Anything that consumes `legal_actions()` as *"what a player may click"* rather
than *"what changes the run"* is also reading it wrongly here, and should read
the shop offers off the decision context instead, which carries every offer
unfiltered.

## Driftwood and Pael's Wing together are a card reward the build cannot show

**The build.** A card reward carries buttons besides its cards: Skip, when the
reward may be skipped; Reroll, when something has granted a reroll; and
whatever the player's relics add. The build supports at most two such buttons,
and Skip counts toward that limit — skipping is allowed by default, so Skip is
always the first button rather than a property of the screen. Driftwood grants
a reroll on every card reward of a set, and Pael's Wing adds a Sacrifice
button. Holding both makes three buttons, and the build assembles them before
the screen is shown, so claiming any card reward with both relics crashes the
game.

**This simulator.** The skip is the screen being cancelable rather than a
button, so `alternatives` holds the reroll and the sacrifice and nothing counts
them. `claim_reward` builds both buttons, `compute_decision_actions` emits a
`ChooseAlternative` for each alongside the empty answer, and all three are
enumerated and applied.

**Whether it can reach a conformance claim.** Not through a recording: the
build crashes where the screen would open, so no trace carries the claim, only
everything before it. It reaches playback instead. A decision script authored
here can walk a player holding both relics into a card reward, and the
playback driver has no click that survives that room.

**What would change this.** Modelling Skip as one of the buttons and refusing
the third the way the build does, which turns the state into an engine error
rather than a decision. Nothing forces it while both relics are Ancient and
drawn a pair at a time; anything that makes Ancients common, or hands one out
as an event reward, makes the state ordinary.

## A screen opened before a side's turn starts is refused

**The build.** Six listener sites let each listener pause on its own, carry on
to the next with the screen still owed, and join the paused bodies later. At
five of them the join is the end of the site. At `BeforeSideTurnStart` the
build parks a paused body with nothing ever waiting on it, so a screen a body
opens there stands until the player answers it and the turn start runs on
regardless.

**This simulator.** The pass is dispatched, but a body that opens a screen in
it fails as unsupported content naming the discipline rather than as an
invariant. The other five sites are carried in full.

**Whether it can reach a conformance claim.** Only through a body that asks at
that site. A recording of one stops at the ask with an unsupported-content
error and is never replayed past it, so the divergence is loud rather than
silent; nothing the pass runs for any other body differs.

**What would change this.** A body that asks there, which would need the
parked-and-unwaited discipline modelled: a screen banked by the pass and
answered while the turn start has already run on.

## Three card faces are drawn from the wrong variable

**The build.** A card's face is drawn from the variables its body declares.
Pillar of Creation and Shroud declare a block variable and spend it as a
power's amount rather than as a gain of block, and their faces draw three and
two. Entrench declares no variable at all and doubles the block standing, and
its face draws no number.

**This simulator.** A face is read off the body: a block variable spent as a
power's amount draws no block, so Pillar of Creation and Shroud draw none, and
Entrench draws the doubled block standing where the build draws nothing.

**Whether it can reach a conformance claim.** Not through a recording: a
recording carries no face values, so no replay compares one. The face is part
of what the observation encodes for a card, so a consumer of the observation
sees the deviation on those three cards and nowhere else.

**What would change this.** Reading those three faces off a per-card override
rather than off the body, which nothing that replays needs.

## A defeat's turn-ending flag can be recorded before it settles

**The build.** A lost fight raises the game-over screen from inside the very
kill that ended it, and the flags of the turn that was ending stay exactly as
they stood. `ending_player_turn_phase_two` is cleared by the handover that set
it, and the recorder's snapshot of the defeat races that clear: the same loss
at the same point can be recorded with the flag down or still up.

**This simulator.** The settled reading, with the flag down: a defeat is
projected as a player no longer in the middle of a turn ending.

**Whether it can reach a conformance claim.** Yes, on the terminal observation
of a defeat whose snapshot caught the flag up. Such a recording is a documented
mid-settle stop: its final observation is a scheduling artefact of the recorder
rather than a state the build settles on, and it is not replayed past it.

**What would change this.** Nothing in the model. The flag's value in that
observation is recorder timing, not run state, and a recorder that snapshots
after the settle records the reading this simulator gives.

## Not on this list

**A potion offer the belt has no room for is not a divergence.** The build
accepts that click and answers that there is no room, and so does this
simulator: `legal_actions()` carries the offer and `buy_shop_item_at` answers
it with an accepted no-op — nothing paid, the offer still on the shelf, the
player free to make room and click again.

It could not be kept as one, because the build checks the belt only after it
has checked the gold, and the recorder records the purchase as soon as the
gold check passes. A trace *can* contain an affordable purchase that failed
for want of a slot, so refusing that action would be a live replay hazard
rather than an inert asymmetry.

The distinction is the whole point of this document: the shelf answers both
clicks the same way, and only where the recorder writes its record separates
the one that may be filtered out of the action list from the one that may not.
