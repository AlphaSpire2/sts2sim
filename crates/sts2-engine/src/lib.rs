//! Deterministic, headless simulator substrate.
//!
//! Content is supplied through [`ContentRegistry`]; this crate owns state,
//! decisions, validation, effect settlement, hooks, and exact-state identity.

mod ascension;
mod belief;
mod choice_semantics;
mod command;
mod model;
mod netsort;
mod run;
mod scenario;
mod setup;
mod simulator;

pub use ascension::*;
pub use belief::{ACT_BELIEF_MODEL_VERSION, ActBeliefState, BELIEF_MODEL_VERSION, BeliefState};
pub use choice_semantics::*;
pub use command::*;
pub use model::*;
pub use run::*;
pub use scenario::{PileName, ScenarioBuilder};
pub use setup::{
    COMBAT_SETUP_VERSION, CombatSetup, CounterKind, SetupCard, SetupEnchantment, SetupRelic,
    SetupRoom,
};
pub use simulator::{ContentRegistry, Simulator, Snapshot};
