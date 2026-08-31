use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "evidence-loop", version, about = "Deterministic evidence bookkeeping and closure enforcement.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create BOARD.md and .evidence-loop/ in the current directory.
    Init,

    /// Register a new experiment and its hypothesis.
    New {
        hypothesis: String,
    },

    /// Show the current experiment and its state.
    Status {
        #[arg(long)]
        json: bool,
    },

    /// Show the next permitted action for the current experiment.
    Next {
        #[arg(long)]
        json: bool,
        /// Operate on this experiment instead of the current one.
        experiment: Option<String>,
    },

    /// Mechanically verify a raw artifact and record its structure.
    Verify {
        experiment: String,
        /// Path to the raw artifact, relative to the project root.
        path: String,
        #[arg(long)]
        baseline: Option<String>,
    },

    /// Record a result from verified raw evidence.
    Result {
        experiment: String,
        #[arg(long)]
        classification: String,
        #[arg(long)]
        mechanism: String,
        #[arg(long = "hypothesis-status")]
        hypothesis_status: String,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        interpretation: String,
        /// Explicitly override the DEGENERATE-implies-UNTESTED rule.
        #[arg(long)]
        override_flag: bool,
        #[arg(long)]
        override_reason: Option<String>,
    },

    /// Record that a result has been independently reviewed.
    ReviewResult {
        experiment: String,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        notes: Option<String>,
    },

    /// Commit the raw artifact to git, separately from interpretation.
    CommitArtifact {
        experiment: String,
    },

    /// Record experiment closure.
    Close {
        experiment: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        established: String,
        #[arg(long, name = "not-established")]
        not_established: String,
        #[arg(long)]
        remaining_questions: String,
    },

    /// Record that closure has been independently reviewed.
    ReviewClosure {
        experiment: String,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        notes: Option<String>,
    },

    /// Confirm every prerequisite is met and open the hypothesis gate.
    Gate {
        experiment: String,
        #[arg(long)]
        json: bool,
    },
}
