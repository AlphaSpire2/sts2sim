//! Public meaning of a card selection, independent of its execution queue.

use serde::{Deserialize, Serialize};
use sts2_core::ModelId;

use crate::{CardKeyword, ChoiceIdentity};

/// Whether the public selection effect is fully described.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceCoverage {
    Known,
    Partial,
    #[default]
    Unknown,
}

/// Which lifetime's card is changed, independently of candidate identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceScope {
    Combat,
    Run,
}

/// The object an operation acts upon. Results are bound by operation order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceSubject {
    #[default]
    Selected,
    Copy,
    Replacement,
    Player,
}

/// Public destination; no entry contains a hidden draw position.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceDestination {
    #[default]
    NotApplicable,
    Unknown,
    Hand,
    Discard,
    Exhaust,
    DrawTop,
    DrawBottom,
    DrawShuffled,
    RunDeck,
    Removed,
    RelicStorage,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceTiming {
    #[default]
    Now,
    NextTurn,
    AfterEachCombat,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceDuration {
    #[default]
    NotApplicable,
    Turn,
    Combat,
    Run,
    UntilPlayed,
    UntilPlayedOrTurnEnds,
}

/// An applicability rule, never the result of a hidden-state test.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceCondition {
    #[default]
    Always,
    WritableEnergyCost,
    InDrawOrDiscard,
    SelectedModel(ModelId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceOperationKind {
    Upgrade,
    Exhaust,
    Discard,
    Remove,
    Gain,
    Move,
    Copy,
    Transform,
    Enchant,
    Play,
    GrantKeyword,
    AddCost,
    SetCost,
    AddReplay,
    Store,
    ApplyPower,
}

/// One ordered effect. `count` is per selected card, except a timed storage
/// return, whose count describes the cards returned at each trigger.
/// `model` names only a public enchantment, power, or named replacement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent public inheritance and cost flags"
)]
pub struct ChoiceOperation {
    pub kind: ChoiceOperationKind,
    pub subject: ChoiceSubject,
    pub scope: ChoiceScope,
    pub destination: ChoiceDestination,
    pub amount: i32,
    pub count: i32,
    pub timing: ChoiceTiming,
    pub duration: ChoiceDuration,
    pub condition: ChoiceCondition,
    pub model: Option<ModelId>,
    pub keyword: Option<CardKeyword>,
    /// A rolled replacement is publicly specified, but its identity is not.
    pub rolled: bool,
    pub keeps_upgrade: bool,
    pub keeps_enchantment: bool,
    /// A free-card effect clears energy and star cost together.
    pub affects_star_cost: bool,
}

impl ChoiceOperation {
    #[must_use]
    pub const fn new(kind: ChoiceOperationKind, scope: ChoiceScope) -> Self {
        Self {
            kind,
            scope,
            subject: ChoiceSubject::Selected,
            destination: ChoiceDestination::NotApplicable,
            amount: 0,
            count: 1,
            timing: ChoiceTiming::Now,
            duration: ChoiceDuration::NotApplicable,
            condition: ChoiceCondition::Always,
            model: None,
            keyword: None,
            rolled: false,
            keeps_upgrade: false,
            keeps_enchantment: false,
            affects_star_cost: false,
        }
    }
    #[must_use]
    pub const fn to(mut self, value: ChoiceDestination) -> Self {
        self.destination = value;
        self
    }
    #[must_use]
    pub const fn amount(mut self, value: i32) -> Self {
        self.amount = value;
        self
    }
    #[must_use]
    pub const fn count(mut self, value: i32) -> Self {
        self.count = value;
        self
    }
    #[must_use]
    pub const fn at(mut self, value: ChoiceTiming) -> Self {
        self.timing = value;
        self
    }
    #[must_use]
    pub const fn lasting(mut self, value: ChoiceDuration) -> Self {
        self.duration = value;
        self
    }
    #[must_use]
    pub const fn on(mut self, value: ChoiceSubject) -> Self {
        self.subject = value;
        self
    }
    #[must_use]
    pub fn when(mut self, value: ChoiceCondition) -> Self {
        self.condition = value;
        self
    }
    #[must_use]
    pub fn model(mut self, value: ModelId) -> Self {
        self.model = Some(value);
        self
    }
    #[must_use]
    pub const fn keyword(mut self, value: CardKeyword) -> Self {
        self.keyword = Some(value);
        self
    }
    #[must_use]
    pub const fn rolled(mut self) -> Self {
        self.rolled = true;
        self
    }
    #[must_use]
    pub const fn including_stars(mut self) -> Self {
        self.affects_star_cost = true;
        self
    }
    #[must_use]
    pub const fn inheriting(mut self, upgrade: bool, enchantment: bool) -> Self {
        self.keeps_upgrade = upgrade;
        self.keeps_enchantment = enchantment;
        self
    }
}

