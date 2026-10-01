use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde_json::Value;
use sts2_core::ModelId;
use sts2_engine::{
    Action, CardFingerprint, ErrorCode, MapPointType, RewardFingerprint, Simulator, Snapshot,
    Transition,
};

use crate::{Record, TimelineRole, TimelineValidator, TraceProfile, VerificationLevel};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterErrorCode {
    MalformedRecord,
    IllegalAction,
    UnsupportedContent,
    /// The simulation and the recording disagree about state.
    StateDivergence,
}

#[derive(Debug)]
pub struct AdapterError {
    pub code: AdapterErrorCode,
    pub record_sequence: u64,
    pub record_kind: String,
    pub detail: String,
    /// Present on a state divergence: what differed and where.
    pub divergence: Option<Box<crate::FirstDivergence>>,
}

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} at record {} ({}): {}",
            self.code, self.record_sequence, self.record_kind, self.detail
        )
    }
}

impl Error for AdapterError {}

#[derive(Clone, Debug)]
pub enum AdapterOutcome {
    Applied(Box<Transition>),
    Ignored(TimelineRole),
}

/// A recorder checkpoint inside a transition the simulator settles in one step.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ObservationStage {
    #[default]
    Settled,
    Handover,
    FightEntrance,
    ParentExit,
}

/// Drives a simulator from decoded STS2PGN action records. Timeline-only records are
/// validated and ignored; unsupported initiating records fail at their sequence.
#[derive(Clone, Debug)]
pub struct ReplayAdapter {
    simulator: Simulator,
    timeline: TimelineValidator,
    verify_observations: bool,
    profile: Option<TraceProfile>,
    last_matching_checkpoint: Option<u64>,
    compared_observations: usize,
    applied_actions: usize,
    pending_observation: Option<(u64, Value)>,
    uncovered: BTreeSet<String>,
    dropped_room_entries: usize,
    mid_handover_observations: usize,
    pending_stage: ObservationStage,
    fight_entrance_observations: usize,
    /// The proceed that popped a combat room and resumed its parent event.
    parent_resume_action: Option<u64>,
    parent_exit_record: Option<u64>,
    cleared_event_pages: usize,
    unwritten_reward_blocks: usize,
    /// The state the build last wrote to its save file, which is where a
    /// `run.resume` puts the run back.
    save_point: Option<Snapshot>,
    /// Actions applied since the save point was written, which a reload
    /// would discard.
    actions_since_save: usize,
    resumes: usize,
    rewound_actions: usize,
}

impl ReplayAdapter {
    #[must_use]
    pub fn new(simulator: Simulator) -> Self {
        // The build saves the run the moment character selection finishes, so
        // an unplayed run already has a save to come back to.
        let save_point = Some(simulator.snapshot());
        Self {
            simulator,
            timeline: TimelineValidator::default(),
            verify_observations: false,
            profile: None,
            last_matching_checkpoint: None,
            compared_observations: 0,
            applied_actions: 0,
            pending_observation: None,
            uncovered: BTreeSet::new(),
            dropped_room_entries: 0,
            mid_handover_observations: 0,
            pending_stage: ObservationStage::Settled,
            fight_entrance_observations: 0,
            parent_resume_action: None,
            parent_exit_record: None,
            cleared_event_pages: 0,
            unwritten_reward_blocks: 0,
            save_point,
            actions_since_save: 0,
            resumes: 0,
            rewound_actions: 0,
        }
    }

    /// Select verification from validated trace identity, shared by every consumer.
    #[must_use]
    pub fn for_profile(simulator: Simulator, profile: TraceProfile) -> Self {
        let mut adapter = Self::new(simulator);
        adapter.profile = Some(profile);
        adapter.verify_observations =
            profile.verification_level() == VerificationLevel::RecordedState;
        adapter
    }

    /// How many times the recording picked the run back up off its save file.
    #[must_use]
    pub const fn resumes(&self) -> usize {
        self.resumes
    }

    /// How many applied actions were rewound by those reloads: the ones taken
    /// between the build's last save and the quit, which the resumed timeline
    /// never saw.
    #[must_use]
    pub const fn rewound_actions(&self) -> usize {
        self.rewound_actions
    }

    /// How many actions have been applied since the build last wrote its save
    /// file: what a reload right now would rewind. Zero just after a step
    /// means that step saved.
    #[must_use]
    pub const fn actions_since_save(&self) -> usize {
        self.actions_since_save
    }

    /// Checks every recorded observation against the simulated state as replay
    /// runs, stopping at the first difference.
    #[must_use]
    pub const fn verifying_observations(mut self) -> Self {
        self.verify_observations = true;
        self
    }

    /// How many observations have been compared so far.
    #[must_use]
    pub const fn compared_observations(&self) -> usize {
        self.compared_observations
    }

    /// The sequence of the last observation that matched, if any.
    #[must_use]
    pub const fn last_matching_checkpoint(&self) -> Option<u64> {
        self.last_matching_checkpoint
    }

    /// How many settled observations were taken between two rooms. Leaving a
    /// room and entering the next, or resuming its parent, are one step here.
    /// Such an observation is a state this engine never holds; it is neither
    /// compared nor counted as compared, but counted here so that a recording
    /// stopping short stays visible.
    #[must_use]
    pub const fn dropped_room_entries(&self) -> usize {
        self.dropped_room_entries
    }

    /// How many settled observations were taken while an ancient's page stood
    /// cleared: entering the room takes the option down before the line
    /// plays, and it only goes back up once the speech has finished
    /// animating. There is no animation here, so the state this
    /// engine settles on is the page with its option on it, and the recorded
    /// snapshot of the gap is passed over rather than compared.
    #[must_use]
    pub const fn cleared_event_pages(&self) -> usize {
        self.cleared_event_pages
    }

    /// How many settled observations were taken partway along the turn
    /// handover, because the player acted again before the recorder's queue
    /// had drained. `EndTurn` is one decision here and settles whole, so a
    /// stage of it is a state this engine never holds.
    #[must_use]
    pub const fn mid_handover_observations(&self) -> usize {
        self.mid_handover_observations
    }

    /// How many settled observations were a fight's entrance, because the
    /// player acted during it. The build takes the input and holds it until
    /// the first turn is standing; entering the room and opening the fight
    /// are one step here, so the entrance is a state this engine never holds.
    #[must_use]
    pub const fn fight_entrance_observations(&self) -> usize {
        self.fight_entrance_observations
    }

    /// How many reward fingerprints were compared with their save-derived block
    /// passed over, because the recording carries none of it.
    ///
    /// That block — the pools, the width, the odds, the source, the gold — is
    /// read off the offer's save-derived description and written whole or not
    /// at all. Written as nothing, it is not a description of the offer but a
    /// record of not having one, and holding a rendered block against it
    /// compares a value to an absence; so the block is compared only where
    /// the recording carries it.
    #[must_use]
    pub const fn unwritten_reward_blocks(&self) -> usize {
        self.unwritten_reward_blocks
    }

    /// How many observations named an action earlier than one already named:
    /// the recorder writing an observation late.
    #[must_use]
    pub const fn late_observations(&self) -> usize {
        self.timeline.late_observations()
    }

    /// How many recorded actions have been applied to the simulator so far.
    #[must_use]
    pub const fn applied_actions(&self) -> usize {
        self.applied_actions
    }

    /// Every recorded field the rendering layer does not produce, and so does
    /// not check, across the observations compared so far.
    #[must_use]
    pub const fn uncovered_fields(&self) -> &BTreeSet<String> {
        &self.uncovered
    }

