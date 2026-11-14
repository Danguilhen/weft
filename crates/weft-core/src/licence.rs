// SPDX-License-Identifier: MIT

//! Licence and moderation state.

use serde::{Deserialize, Serialize};

/// What a node has actually established about a licence claim.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenceStatus {
    #[default]
    Unverified,
    EvidenceProvided,
    VerifiedEvidence,
    Disputed,
    Invalidated,
}

impl LicenceStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::EvidenceProvided => "evidence_provided",
            Self::VerifiedEvidence => "verified_evidence",
            Self::Disputed => "disputed",
            Self::Invalidated => "invalidated",
        }
    }

    #[must_use]
    pub fn is_verified(self) -> bool {
        self == Self::VerifiedEvidence
    }
}

/// Lifecycle of a local content listing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationState {
    #[default]
    Pending,
    Active,
    Restricted,
    Removed,
    Rejected,
}

impl ModerationState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Restricted => "restricted",
            Self::Removed => "removed",
            Self::Rejected => "rejected",
        }
    }

    #[must_use]
    pub fn is_public(self) -> bool {
        self == Self::Active
    }
}

#[cfg(test)]
mod tests {
    use super::{LicenceStatus, ModerationState};

    #[test]
    fn only_verified_evidence_is_verified() {
        assert!(LicenceStatus::VerifiedEvidence.is_verified());
        assert!(!LicenceStatus::Unverified.is_verified());
        assert!(!LicenceStatus::EvidenceProvided.is_verified());
    }

    #[test]
    fn only_active_content_is_public() {
        assert!(ModerationState::Active.is_public());
        assert!(!ModerationState::Pending.is_public());
        assert_eq!(ModerationState::Removed.as_str(), "removed");
    }
}
