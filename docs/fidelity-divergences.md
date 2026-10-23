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

## Victory ends gameplay at the final boss

**The build.** Final-boss combat settles before the Architect presentation and
the game's later victory-history bookkeeping.

**This simulator.** Victory is terminal after the last required boss's combat
finishes, including end-of-combat effects. Double Boss requires both bosses.
The simulator retains the final combat state and has no Architect content.

**Whether it can reach a conformance claim.** Conformance ends at the settled
final combat. New recorder versions finalize there. Replay of older traces
checks the final combat before excluding their post-victory presentation and
reports how many records were excluded. Presentation state and RNG are outside
the claim; earlier gameplay and the final combat remain checked.

**What would change this.** A future supported build with gameplay after this
boss requires a revised gameplay boundary before that content is supported.

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

## Reward decisions are final before map navigation

**The build.** Proceeding past terminal combat rewards opens the map without
leaving the room. The player can close the map and collect remaining rewards;
the reward synchronizer skips them only when leaving the room. Reopening an
already open map changes no gameplay state. These rules are in
`RunManager.ProceedFromTerminalRewardsScreen`, `NMapScreen.Open`, and
`RewardsSetSynchronizer.BeforeLeavingRoom` in the pinned decomp.

**This simulator.** The agent resolves rewards, proceeds once, and chooses its
next destination. Map navigation offers neither reward claims nor Proceed.
Dismissed rewards remain only as replay-observation bookkeeping until the next
room is entered; they cannot change gameplay. This avoids unlimited map-open
decisions and reward backtracking. A player can make the same reward selections
before proceeding, without closing and reopening UI.

**Whether it can reach a conformance claim.** Yes. Replay consumes a redundant
`room.proceed` only on the map after terminal combat, without changing state or
RNG and with observation verification still enabled. A recorded `reward.claim`
for an unclaimed dismissed reward instead fails explicitly as unsupported; it
is never ignored or reported as conformant. The scenario tests
`replay_reopening_the_map_preserves_state_rng_and_observation_checks` and
`replay_cannot_collect_rewards_after_proceeding` exercise these two sequences
without embedding a recording. Proceeds that finish a room, resume a parent
event, or lead to an act transition retain their gameplay behavior.

**What would change this.** Supporting human recordings that backtrack for
rewards would require translation at the replay boundary, preserving reward
effects, ordering, and RNG. It does not require adding UI navigation decisions
to the agent's action space.

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

## A death listener's screen outside the player's own action is refused

**The build.** A death listener that asks the player something is put on the
player's action queue as an action of its own and nothing waits for it. The
build's executor runs it once the action ahead of it has finished. Inside the
player's own turn that is the card, potion, or answer that caused the death.
During the turn handover, the enemy's turn, or a turn opening, the turn flow is
not itself an action on that queue, and the executor picks the queued action up
beside it, wherever the turn flow next stops to wait.

**This simulator.** A death listener's screen asked for while the player is
acting is queued and shown behind the action, as the build shows it. One asked
for anywhere else fails as unsupported content naming the discipline rather
than as an invariant, and a full step puts the state back. A Gremlin Horn whose
draw shuffles into a Stratagem as an enemy dies on its own turn, or during the
end of the player's turn, is the registered content that reaches it.

**Whether it can reach a conformance claim.** Only through a death listener that
asks outside the player's own action. A recording of one stops at the ask with
an unsupported-content error and is never replayed past it. A second, narrower
shape is ordering alone. A player who queues a card or a potion while the
killing action is still animating has that input run ahead of the death
listener's queued action, while this simulator, which takes one action at a
time, shows the screen first. A recording of that would replay the input after
the answer rather than before it.

**What would change this.** A recording of a death listener asking during the
enemy's turn or the handover, which would pin where the build's executor runs
the queued action against the turn flow.

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

## A potion asked for beside a screen inside a fight is offered after it

**The build.** Inside a fight every selection screen belongs to the card,
potion, or listener action that opened it, which waits at the front of the
player's action queue for the answer. The belt is still in reach, and a potion
drunk or discarded meanwhile is accepted onto the queue behind that action. It
runs once the action has finished, or is cancelled if the fight ends first.

**This simulator.** The belt is not offered beside a screen inside a fight. It
is offered again at the next decision, once the action that opened the screen
has finished.

**Whether it can reach a conformance claim.** Through action order alone, and
only in one shape. The recorder writes a potion when it starts to run, so a
trace shows the potion after the answer and after the opening action settled,
exactly where this simulator offers it, and a request the fight's end
cancelled is never written. What a trace could show and this simulator would
not reproduce is a potion requested beside the screen running ahead of a
listener's own queued screen that the answer set going. That listener would be
put on the queue behind the potion, while here it settles before the next
decision offers the belt.

**What would change this.** A recording in which a potion runs between an
in-combat answer and a listener screen that answer queued. Reproducing it would
need a potion request that waits on the queue beside the screen rather than an
action offered at a decision.

## A run abandoned inside a fight is refused

**The build.** Abandoning a run kills every player with a forced kill that no
death prevention answers. Inside a fight that kill also loses the combat, and
every listener the fight has for a death or a lost fight runs.

**This simulator.** A `run.finish ABANDONED` outside a fight takes the player's
HP to nothing, which is the whole of what the build's kill changes there. One
inside a fight, the fight's end screen included, fails as unsupported content.

**Whether it can reach a conformance claim.** Only through a recording
abandoned inside a fight, which stops at its `run.finish` with an
unsupported-content error. Everything it recorded before the abandonment is
still compared.

**What would change this.** A recording abandoned inside a fight, which would
pin which of the fight's death and loss listeners the forced kill runs and what
its final observation shows.

## Bone Shards whose sweep fells the pet give no block

**The build.** Bone Shards asks once, as the play begins, whether the pet is
standing. With no pet it does nothing at all. With one it runs the sweep, the
block, and the pet's death behind that one answer, so the block lands even
where the sweep itself cost the pet its life.

**This simulator.** The sweep, the block, and the kill each ask whether the pet
is standing as they run, so that a sweep that wins the fight leaves the block
and the kill to the guards they carry over a won fight. A sweep that fells the
pet, a retaliation for the pet's own blow say, takes the block with it.

**Whether it can reach a conformance claim.** Only through a Bone Shards sweep
that leaves its own pet dead, which would diverge on the player's block at the
play's settled observation.

**What would change this.** A recording of that play, or a body nested under a
test that runs the rest of itself over a won fight with each command's own
guard, as a top-level card body does.

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