    #[must_use]
    pub const fn simulator(&self) -> &Simulator {
        &self.simulator
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per record kind the format carries"
    )]
    pub fn accept(&mut self, record: &Record) -> Result<AdapterOutcome, AdapterError> {
        if let Some(profile) = self.profile {
            profile.validate_record(record).map_err(|error| {
                Self::error(record, AdapterErrorCode::MalformedRecord, error.to_string())
            })?;
        }
        let role = self.timeline.accept(record).map_err(|error| {
            Self::error(record, AdapterErrorCode::MalformedRecord, error.to_string())
        })?;
        if self.verify_observations {
            self.settle_observations(record)?;
        }
        let action = match record.kind.as_str() {
            "event.choose" => Some(self.event_choice(record)?),
            "map.choose" => Some(self.map_choice(record)?),
            "combat.play_card" => Some(self.play_card(record)?),
            "combat.end_turn" => {
                Some(self.find_action(record, |action| matches!(action, Action::EndTurn { .. }))?)
            }
            "choice.index" => Some(self.choice_index(record)?),
            "choice.cards" => Some(self.choice_cards(record)?),
            // A purchase the game made for nothing on a lord's parasol's
            // behalf is the engine's own next owed step rather than a choice
            // off the legal list, and so is the removal the parasol forces.
            "shop.purchase" => Some(
                record
                    .payload
                    .pointer("/data/ignore_cost")
                    .and_then(Value::as_bool)
                    .filter(|ignored| *ignored)
                    .and_then(|_| {
                        let index = integer(record, "/data/inventory_index").ok()? as usize;
                        self.simulator.automatic_purchase(index)
                    })
                    .map_or_else(|| self.shop_purchase(record), Ok)?,
            ),
            "shop.remove_card" => Some(
                record
                    .payload
                    .pointer("/data/cancelable")
                    .and_then(Value::as_bool)
                    .filter(|cancelable| !*cancelable)
                    .and_then(|_| self.simulator.automatic_removal())
                    .map_or_else(
                        || {
                            self.find_action(record, |action| {
                                matches!(action, Action::BuyCardRemoval)
                            })
                        },
                        Ok,
                    )?,
            ),
            "treasure.choose_relic" => Some(self.take_treasure(record)?),
            "combat.use_potion" => Some(self.use_potion(record)?),
            // The belt being made room in. It names the slot and what stood in
            // it, and nothing the potion does runs.
            "combat.discard_potion" => {
                let slot = usize::try_from(
                    record
                        .payload
                        .pointer("/data/potion_slot")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| {
                            Self::error(
                                record,
                                AdapterErrorCode::MalformedRecord,
                                "a discard names the slot",
                            )
                        })?,
                )
                .unwrap_or(usize::MAX);
                Some(self.find_action(record, |action| {
                    matches!(action, Action::DiscardPotion { slot: at, .. } if *at == slot)
                })?)
            }
            "rest.choose" => Some(self.rest_choice(record)?),
            "event.crystal_sphere.click" => Some(self.uncover_crystal_sphere(record)?),
            "act.next" => {
                Some(self.find_action(record, |action| matches!(action, Action::AdvanceAct))?)
            }

            "reward.claim" => Some(self.reward_claim(record)?),
            // The player dismissing a rewards screen and leaving what is on it.
            // A screen with nothing left to answer has already gone by the time
            // the build sends this, and then there is nothing to dismiss.
            "reward.skip" => self
                .simulator
                .legal_actions()
                .iter()
                .find(|action| matches!(action, Action::Proceed))
                .cloned(),
            "room.proceed" => self.room_proceed(record)?,
            // Records with no decision behind them.
            //
            // `event.advance` is an ancient's dialogue being clicked through.
            // The page whose options the player will answer is already standing
            // — the dialogue set a room entry rolled is localization
            // lines over it — so advancing one changes nothing a projection
            // carries, and every recorded observation on either side of one is
            // identical. It is still a record the player caused, so the
            // observation before it is settled and compared like any other
            // input's.
            //
            // The caller constructs the simulator from the validated
            // `run.start`; accepting it keeps replay streaming from record 1
            // without applying initialization twice.
            //
            // `event.crystal_sphere.start` is the fog board being stood up,
            // which the option that stood it up already did. `set_tool` only
            // changes which of the screen's two tool buttons is selected; the
            // following click names its tool again, and the simulator exposes
            // both tool variants as legal actions. Finally,
            // `event.crystal_sphere.proceed` is the board's own proceed button:
            // leaving a rewards screen the player has already dismissed, which
            // is nothing at all.
            "event.advance"
            | "run.start"
            | "state.observe"
            | "reward.resolve"
            | "reward.settled"
            | "event.crystal_sphere.start"
            | "event.crystal_sphere.set_tool"
            | "event.crystal_sphere.proceed" => None,
            // The run picked back up off its save file. It is not an action —
            // nobody chose it — but it puts the run back where the build last
            // saved it and leaves marks the projection carries from there on.
            "run.resume" => {
                self.resume(record)?;
                None
            }
            _ if matches!(
                role,
                TimelineRole::Observation
                    | TimelineRole::Resolve
                    | TimelineRole::Lifecycle
                    | TimelineRole::Finish
            ) =>
            {
                None
            }
            _ => {
                return Err(Self::error(
                    record,
                    AdapterErrorCode::UnsupportedContent,
                    "record kind is outside the implemented replay slice",
                ));
            }
        };
        let Some(action) = action else {
            return Ok(AdapterOutcome::Ignored(role));
        };
        self.applied_actions += 1;
        let before = self.save_marks();
        let resumes_parent = matches!(action, Action::Proceed)
            && self.simulator.state().run.as_ref().is_some_and(|run| {
                run.pushed_room
                    .as_ref()
                    .is_some_and(|room| room.resumes_parent)
            });
        self.simulator
            .step(action.clone())
            .map(|transition| {
                self.parent_exit_record = None;
                self.parent_resume_action = (resumes_parent
                    && self
                        .simulator
                        .state()
                        .run
                        .as_ref()
                        .is_some_and(|run| run.pushed_room.is_none()))
                .then_some(record.sequence);
                if self.build_saved(&action, before) {
                    self.save_point = Some(self.simulator.snapshot());
                    self.actions_since_save = 0;
                } else {
                    self.actions_since_save += 1;
                }
                AdapterOutcome::Applied(Box::new(transition))
            })
            .map_err(|error| {
                let code = if error.code == ErrorCode::UnsupportedContent {
                    AdapterErrorCode::UnsupportedContent
                } else {
                    AdapterErrorCode::IllegalAction
                };
                let detail = error
                    .details
                    .get("model_id")
                    .and_then(Value::as_str)
                    .map_or_else(
                        || error.to_string(),
                        |model| format!("unsupported model {model}"),
                    );
                Self::error(record, code, detail)
            })
    }

    /// `run.resume`: the run put back where the build last saved it.
    ///
    /// The recorder appends the resumed session to the same trace, after the
    /// records of the session that quit. The build saves no mid-room state, so
    /// the load rewinds the run to its last save and everything played after
    /// that save really happened but is gone from the resumed timeline. The
    /// same happens here: the last settled observation before the quit is
    /// checked against the state it describes, then that state is dropped for
    /// the save point, and the load's own marks are applied on top of it.
    fn resume(&mut self, record: &Record) -> Result<(), AdapterError> {
        // The last thing the quit session wrote was a stage of a turn
        // handover: the run was quit with the enemy's turn under way.
        let quit_inside_a_handover = self.pending_stage == ObservationStage::Handover;
        if self.verify_observations {
            self.pass_over_stage_observation();
            self.verify_pending_observation()?;
        }
        let Some(save_point) = self.save_point.as_ref() else {
            return Err(Self::error(
                record,
                AdapterErrorCode::IllegalAction,
                "there is no save to resume from",
            ));
        };
        self.simulator.restore(save_point).map_err(|error| {
            Self::error(record, AdapterErrorCode::IllegalAction, error.to_string())
        })?;
        self.rewound_actions += self.actions_since_save;
        self.actions_since_save = 0;
        self.resumes += 1;
        let resumed = if quit_inside_a_handover {
            self.simulator.resumed_from_save_inside_a_handover()
        } else {
            self.simulator.resumed_from_save()
        };
        resumed.map_err(|error| {
            Self::error(record, AdapterErrorCode::IllegalAction, error.to_string())
        })?;
        // Resuming raises the run's reload count and writes the save back
        // out with it, so a second reload lands on this state, not the first.
        self.save_point = Some(self.simulator.snapshot());
        Ok(())
    }

    /// What the save rule needs to know about the state before a step, so that
    /// a fight or an ancient that was already over is not saved again.
    fn save_marks(&self) -> SaveMarks {
        let state = self.simulator.state();
        SaveMarks {
            in_fight: state
                .combat
                .as_ref()
                .is_some_and(|combat| !combat.has_ended),
            ancient_finished: ancient_finished(state),
        }
    }

    /// Whether the build wrote its save file somewhere along the step just
    /// taken. The build saves on every map point the player walks onto —
    /// before it rolls the room, and rolling it again off the same counters
    /// lands on the same room, so the settled state after the walk is the
    /// state a load comes back to. It saves again the moment a fight is won,
    /// with the room marked pre-finished, and once more when an act's ancient
    /// is finished with. A run that ended has no save to come back to.
    fn build_saved(&self, action: &Action, before: SaveMarks) -> bool {
        let state = self.simulator.state();
        if state.terminal.is_some() {
            return false;
        }
        if matches!(action, Action::ChooseMap { .. } | Action::AdvanceAct) {
            return true;
        }
        let fight_over = state
            .combat
            .as_ref()
            .map_or(before.in_fight, |combat| combat.has_ended);
        let combat_won = before.in_fight && fight_over;
        let ancient_finished = !before.ancient_finished && ancient_finished(state);
        combat_won || ancient_finished
    }

    /// Drops a pending observation that was taken partway along a step this
    /// engine takes whole — a turn handover stage or a fight's entrance — so
    /// that it is not compared against a state the build never settled on.
    fn pass_over_stage_observation(&mut self) {
        // The handover was still running when the player acted again, so what
        // the recorder last wrote is a stage of it rather than the state it
        // came to rest in. There is no such intermediate state here — `EndTurn`
        // is one decision and settles whole — so the snapshot is passed over.
        if self.pending_stage == ObservationStage::Handover {
            self.pending_observation = None;
            self.pending_stage = ObservationStage::Settled;
            self.mid_handover_observations += 1;
        }
        if self.pending_stage == ObservationStage::FightEntrance {
            self.pending_observation = None;
            self.pending_stage = ObservationStage::Settled;
            self.fight_entrance_observations += 1;
        }
    }

    fn observation_stage(&self, record: &Record, state: &Value) -> ObservationStage {
        let context = record.payload.pointer("/context").and_then(Value::as_str);
        // A pushed combat exits before Resume pays its parent's rewards and
        // finishes its page. The observation can name either the proceed or
        // its nested room.exit lifecycle record. Later observations compare
        // the complete deck, rewards, and event choices as usual.
        if context == Some("room.exit")
            && self.parent_resume_action.is_some_and(|action| {
                record
                    .payload
                    .pointer("/after")
                    .and_then(Value::as_u64)
                    .is_some_and(|after| after == action || Some(after) == self.parent_exit_record)
            })
        {
            ObservationStage::ParentExit
        } else if matches!(context, Some("combat.turn.end" | "combat.enemy_turn.start")) {
            // EndTurn settles the whole handover here, while the recorder
            // can still be writing one of its stages when the player acts.
            ObservationStage::Handover
        } else if context == Some("room.enter")
            && state.get("combat").is_some_and(|combat| !combat.is_null())
        {
            // Input taken during an entrance lands after the turn starts.
            ObservationStage::FightEntrance
        } else {
            ObservationStage::Settled
        }
    }

    /// Keeps the settled-observation bookkeeping as records stream past: the
    /// last observation before an input is the state this engine can hold, and
    /// an input is where the one before it is checked.
    fn settle_observations(&mut self, record: &Record) -> Result<(), AdapterError> {
        if record.kind == "room.exit"
            && self.parent_resume_action.is_some()
            && self.parent_resume_action == self.timeline.last_input()
            && record
                .payload
                .pointer("/data/stack_depth")
                .and_then(Value::as_u64)
                .is_some_and(|depth| depth > 1)
        {
            self.parent_exit_record = Some(record.sequence);
        }
        // A single action is observed several times as it settles. The last
        // observation before the next record is the settled state, which is
        // the one this engine can hold.
        if record.kind == "state.observe" {
            if let Some(state) = record.payload.pointer("/state") {
                self.pending_observation = Some((record.sequence, state.clone()));
                self.pending_stage = self.observation_stage(record, state);
            }
            return Ok(());
        }
        if !matches!(
            crate::record_role(&record.kind),
            TimelineRole::InitiatingAction | TimelineRole::NestedChoice
        ) {
            return Ok(());
        }
        // The rewards screen being dismissed is written down after the state it
        // produced. The offer is over the moment the set is marked skipped,
        // and everything waiting behind it — the event finishing among them —
        // happens from there, while the recorder writes the input record
        // afterwards. So the observations in front of
        // one already show the set gone, and settling them against the state
        // before the skip would compare a state the build was never in.
        if record.kind == "reward.skip" {
            return Ok(());
        }
        self.pass_over_stage_observation();
        // An event page with nothing on it, right before a choice that names
        // one of its options, is the recorder catching an ancient mid-line:
        // entering the room clears the page's options, and they only go back
        // up once the speech has animated. The state this engine settles on is
        // the one after that, so the snapshot taken during it is passed over.
        if record.kind == "event.choose"
            && record
                .payload
                .pointer("/data/option_id")
                .is_some_and(Value::is_string)
            && self.pending_observation.as_ref().is_some_and(|(_, state)| {
                state
                    .pointer("/choices/events/0/options")
                    .is_some_and(|options| options.as_array().is_some_and(Vec::is_empty))
            })
        {
            self.pending_observation = None;
            self.cleared_event_pages += 1;
        }
        // Lifecycle records sit inside a single step here, so only the
        // observation right before the next input describes a state this engine
        // settles on. A run's finish is not such a boundary: it is recorded
        // while the killing action is still settling, so the observation before
        // it is mid-settle and the one after it is the terminal state.
        self.verify_pending_observation()
    }

    fn event_choice(&self, record: &Record) -> Result<Action, AdapterError> {
        let option_id = string(record, "/data/option_id")?;
        // An event can lay out two options with the same id, so the place on
        // the page is part of which one was chosen.
        let index = usize::try_from(integer(record, "/data/option_index")?).unwrap_or(usize::MAX);
        self.find_action(record, |action| {
            matches!(action, Action::ChooseEvent { index: at, option_id: legal }
                if *at == index && legal == option_id)
        })
    }

    /// One divination spent, named by the cell it was aimed at and the tool it
    /// was spent with.
    fn uncover_crystal_sphere(&self, record: &Record) -> Result<Action, AdapterError> {
        let x = i32::try_from(integer(record, "/data/x")?).unwrap_or(-1);
        let y = i32::try_from(integer(record, "/data/y")?).unwrap_or(-1);
        let tool = match string(record, "/data/tool")? {
            "small" => sts2_engine::DivinationTool::Small,
            _ => sts2_engine::DivinationTool::Big,
        };
        self.find_action(record, |action| {
            matches!(action, Action::UncoverCrystalSphere { x: at_x, y: at_y, tool: with }
                if *at_x == x && *at_y == y && *with == tool)
        })
    }

    fn map_choice(&self, record: &Record) -> Result<Action, AdapterError> {
        let col = coordinate(record, "/data/col")?;
        let row = coordinate(record, "/data/row")?;
        self.find_action(record, |action| {
            matches!(action, Action::ChooseMap { destination }
                if destination.col == col && destination.row == row)
        })
    }

    fn play_card(&self, record: &Record) -> Result<Action, AdapterError> {
        let card_id = integer(record, "/data/combat_card_index")?;
        let card_model = model_id(record, "/data/card_model")?;
        let recorded_card = value(record, "/data/card")?;
        let target_id = record
            .payload
            .pointer("/data/target_id")
            .and_then(Value::as_u64);
        let target_model = optional_model_id(record, "/data/target/model_id")?;
        self.find_action(record, |action| {
            matches!(action, Action::PlayCard { card, target }
                if card.card_id.get() == card_id
                    && card.fingerprint.model_id == card_model
                    && card_fingerprint_matches(&card.fingerprint, recorded_card)
                    && target.as_ref().map(|target| &target.model_id) == target_model.as_ref()
                    && target.as_ref().map(|target| target.combat_id.get()) == target_id)
        })
    }

    fn choice_index(&self, record: &Record) -> Result<Action, AdapterError> {
        let recorded_choice_id = integer(record, "/data/choice_id")?;
        let indexes: Vec<usize> = record
            .payload
            .pointer("/data/indexes")
            .and_then(Value::as_array)
            .map(|indexes| {
                indexes
                    .iter()
                    .filter_map(|index| index.as_u64().map(|index| index as usize))
                    .collect()
            })
            .unwrap_or_default();
        // A grid lays several cards out at once and takes more than one.
        if let Some(options) = record.payload.pointer("/data/offered/options/options") {
            return self.grid_choice(record, recorded_choice_id, &indexes, options);
        }
        let index = *indexes.first().ok_or_else(|| {
            Self::error(
                record,
                AdapterErrorCode::MalformedRecord,
                "missing data.indexes[0]",
            )
        })?;
        let recorded_card = record
            .payload
            .pointer("/data/offered/options/cards")
            .and_then(Value::as_array)
            .and_then(|cards| {
                cards
                    .iter()
                    .find(|card| card.get("index").and_then(Value::as_u64) == Some(index as u64))
            })
            .and_then(|card| card.get("fingerprint"));
        // An index that names an alternative rather than a card is the skip
        // option, which resolves the choice with nothing taken.
        let Some(recorded_card) = recorded_card else {
            let alternative = record
                .payload
                .pointer("/data/offered/options/alternatives")
                .and_then(Value::as_array)
                .and_then(|alternatives| {
                    alternatives
                        .iter()
                        .find(|alternative| {
                            alternative.get("index").and_then(Value::as_u64) == Some(index as u64)
                        })
                        .and_then(|alternative| alternative.get("option_id"))
                        .and_then(Value::as_str)
                });
            let Some(alternative) = alternative else {
                return Err(Self::error(
                    record,
                    AdapterErrorCode::MalformedRecord,
                    "selected index has no offered card fingerprint",
                ));
            };
            if alternative == "skip" {
                return self.find_action(record, |action| {
                    matches!(action, Action::ChooseCards { choice_id, cards }
                        if choice_id.get() == recorded_choice_id && cards.is_empty())
                });
            }
            return self.find_action(record, |action| {
                matches!(action, Action::ChooseAlternative { choice_id, option_id }
                    if choice_id.get() == recorded_choice_id && option_id == alternative)
            });
        };
        self.find_action(record, |action| {
            matches!(action, Action::ChooseCards { choice_id, cards }
                if choice_id.get() == recorded_choice_id
                    && cards.len() == 1 && cards[0].index == index
                    && card_fingerprint_matches(&cards[0].fingerprint, recorded_card))
        })
    }

    /// A choice made off a grid of laid-out cards, by index.
    /// A screen whose options are whole bundles of cards: the index names a
    /// bundle, and taking it takes every card in it, in the order the bundle
    /// holds them.
    fn bundle_choice(
        &self,
        record: &Record,
        choice_id: u64,
        indexes: &[usize],
        options: &[Value],
    ) -> Result<Action, AdapterError> {
        let mut chosen: Vec<&Value> = Vec::new();
        for index in indexes {
            let bundle = options
                .iter()
                .find(|option| option.get("index").and_then(Value::as_u64) == Some(*index as u64))
                .and_then(|option| option.get("cards"))
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    Self::error(
                        record,
                        AdapterErrorCode::MalformedRecord,
                        "a chosen index names no offered bundle",
                    )
                })?;
            chosen.extend(bundle);
        }
        self.find_action(record, |action| {
            matches!(action, Action::ChooseCards { choice_id: id, cards }
            if id.get() == choice_id
                && cards.len() == chosen.len()
                && cards.iter().zip(&chosen).all(|(card, expected)| {
                    card_fingerprint_matches(&card.fingerprint, expected)
                }))
        })
    }

    fn grid_choice(
        &self,
        record: &Record,
        choice_id: u64,
        indexes: &[usize],
        options: &Value,
    ) -> Result<Action, AdapterError> {
        let offered: Vec<&Value> = options
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .collect();
        // A bundle screen lays out groups rather than single cards.
        if offered.iter().any(|option| option.get("cards").is_some()) {
            let bundles: Vec<Value> = offered.into_iter().cloned().collect();
            return self.bundle_choice(record, choice_id, indexes, &bundles);
        }
        let chosen: Vec<&Value> = indexes
            .iter()
            .filter_map(|index| {
                offered
                    .iter()
                    .find(|option| {
                        option.get("index").and_then(Value::as_u64) == Some(*index as u64)
                    })
                    .and_then(|option| option.get("fingerprint"))
            })
            .collect();
        if chosen.len() != indexes.len() {
            return Err(Self::error(
                record,
                AdapterErrorCode::MalformedRecord,
                "a chosen index has no offered card fingerprint",
            ));
        }
        // Large grids expose individual clicks instead of enumerating every
        // combination. Reconstruct the recorded selection from the standing
        // candidates and let the engine validate its counts and distinctness.
        let illegal = || {
            Self::error(
                record,
                AdapterErrorCode::IllegalAction,
                "record does not match the standing card selection",
            )
        };
        let sts2_engine::DecisionContext::ChooseCards {
            choice_id: standing,
            candidates,
            ..
        } = self.simulator.decision()
        else {
            return Err(illegal());
        };
        if standing.get() != choice_id {
            return Err(illegal());
        }
        let cards = indexes
            .iter()
            .zip(chosen)
            .map(|(index, expected)| {
                candidates
                    .iter()
                    .find(|card| {
                        card.index == *index
                            && card_fingerprint_matches(&card.fingerprint, expected)
                    })
                    .cloned()
                    .ok_or_else(illegal)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Action::ChooseCards {
            choice_id: *standing,
            cards,
        })
    }

    /// A card choice names every card it took, in the order the screen sent
    /// them: a screen that asks for two writes both down in one record.
    fn choice_cards(&self, record: &Record) -> Result<Action, AdapterError> {
        let choice_type = string(record, "/data/choice_type")?;
        // An empty list is an answer: a cancelable screen backed out of names
        // no cards, and the build sends the choice all the same.
        let chosen = record
            .payload
            .pointer("/data/cards")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                Self::error(
                    record,
                    AdapterErrorCode::MalformedRecord,
                    "missing data.cards",
                )
            })?;
        let mut fingerprints = Vec::with_capacity(chosen.len());
        for card in chosen {
            fingerprints.push(card.get("fingerprint").ok_or_else(|| {
                Self::error(
                    record,
                    AdapterErrorCode::MalformedRecord,
                    "chosen card has no fingerprint",
                )
            })?);
        }
        let key = |name: &str| -> Result<Vec<u64>, AdapterError> {
            chosen
                .iter()
                .map(|card| {
                    card.get(name).and_then(Value::as_u64).ok_or_else(|| {
                        Self::error(
                            record,
                            AdapterErrorCode::MalformedRecord,
                            format!("chosen card has no {name}"),
                        )
                    })
                })
                .collect()
        };
        match choice_type {
            "deck_card" => {
                let indexes = key("deck_index")?;
                let ordered = self.find_action(record, |action| {
                    matches!(action, Action::ChooseCards { cards, .. }
                    if cards.len() == indexes.len()
                        && cards.iter().zip(&indexes).all(|(card, index)| {
                            card.index as u64 == *index
                        })
                        && cards.iter().zip(&fingerprints).all(|(card, fingerprint)| {
                            card_fingerprint_matches(&card.fingerprint, fingerprint)
                        }))
                });
                if ordered.is_ok() {
                    return ordered;
                }
                // A screen with more candidates than the engine can lay out in
                // every order offers each set once, in candidate order. So a
                // recorded answer that named those same cards in another order
                // is that set: the cap the engine draws on its action list is
                // showing through, and nothing past the screen reads the order
                // a deck screen this size was answered in.
                let mut wanted = indexes.clone();
                wanted.sort_unstable();
                self.find_action(record, |action| {
                    matches!(action, Action::ChooseCards { cards, .. }
                    if cards.len() == wanted.len()
                        && {
                            let mut named: Vec<u64> =
                                cards.iter().map(|card| card.index as u64).collect();
                            named.sort_unstable();
                            named == wanted
                        })
                })
            }
            "combat_card" => {
                let card_ids = key("combat_card_index")?;
                let ordered = self.find_action(record, |action| {
                    matches!(action, Action::ChooseCards { cards, .. }
                    if cards.len() == card_ids.len()
                        && cards.iter().zip(&card_ids).all(|(card, card_id)| {
                            card.card_id.get() == *card_id
                        })
                        && cards.iter().zip(&fingerprints).all(|(card, fingerprint)| {
                            card_fingerprint_matches(&card.fingerprint, fingerprint)
                        }))
                });
                if ordered.is_ok() {
                    return ordered;
                }
                self.combat_cards_in_recorded_order(record, &card_ids, &fingerprints)
            }
            _ => Err(Self::error(
                record,
                AdapterErrorCode::UnsupportedContent,
                format!("card choice type {choice_type} is outside the implemented replay slice"),
            )),
        }
    }

    /// A screen with more orders than the engine lays out offers each set once,
    /// in candidate order, and takes any order of a set it offers. The order a
    /// combat screen was answered in is read by what follows it, so the answer
    /// is that set put back in the order the record names.
    fn combat_cards_in_recorded_order(
        &self,
        record: &Record,
        card_ids: &[u64],
        fingerprints: &[&Value],
    ) -> Result<Action, AdapterError> {
        let mut wanted = card_ids.to_vec();
        wanted.sort_unstable();
        let set = self.find_action(record, |action| {
            matches!(action, Action::ChooseCards { cards, .. }
            if cards.len() == wanted.len()
                && {
                    let mut named: Vec<u64> =
                        cards.iter().map(|card| card.card_id.get()).collect();
                    named.sort_unstable();
                    named == wanted
                })
        })?;
        let Action::ChooseCards { choice_id, cards } = set else {
            return Ok(set);
        };
        let mut in_order = Vec::with_capacity(cards.len());
        for (card_id, fingerprint) in card_ids.iter().zip(fingerprints) {
            let card = cards
                .iter()
                .find(|card| card.card_id.get() == *card_id)
                .filter(|card| card_fingerprint_matches(&card.fingerprint, fingerprint))
                .ok_or_else(|| {
                    Self::error(
                        record,
                        AdapterErrorCode::IllegalAction,
                        "record does not match any legal action",
                    )
                })?;
            in_order.push(card.clone());
        }
        Ok(Action::ChooseCards {
            choice_id,
            cards: in_order,
        })
    }

    fn shop_purchase(&self, record: &Record) -> Result<Action, AdapterError> {
        let index = integer(record, "/data/inventory_index")? as usize;
        let model = model_id(record, "/data/model_id")?;
        self.find_action(record, |action| {
            matches!(action, Action::BuyShopItem { offer_index, fingerprint }
                if *offer_index == index && shop_item_model(fingerprint) == &model)
        })
    }

    /// `rest.choose` names the option by its option id, which is what the
    /// generated list carries: the index it stands at is the engine's own and
    /// is not recorded, so the option is what a record is matched on.
    fn rest_choice(&self, record: &Record) -> Result<Action, AdapterError> {
        let recorded = string(record, "/data/option_id")?;
        self.find_action(record, |action| {
            matches!(action, Action::RestOption { option, .. }
                if option.option_id() == recorded)
        })
    }

    fn use_potion(&self, record: &Record) -> Result<Action, AdapterError> {
        let slot = integer(record, "/data/potion_slot")? as usize;
        let model = model_id(record, "/data/potion_model")?;
        let target_id = record
            .payload
            .pointer("/data/target/combat_id")
            .and_then(Value::as_u64);
        self.find_action(record, |action| {
            matches!(action, Action::UsePotion { slot: legal, model_id, target }
                if *legal == slot
                    && *model_id == model
                    && target.as_ref().map(|target| target.combat_id.get()) == target_id)
        })
    }

    fn take_treasure(&self, record: &Record) -> Result<Action, AdapterError> {
        let model = model_id(record, "/data/relic_model")?;
        self.find_action(
            record,
            |action| matches!(action, Action::TakeTreasure { relic } if *relic == model),
        )
    }

    fn room_proceed(&self, record: &Record) -> Result<Option<Action>, AdapterError> {
        // After terminal combat rewards, the room's button only reopens the
        // map. Consume that UI record without adding a simulation decision.
        // A parent event that still needs resuming is not a UI-only proceed.
        if matches!(
            self.simulator.decision(),
            sts2_engine::DecisionContext::MapNavigation { .. }
        ) && self.simulator.state().run.as_ref().is_some_and(|run| {
            run.pushed_room
                .as_ref()
                .is_none_or(|room| !room.resumes_parent)
                && (matches!(
                    run.current_room_type,
                    Some(MapPointType::Monster | MapPointType::Elite)
                ) || run.pushed_room.is_some())
        }) {
            return Ok(None);
        }
        self.find_action(record, |action| matches!(action, Action::Proceed))
            .map(Some)
    }

    fn reward_claim(&self, record: &Record) -> Result<Action, AdapterError> {
        let recorded_set_id = integer(record, "/data/set_id")?;
        let index = integer(record, "/data/reward_index")? as usize;
        let recorded_reward = value(record, "/data/reward")?;
        if matches!(
            self.simulator.decision(),
            sts2_engine::DecisionContext::MapNavigation { .. }
        ) && self
            .simulator
            .state()
            .run
            .as_ref()
            .and_then(|run| run.dismissed_reward_set.as_ref())
            .is_some_and(|set| {
                set.id.get() == recorded_set_id
                    && set.rewards.iter().any(|reward| {
                        reward.index == index
                            && !reward.selected
                            && !reward.passed_over
                            && reward_fingerprint_matches(&reward.fingerprint, recorded_reward)
                    })
            })
        {
            return Err(Self::error(
                record,
                AdapterErrorCode::UnsupportedContent,
                "collecting rewards after proceeding to the map is unsupported; reward decisions are final at Proceed",
            ));
        }
        self.find_action(record, |action| {
            matches!(action, Action::ClaimReward { set_id, reward_index, fingerprint }
                if set_id.get() == recorded_set_id && *reward_index == index
                    && reward_fingerprint_matches(fingerprint, recorded_reward))
        })
    }

    fn find_action(
        &self,
        record: &Record,
        predicate: impl Fn(&Action) -> bool,
    ) -> Result<Action, AdapterError> {
        self.simulator
            .legal_actions()
            .iter()
            .find(|action| predicate(action))
            .cloned()
            .ok_or_else(|| {
                Self::error(
                    record,
                    AdapterErrorCode::IllegalAction,
                    "record does not match any legal action",
                )
            })
    }

    /// Compares the settled observation, if one is waiting, against the state.
    ///
    /// Observations recorded before the first applied action describe the run
    /// being built, which is a state the caller's simulator never holds: it is
    /// constructed at the first decision.
    ///
    /// `accept` calls this at every record that closes a settled state, but a
    /// recording ends with an observation and no record after it. A caller
    /// replaying a whole file should call this once the stream is exhausted so
    /// that the terminal state is compared too.
    pub fn verify_pending_observation(&mut self) -> Result<(), AdapterError> {
        let Some((sequence, expected)) = self.pending_observation.take() else {
            return Ok(());
        };
        if self.pending_stage == ObservationStage::ParentExit {
            self.pending_stage = ObservationStage::Settled;
            self.dropped_room_entries += 1;
            return Ok(());
        }
        if self.applied_actions == 0 {
            return Ok(());
        }
        let record = Record {
            sequence,
            kind: "state.observe".to_owned(),
            payload: Value::Null,
        };
        let expected = &expected;
        let rendered = crate::observation::render(&self.simulator);
        // A recorded state standing in no room at all, while this engine stands
        // in one, is a snapshot taken between two rooms: the recorder writes
        // `room.exit` and its observation as the walk begins, and the room
        // ahead is entered inside the same step here. Such a moment is one this
        // engine passes through rather than settles on, so it is passed over.
        // Both sides standing roomless is a state the engine does hold — an act
        // boundary, or the map — and is compared as usual.
        if stands_in_no_room(expected) && !stands_in_no_room(&rendered) {
            self.dropped_room_entries += 1;
            return Ok(());
        }
        self.uncovered
            .extend(crate::observation::uncovered(expected, &rendered));
        let mut differences = crate::observation::compare(expected, &rendered);
        let before = differences.len();
        differences.retain(|difference| !unwritten_reward_block(expected, difference));
        self.unwritten_reward_blocks += before - differences.len();
        if differences.is_empty() {
            self.compared_observations += 1;
            self.last_matching_checkpoint = Some(sequence);
            return Ok(());
        }
        let first = differences[0].clone();
        let mut error = Self::error(
            &record,
            AdapterErrorCode::StateDivergence,
            format!(
                "{} expected {} but simulated {}",
                first.pointer,
                first
                    .expected
                    .as_ref()
                    .map_or("nothing".to_owned(), ToString::to_string),
                first
                    .actual
                    .as_ref()
                    .map_or("nothing".to_owned(), ToString::to_string),
            ),
        );
        error.divergence = Some(Box::new(crate::FirstDivergence {
            fixture_id: String::new(),
            compatibility_id: self.simulator.state().compatibility_id.clone(),
            record_sequence: sequence,
            initiating_action: None,
            last_matching_checkpoint: self.last_matching_checkpoint,
            expected_digest: None,
            actual_digest: None,
            differences,
            rng_counter_deltas: BTreeMap::new(),
            queue_context: None,
            choice_context: None,
            compatibility_warnings: Vec::new(),
        }));
        Err(error)
    }

    fn error(record: &Record, code: AdapterErrorCode, detail: impl Into<String>) -> AdapterError {
        AdapterError {
            code,
            record_sequence: record.sequence,
            record_kind: record.kind.clone(),
            detail: detail.into(),
            divergence: None,
        }
    }
}

