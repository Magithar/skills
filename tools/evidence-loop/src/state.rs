use serde::{Deserialize, Serialize};
use std::fmt;

/// The evidence lifecycle for a single experiment. See
/// `docs/evidence-loop/state-machine.md` for the full transition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    RawPending,
    RawVerified,
    ResultRecorded,
    ResultReviewed,
    ArtifactCommitted,
    ClosureRecorded,
    ClosureReviewed,
    HypothesisGate,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::RawPending => "RAW_PENDING",
            State::RawVerified => "RAW_VERIFIED",
            State::ResultRecorded => "RESULT_RECORDED",
            State::ResultReviewed => "RESULT_REVIEWED",
            State::ArtifactCommitted => "ARTIFACT_COMMITTED",
            State::ClosureRecorded => "CLOSURE_RECORDED",
            State::ClosureReviewed => "CLOSURE_REVIEWED",
            State::HypothesisGate => "HYPOTHESIS_GATE",
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A named command that performs at most one transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Verify,
    Result,
    ReviewResult,
    CommitArtifact,
    Close,
    ReviewClosure,
    Gate,
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Command::Verify => "verify",
            Command::Result => "result",
            Command::ReviewResult => "review-result",
            Command::CommitArtifact => "commit-artifact",
            Command::Close => "close",
            Command::ReviewClosure => "review-closure",
            Command::Gate => "gate",
        }
    }

    /// The state an experiment must be in for this command to be legal.
    pub fn requires(&self) -> State {
        match self {
            Command::Verify => State::RawPending,
            Command::Result => State::RawVerified,
            Command::ReviewResult => State::ResultRecorded,
            Command::CommitArtifact => State::ResultReviewed,
            Command::Close => State::ArtifactCommitted,
            Command::ReviewClosure => State::ClosureRecorded,
            Command::Gate => State::ClosureReviewed,
        }
    }

    /// The state this command transitions the experiment into.
    pub fn produces(&self) -> State {
        match self {
            Command::Verify => State::RawVerified,
            Command::Result => State::ResultRecorded,
            Command::ReviewResult => State::ResultReviewed,
            Command::CommitArtifact => State::ArtifactCommitted,
            Command::Close => State::ClosureRecorded,
            Command::ReviewClosure => State::ClosureReviewed,
            Command::Gate => State::HypothesisGate,
        }
    }

    /// What the CLI tells an agent it may and may not do while performing
    /// this command. Kept here, not left for the agent to infer.
    pub fn required_facts(&self) -> &'static [&'static str] {
        match self {
            Command::Verify => &[
                "artifact exists at the given path",
                "baseline commit identified",
                "structure (components/rows) computed",
                "content hash recorded",
            ],
            Command::Result => &[
                "raw artifact verified",
                "classification is one of CONCLUSIVE, INCONCLUSIVE, DEGENERATE",
                "mechanism is EXERCISED or UNEXERCISED",
                "hypothesis_status is SUPPORTED, REFUTED, or UNTESTED",
            ],
            Command::ReviewResult => &[
                "observation matches the raw artifact",
                "classification is justified",
                "mechanism status is correct",
                "interpretation does not exceed the evidence",
                "hypothesis status is justified",
            ],
            Command::CommitArtifact => &[
                "result has been reviewed",
                "artifact file exists and is unchanged since verification",
            ],
            Command::Close => &[
                "artifact committed to git",
                "closure status follows from the reviewed result",
            ],
            Command::ReviewClosure => &[
                "closure follows the result and the raw evidence",
                "no new evidence introduced",
                "no unsupported inference introduced",
                "remaining uncertainty stated explicitly",
            ],
            Command::Gate => &[
                "raw verified",
                "result reviewed",
                "artifact committed",
                "closure reviewed",
            ],
        }
    }

    pub fn forbidden(&self) -> &'static [&'static str] {
        match self {
            Command::Verify => &["interpret the result", "create a new hypothesis"],
            Command::Result => &["skip stating the mechanism status", "claim REFUTED for an UNEXERCISED mechanism"],
            Command::ReviewResult => &["introduce new evidence"],
            Command::CommitArtifact => &["modify historical evidence", "commit anything but the raw artifact"],
            Command::Close => &["convert a DEGENERATE/UNTESTED result into a refutation without an explicit override"],
            Command::ReviewClosure => &["introduce new evidence", "soften stated uncertainty"],
            Command::Gate => &["propose a new hypothesis before this command succeeds"],
        }
    }
}

/// The command legal from a given state, if any. `HypothesisGate` is
/// terminal for v1 — a new hypothesis is a new experiment, not a
/// transition of this one.
pub fn next_command_for(state: State) -> Option<Command> {
    match state {
        State::RawPending => Some(Command::Verify),
        State::RawVerified => Some(Command::Result),
        State::ResultRecorded => Some(Command::ReviewResult),
        State::ResultReviewed => Some(Command::CommitArtifact),
        State::ArtifactCommitted => Some(Command::Close),
        State::ClosureRecorded => Some(Command::ReviewClosure),
        State::ClosureReviewed => Some(Command::Gate),
        State::HypothesisGate => None,
    }
}

#[derive(Debug, thiserror::Error)]
#[error("illegal transition: experiment {experiment} is in state {actual}; `{command}` requires {required}")]
pub struct TransitionError {
    pub experiment: String,
    pub actual: State,
    pub required: State,
    pub command: &'static str,
}

pub fn check_transition(experiment: &str, actual: State, command: Command) -> Result<(), TransitionError> {
    let required = command.requires();
    if actual != required {
        return Err(TransitionError {
            experiment: experiment.to_string(),
            actual,
            required,
            command: command.name(),
        });
    }
    Ok(())
}
