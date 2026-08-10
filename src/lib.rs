pub mod error;
pub mod goals;
pub mod repair;
mod translator;
pub mod validate;
pub mod verification;

pub use error::TranslateError;
pub use goals::{GoalPredicate, GoalSpec, UnmetGoal, check_goals};
pub use translator::{translate, translate_goals};
pub use verification::{verify_program, VerificationConfig, VerificationResult, VerificationStatus};