/// What the save rule reads off the state before a step.
#[derive(Clone, Copy, Debug)]
struct SaveMarks {
    in_fight: bool,
    ancient_finished: bool,
}

/// Whether the run stands on an act's ancient whose event is finished with:
/// the build marks an event finished, and a finished event is projected as a
/// page with no options left on it.
fn ancient_finished(state: &sts2_engine::GameState) -> bool {
    state.run.as_ref().is_some_and(|run| {
        run.current_room_type == Some(MapPointType::Ancient)
            // Ordinary events share this room type. Only the act's starting
            // point hosts its Ancient and writes a save on event completion.
            && run.current_coord == Some(run.map().start)
            && run
                .begun_event
                .as_ref()
                .is_some_and(|event| event.options.is_empty())
    })
}

/// Whether a rendered state stands on a map point with no room on it.
fn stands_in_no_room(state: &Value) -> bool {
    state
        .pointer("/rooms")
        .is_some_and(|rooms| rooms.as_array().is_some_and(Vec::is_empty))
}

const fn shop_item_model(item: &sts2_engine::ShopItem) -> &ModelId {
    match item {
        sts2_engine::ShopItem::Card(card) => &card.model_id,
        sts2_engine::ShopItem::Potion(model_id) | sts2_engine::ShopItem::Relic(model_id) => {
            model_id
        }
    }
}

