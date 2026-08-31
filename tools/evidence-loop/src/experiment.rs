use crate::state::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Classification {
    Conclusive,
    Inconclusive,
    Degenerate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Mechanism {
    Exercised,
    Unexercised,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HypothesisStatus {
    Supported,
    Refuted,
    Untested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClosureStatus {
    Confirmed,
    Refuted,
    Inconclusive,
}

/// Whether a review was done by someone other than the person who recorded
/// what's being reviewed, or by the same party because no independent
/// reviewer was available. Self-review does not block a transition -- it
/// exists so that limitation stays visible on the board and in the sidecar
/// instead of being indistinguishable from a real independent check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewKind {
    Independent,
    #[serde(rename = "SELF")]
    SelfReviewed,
}

macro_rules! screaming_display {
    ($t:ty, $($variant:ident => $s:literal),+ $(,)?) => {
        impl $t {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $s),+ }
            }
        }
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }
        impl std::str::FromStr for $t {
            type Err = anyhow::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_ascii_uppercase().as_str() {
                    $($s => Ok(Self::$variant),)+
                    other => anyhow::bail!("invalid value {other:?} for {}", stringify!($t)),
                }
            }
        }
    };
}

screaming_display!(Classification, Conclusive => "CONCLUSIVE", Inconclusive => "INCONCLUSIVE", Degenerate => "DEGENERATE");
screaming_display!(Mechanism, Exercised => "EXERCISED", Unexercised => "UNEXERCISED");
screaming_display!(HypothesisStatus, Supported => "SUPPORTED", Refuted => "REFUTED", Untested => "UNTESTED");
screaming_display!(ClosureStatus, Confirmed => "CONFIRMED", Refuted => "REFUTED", Inconclusive => "INCONCLUSIVE");
screaming_display!(ReviewKind, Independent => "INDEPENDENT", SelfReviewed => "SELF");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInfo {
    pub path: String,
    pub baseline_commit: String,
    pub hash: String,
    pub components: u64,
    pub rows: u64,
    pub terminus: String,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultInfo {
    pub classification: Classification,
    pub mechanism: Mechanism,
    pub hypothesis_status: HypothesisStatus,
    pub observation: String,
    pub interpretation: String,
    #[serde(default)]
    pub override_applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_reason: Option<String>,
    pub recorded_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_kind: Option<ReviewKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub committed_at: DateTime<Utc>,
    pub commit_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureInfo {
    pub status: ClosureStatus,
    pub established: String,
    pub not_established: String,
    pub remaining_questions: String,
    pub recorded_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_kind: Option<ReviewKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    pub from: State,
    pub to: State,
    pub command: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentRecord {
    pub id: String,
    pub hypothesis: String,
    pub state: State,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ResultInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<ArtifactInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closure: Option<ClosureInfo>,
    #[serde(default)]
    pub history: Vec<TransitionRecord>,
}

impl ExperimentRecord {
    pub fn new(id: String, hypothesis: String) -> Self {
        Self {
            id,
            hypothesis,
            state: State::RawPending,
            created_at: Utc::now(),
            raw: None,
            result: None,
            artifact: None,
            closure: None,
            history: Vec::new(),
        }
    }

    pub fn record_transition(&mut self, command: crate::state::Command) {
        let from = self.state;
        let to = command.produces();
        self.history.push(TransitionRecord {
            from,
            to,
            command: command.name().to_string(),
            at: Utc::now(),
        });
        self.state = to;
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
        Ok(serde_yaml::from_str(&text)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_yaml::to_string(self)?;
        fs::write(path, text)?;
        Ok(())
    }
}
