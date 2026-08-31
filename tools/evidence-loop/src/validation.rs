use crate::experiment::{Classification, HypothesisStatus, Mechanism};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error(
        "mechanism UNEXERCISED cannot imply hypothesis_status REFUTED: a mechanism that never ran cannot refute anything"
    )]
    MechanismRefutedConflict,
    #[error(
        "classification DEGENERATE with hypothesis_status {0} requires --override plus --override-reason: a degenerate experiment cannot silently become a refutation"
    )]
    DegenerateRequiresOverride(HypothesisStatus),
}

/// Semantic validation for a result. Structural verification (does the raw
/// artifact exist, was it hashed) happens in `evidence.rs`; this only
/// checks that the *combination* of fields is not self-contradictory.
pub fn validate_result(
    classification: Classification,
    mechanism: Mechanism,
    hypothesis_status: HypothesisStatus,
    override_applied: bool,
    override_reason: &Option<String>,
) -> Result<(), ValidationError> {
    if mechanism == Mechanism::Unexercised && hypothesis_status == HypothesisStatus::Refuted {
        return Err(ValidationError::MechanismRefutedConflict);
    }
    if classification == Classification::Degenerate && hypothesis_status != HypothesisStatus::Untested {
        let has_reason = override_reason.as_ref().is_some_and(|s| !s.trim().is_empty());
        if !override_applied || !has_reason {
            return Err(ValidationError::DegenerateRequiresOverride(hypothesis_status));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexercised_refuted_is_rejected() {
        let err = validate_result(
            Classification::Inconclusive,
            Mechanism::Unexercised,
            HypothesisStatus::Refuted,
            false,
            &None,
        )
        .unwrap_err();
        assert!(matches!(err, ValidationError::MechanismRefutedConflict));
    }

    #[test]
    fn degenerate_untested_is_accepted() {
        validate_result(
            Classification::Degenerate,
            Mechanism::Unexercised,
            HypothesisStatus::Untested,
            false,
            &None,
        )
        .unwrap();
    }

    #[test]
    fn degenerate_supported_without_override_is_rejected() {
        let err = validate_result(
            Classification::Degenerate,
            Mechanism::Exercised,
            HypothesisStatus::Supported,
            false,
            &None,
        )
        .unwrap_err();
        assert!(matches!(err, ValidationError::DegenerateRequiresOverride(_)));
    }

    #[test]
    fn degenerate_supported_with_override_is_accepted() {
        validate_result(
            Classification::Degenerate,
            Mechanism::Exercised,
            HypothesisStatus::Supported,
            true,
            &Some("reviewed manually, see notes".into()),
        )
        .unwrap();
    }
}