fn coordinate(record: &Record, pointer: &str) -> Result<u8, AdapterError> {
    u8::try_from(integer(record, pointer)?).map_err(|_| AdapterError {
        code: AdapterErrorCode::MalformedRecord,
        record_sequence: record.sequence,
        record_kind: record.kind.clone(),
        detail: format!("coordinate at {pointer} is outside the supported map range"),
        divergence: None,
    })
}

fn model_id(record: &Record, pointer: &str) -> Result<ModelId, AdapterError> {
    string(record, pointer)?
        .parse()
        .map_err(|error| AdapterError {
            code: AdapterErrorCode::MalformedRecord,
            record_sequence: record.sequence,
            record_kind: record.kind.clone(),
            detail: format!("invalid model ID at {pointer}: {error}"),
            divergence: None,
        })
}

fn optional_model_id(record: &Record, pointer: &str) -> Result<Option<ModelId>, AdapterError> {
    record
        .payload
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(|_| model_id(record, pointer))
        .transpose()
}

fn value<'a>(record: &'a Record, pointer: &str) -> Result<&'a Value, AdapterError> {
    record.payload.pointer(pointer).ok_or_else(|| AdapterError {
        code: AdapterErrorCode::MalformedRecord,
        record_sequence: record.sequence,
        record_kind: record.kind.clone(),
        detail: format!("missing value at {pointer}"),
        divergence: None,
    })
}