/// Producer-owned public effect. Missing legacy metadata stays unknown.
/// The producer of the actual screen sets `origin` from its selection source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChoiceSemantics {
    pub coverage: ChoiceCoverage,
    pub origin: ChoiceIdentity,
    pub operations: Vec<ChoiceOperation>,
}

impl Default for ChoiceSemantics {
    fn default() -> Self {
        Self {
            coverage: ChoiceCoverage::Unknown,
            origin: ChoiceIdentity::Offered,
            operations: Vec::new(),
        }
    }
}

impl ChoiceSemantics {
    #[must_use]
    pub fn known(operations: Vec<ChoiceOperation>) -> Self {
        Self {
            coverage: ChoiceCoverage::Known,
            operations,
            ..Self::default()
        }
    }
    #[must_use]
    pub fn from_origin(mut self, origin: ChoiceIdentity) -> Self {
        self.origin = origin;
        self
    }
    #[must_use]
    pub fn deck_gain() -> Self {
        Self::known(vec![
            ChoiceOperation::new(ChoiceOperationKind::Gain, ChoiceScope::Run)
                .to(ChoiceDestination::RunDeck),
        ])
    }
    #[must_use]
    pub fn deck_removal() -> Self {
        Self::known(vec![
            ChoiceOperation::new(ChoiceOperationKind::Remove, ChoiceScope::Run)
                .to(ChoiceDestination::Removed),
        ])
        .from_origin(ChoiceIdentity::DeckCard)
    }
    #[must_use]
    pub fn deck_upgrade() -> Self {
        Self::known(vec![ChoiceOperation::new(
            ChoiceOperationKind::Upgrade,
            ChoiceScope::Run,
        )])
        .from_origin(ChoiceIdentity::DeckCard)
    }
    /// Reject a malformed descriptor without inventing missing operations.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.coverage == ChoiceCoverage::Known && self.operations.is_empty() {
            return Err("known choice semantics require an operation");
        }
        if self.coverage == ChoiceCoverage::Unknown && !self.operations.is_empty() {
            return Err("unknown choice semantics cannot claim operations");
        }
        for operation in &self.operations {
            if operation.count < 0 {
                return Err("negative operation count");
            }
            if operation.kind == ChoiceOperationKind::GrantKeyword && operation.keyword.is_none() {
                return Err("keyword grant requires its keyword");
            }
            if matches!(
                operation.kind,
                ChoiceOperationKind::Enchant | ChoiceOperationKind::ApplyPower
            ) && operation.model.is_none()
            {
                return Err("operation requires a public model");
            }
            if operation.kind == ChoiceOperationKind::Transform
                && operation.rolled == operation.model.is_some()
            {
                return Err("transform requires either a named or a rolled replacement");
            }
        }
        Ok(())
    }
}

/// Public selection opportunity on a macro screen. Candidate indices refer
/// to the run deck for deck choices and to the owning offer otherwise.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChoiceOpportunity {
    pub semantics: ChoiceSemantics,
    pub candidate_indices: Vec<usize>,
    pub gold_cost: i32,
}