fn card_fingerprint_matches(actual: &CardFingerprint, expected: &Value) -> bool {
    expected.get("model_id").and_then(Value::as_str) == Some(&actual.model_id.to_string())
        && expected.get("upgrade_level").and_then(Value::as_u64)
            == Some(u64::from(actual.upgrade_level))
        && expected.get("floor_added_to_deck").and_then(Value::as_u64)
            == actual.floor_added_to_deck.map(u64::from)
        && enchantment_matches(actual.enchantment.as_ref(), expected.get("enchantment"))
        && saved_properties_match(&actual.properties, expected.get("properties"))
}

/// How a record writes a model's saved properties: one bucket per value type,
/// each a list of `{name, value}`, or nothing where the model declares none of
/// that type. Flattened here so that a fingerprint's own map can be compared
/// against it name by name.
fn saved_properties_match(
    actual: &std::collections::BTreeMap<String, Value>,
    expected: Option<&Value>,
) -> bool {
    let Some(expected) = expected else {
        return actual.is_empty();
    };
    if expected.is_null() {
        return actual.is_empty();
    }
    let Some(buckets) = expected.as_object() else {
        return false;
    };
    let mut recorded = std::collections::BTreeMap::new();
    for entries in buckets.values() {
        let Some(entries) = entries.as_array() else {
            continue;
        };
        for entry in entries {
            let (Some(name), Some(value)) = (
                entry.get("name").and_then(Value::as_str),
                entry.get("value"),
            ) else {
                return false;
            };
            recorded.insert(name.to_owned(), value.clone());
        }
    }
    recorded.len() == actual.len()
        && recorded
            .iter()
            .all(|(name, value)| actual.get(name) == Some(value))
}

/// A card's recorded enchantment: the recorded shape is an object carrying
/// the model, the amount, and the enchantment's own saved properties, so all
/// three are compared. A card with no enchantment is recorded as nothing.
fn enchantment_matches(
    actual: Option<&sts2_engine::EnchantmentFingerprint>,
    expected: Option<&Value>,
) -> bool {
    match (actual, expected) {
        (None, None | Some(Value::Null)) => true,
        (Some(actual), Some(expected)) => {
            expected.get("model_id").and_then(Value::as_str) == Some(&actual.model_id.to_string())
                && expected.get("amount").and_then(Value::as_i64) == Some(i64::from(actual.amount))
                && saved_properties_match(&actual.properties, expected.get("properties"))
        }
        _ => false,
    }
}

/// The half of a reward fingerprint the recorder reads off the offer's
/// save-derived description and writes as one block or not at all.
const SAVE_DERIVED_REWARD_FIELDS: [&str; 6] = [
    "card_pools",
    "gold_amount",
    "option_count",
    "rarity_odds",
    "source",
    "was_gold_stolen_back",
];

/// Whether this difference is a reward field the recording does not describe.
///
/// The block is written whole or not at all, so every one of its fields being
/// nothing is the recorder saying it could not describe the offer rather than
/// saying what the offer was. One field of it standing alone as nothing is a
/// difference like any other, and so is a block the recording carries and this
/// engine does not: the test is only ever made the one way round.
fn unwritten_reward_block(expected: &Value, difference: &crate::FieldDifference) -> bool {
    let Some((set, field)) = difference.pointer.rsplit_once("/fingerprint/") else {
        return false;
    };
    if !SAVE_DERIVED_REWARD_FIELDS.contains(&field) {
        return false;
    }
    let Some(fingerprint) = expected.pointer(&format!("{set}/fingerprint")) else {
        return false;
    };
    reward_block_is_unwritten(fingerprint)
}

/// Whether a recorded fingerprint carries none of the save-derived block.
///
/// This is the recording's own answer, and it has to be: whether the offer
/// could be described at all is not something this engine can work out from
/// the offer it is holding. The same offer of the same build can be written
/// either way, so an offer asked which to expect can only ever be right about
/// one.
fn reward_block_is_unwritten(fingerprint: &Value) -> bool {
    SAVE_DERIVED_REWARD_FIELDS
        .iter()
        .all(|field| fingerprint.get(field).is_none_or(Value::is_null))
}

/// Whether a recorded reward describes this offer.
///
/// The comparison branches on what the recording carries. An offer the
/// recorder cannot describe is written down as its type and the cards standing
/// on the screen, and its whole save-derived block as nothing; held to a gold
/// amount it does not carry, such a record describes no offer at all, so the
/// block is compared only where the recording carries it. The cards still tell
/// apart the offers a screen puts up together: a philosopher's three are one
/// per rarity over disjoint card sets.
fn reward_fingerprint_matches(actual: &RewardFingerprint, expected: &Value) -> bool {
    let cards_match = match expected.get("offered_cards") {
        None | Some(Value::Null) => actual.offered_cards.is_empty(),
        Some(Value::Array(cards)) => {
            cards.len() == actual.offered_cards.len()
                && actual
                    .offered_cards
                    .iter()
                    .zip(cards)
                    .all(|(actual, expected)| card_fingerprint_matches(actual, expected))
        }
        Some(_) => false,
    };
    let type_matches =
        expected.get("reward_type").and_then(Value::as_str) == Some(actual.reward_type.as_str());
    if reward_block_is_unwritten(expected) {
        return type_matches && cards_match;
    }
    let special_matches = match (expected.get("special_card"), actual.special_card.as_ref()) {
        (None | Some(Value::Null), None) => true,
        (Some(expected), Some(actual)) => card_fingerprint_matches(actual, expected),
        _ => false,
    };
    special_matches
        && type_matches
        && expected.get("gold_amount").and_then(Value::as_i64)
            == Some(i64::from(actual.gold_amount))
        && expected.get("model_id").and_then(Value::as_str)
            == actual.model_id.as_ref().map(ToString::to_string).as_deref()
        && cards_match
}

fn string<'a>(record: &'a Record, pointer: &str) -> Result<&'a str, AdapterError> {
    record
        .payload
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| AdapterError {
            code: AdapterErrorCode::MalformedRecord,
            record_sequence: record.sequence,
            record_kind: record.kind.clone(),
            detail: format!("missing string at {pointer}"),
            divergence: None,
        })
}

fn integer(record: &Record, pointer: &str) -> Result<u64, AdapterError> {
    record
        .payload
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| AdapterError {
            code: AdapterErrorCode::MalformedRecord,
            record_sequence: record.sequence,
            record_kind: record.kind.clone(),
            detail: format!("missing integer at {pointer}"),
            divergence: None,
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sts2_core::ModelId;
    use sts2_engine::{
        CardCreation, CardCreationFlags, CardFilter, CardFingerprint, CardPoolRef, CardRarity,
        RarityOdds, RewardFingerprint, RewardSource,
    };

    use serde_json::Value;

    use super::{reward_fingerprint_matches, stands_in_no_room, unwritten_reward_block};
    use crate::observation::render_reward_fingerprint;

    fn card(entry: &str) -> CardFingerprint {
        CardFingerprint {
            model_id: format!("CARD.{entry}").parse().expect("model id"),
            upgrade_level: 0,
            floor_added_to_deck: None,
            enchantment: None,
            properties: std::collections::BTreeMap::new(),
            bing_bong_skip_once: false,
        }
    }

    /// A card offer as a screen puts it up, drawing from one pool under
    /// `filter`. Everything else is what an ordinary combat reward carries.
    fn card_offer(filter: CardFilter, entries: &[&str]) -> RewardFingerprint {
        RewardFingerprint {
            reward_type: "card".to_owned(),
            model_id: None,
            gold_amount: 0,
            offered_cards: entries.iter().copied().map(card).collect(),
            special_card: None,
            card_pools: Vec::new(),
            option_count: entries.len(),
            rarity_odds: Some(RarityOdds::Uniform),
            source: RewardSource::Other,
            custom_description_encounter_source_id: None,
            was_gold_stolen_back: false,
            creation: Some(CardCreation {
                pools: vec![CardPoolRef::Named(
                    "CARD_POOL.NECROBINDER_CARD_POOL"
                        .parse::<ModelId>()
                        .expect("pool id"),
                )],
                filter,
                odds: RarityOdds::Uniform,
                from_an_encounter: false,
                count: entries.len(),
                flags: CardCreationFlags {
                    is_card_reward: true,
                    ..CardCreationFlags::default()
                },
            }),
            can_reroll: false,
            upgrade_after_generation: false,
            relic_is_wax: false,
        }
    }

    /// The invariant the two halves of a reward comparison owe each other: what
    /// [`render_reward_fingerprint`] writes for an offer, `reward_claim` accepts
    /// as that offer. It has to hold on both sides of the save-derived
    /// branch, because the renderer writes the two shapes differently.
    #[test]
    fn a_rendered_offer_matches_the_offer_it_was_rendered_from() {
        let ordinary = card_offer(CardFilter::Any, &["STRIKE_IRONCLAD", "BASH", "ANGER"]);
        assert!(ordinary.has_save_representation());
        assert!(reward_fingerprint_matches(
            &ordinary,
            &render_reward_fingerprint(&ordinary)
        ));

        // A philosopher's offer: the pool is narrowed to one rarity, which is
        // a shape the save-derived description refuses, so the recorder writes
        // the whole block as nothing.
        let philosophers = card_offer(
            CardFilter::OfRarity(CardRarity::Uncommon),
            &["SEVERANCE", "PARSE", "DEATHBRINGER"],
        );
        assert!(!philosophers.has_save_representation());
        let written = render_reward_fingerprint(&philosophers);
        assert!(
            written["gold_amount"].is_null(),
            "an offer the schema refuses carries no gold"
        );
        assert!(reward_fingerprint_matches(&philosophers, &written));
    }

    /// The same offer can be recorded with its save-derived block written or
    /// with the whole of it as nothing, so the claim is matched on what the
    /// recording in hand carries rather than on what this engine expects the
    /// recorder to have managed. An offer whose creation the engine did not
    /// keep looks writable to it either way, and both shapes have to be
    /// readable.
    #[test]
    fn a_claim_is_matched_on_the_block_the_recording_carries() {
        // The Future of Potions, as this engine holds it: the cards were handed
        // to the offer, so nothing about the narrowed pool the build drew them
        // from survives, and the offer reads as one the recorder can
        // describe.
        let mut handed_over = card_offer(CardFilter::Any, &["RICOCHET", "SUCKER_PUNCH", "STOMP"]);
        handed_over.card_pools = Vec::new();
        handed_over.creation = None;
        assert!(handed_over.has_save_representation());

        // The shape that carries none of the block: no description could be
        // got, and the whole of it is nothing.
        let refused = json!({
            "reward_type": "card",
            "card_pools": null,
            "gold_amount": null,
            "option_count": null,
            "rarity_odds": null,
            "source": null,
            "was_gold_stolen_back": null,
            "offered_cards": render_reward_fingerprint(&handed_over)["offered_cards"],
        });
        assert!(reward_fingerprint_matches(&handed_over, &refused));

        // The shape that carries the block, for the same event: it is
        // compared as any other written block is.
        let read = json!({
            "reward_type": "card",
            "card_pools": [],
            "gold_amount": 0,
            "option_count": 3,
            "rarity_odds": "uniform",
            "source": "other",
            "was_gold_stolen_back": false,
            "offered_cards": render_reward_fingerprint(&handed_over)["offered_cards"],
        });
        assert!(reward_fingerprint_matches(&handed_over, &read));

        // Neither reading gives the cards away: a written block still has to
        // agree about the gold, and an unwritten one about the cards.
        let mut wrong_gold = read.clone();
        wrong_gold["gold_amount"] = json!(7);
        assert!(!reward_fingerprint_matches(&handed_over, &wrong_gold));

        let other = card_offer(CardFilter::Any, &["LETHALITY", "CRUSH_UNDER", "PILLAGE"]);
        let mut wrong_cards = refused.clone();
        wrong_cards["offered_cards"] = render_reward_fingerprint(&other)["offered_cards"].clone();
        assert!(!reward_fingerprint_matches(&handed_over, &wrong_cards));
    }

    /// The relaxation is not a surrender: the three offers a philosopher stands
    /// up together are written down by their cards alone, and that is still what
    /// tells them apart.
    #[test]
    fn a_refused_offer_is_still_told_from_its_siblings() {
        let uncommon = card_offer(
            CardFilter::OfRarity(CardRarity::Uncommon),
            &["SEVERANCE", "PARSE", "DEATHBRINGER"],
        );
        let common = card_offer(
            CardFilter::OfRarity(CardRarity::Common),
            &["GRAVE_WARDEN", "POKE", "DEFILE"],
        );
        assert!(!reward_fingerprint_matches(
            &common,
            &render_reward_fingerprint(&uncommon)
        ));
        assert!(!reward_fingerprint_matches(
            &uncommon,
            &render_reward_fingerprint(&common)
        ));
    }

    /// A recorded reward whose save-derived block the recorder could not read,
    /// as it comes out of a recording: the whole block is nothing and the cards
    /// on the screen are all that describes it.
    fn a_recorded_state_with(fingerprint: &Value) -> Value {
        json!({
            "choices": {
                "rewards": [{"set_id": 6, "rewards": [{"index": 0, "fingerprint": fingerprint}]}]
            }
        })
    }

    fn difference(field: &str, expected: Value, actual: Value) -> crate::FieldDifference {
        crate::FieldDifference {
            pointer: format!("/choices/rewards/0/rewards/0/fingerprint/{field}"),
            expected: Some(expected),
            actual: Some(actual),
        }
    }

    /// A block the recording does not carry describes no offer, so nothing of it
    /// is compared. Everything else about the reward still is.
    #[test]
    fn a_block_the_recorder_could_not_read_is_not_compared() {
        let recorded = a_recorded_state_with(&json!({
            "reward_type": "card",
            "card_pools": null,
            "gold_amount": null,
            "option_count": null,
            "rarity_odds": null,
            "source": null,
            "was_gold_stolen_back": null,
            "offered_cards": [{"model_id": "CARD.INFERNO", "upgrade_level": 1}],
        }));
        assert!(unwritten_reward_block(
            &recorded,
            &difference("card_pools", Value::Null, json!([]))
        ));
        assert!(unwritten_reward_block(
            &recorded,
            &difference("option_count", Value::Null, json!(3))
        ));
        assert!(
            !unwritten_reward_block(
                &recorded,
                &difference(
                    "offered_cards/0/model_id",
                    json!("CARD.INFERNO"),
                    json!("CARD.STAMPEDE")
                )
            ),
            "the cards are what the recording does describe"
        );
    }

    /// One field of the block standing alone as nothing is a difference like any
    /// other, and so is a block the recording carries and the rendering does
    /// not: the test is only ever made the one way round.
    #[test]
    fn a_block_the_recorder_did_read_is_compared_whole() {
        let recorded = a_recorded_state_with(&json!({
            "reward_type": "card",
            "card_pools": [],
            "gold_amount": 0,
            "option_count": 3,
            "rarity_odds": "uniform",
            "source": null,
            "was_gold_stolen_back": false,
        }));
        assert!(!unwritten_reward_block(
            &recorded,
            &difference("source", Value::Null, json!("other"))
        ));
        assert!(!unwritten_reward_block(
            &recorded,
            &difference("option_count", json!(3), json!(2))
        ));
    }

    /// The state a recorder writes between two rooms: the room just left is off
    /// the stack and the one ahead is not on it yet. Whether the record after it
    /// stands in a room decides nothing — a belt slot can be emptied here, and a
    /// recording can stop here altogether — so the snapshot is recognised by
    /// what it holds rather than by what follows it.
    #[test]
    fn a_state_between_rooms_stands_in_none() {
        assert!(stands_in_no_room(&json!({"rooms": []})));
        assert!(!stands_in_no_room(
            &json!({"rooms": [{"model": "EVENT.NEOW"}]})
        ));
    }

    /// A rendering that carries no rooms at all is not a state standing between
    /// two of them, and is compared like any other.
    #[test]
    fn a_state_without_the_field_is_not_between_rooms() {
        assert!(!stands_in_no_room(&json!({})));
        assert!(!stands_in_no_room(&json!({"rooms": null})));
    }
}
