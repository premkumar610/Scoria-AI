// contracts/programs/model_registry/src/error.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum ModelRegistryError {
    /* Core Validation Errors (0x1000-0x10FF) */
    #[msg("Invalid cryptographic hash length")]
    InvalidHashLength,            // 0x1770 (6000)
    #[msg("ZK proof verification failed")]
    ZkVerificationFailure,        // 0x1771
    #[msg("Model binary exceeds size limit")]
    ModelSizeExceeded,            // 0x1772
    #[msg("Invalid semantic version format")]
    InvalidSemanticVersion,       // 0x1773

    /* Access Control Errors (0x2000-0x20FF) */
    #[msg("Unauthorized access attempt")]
    UnauthorizedAccess,           // 0x1774
    #[msg("Account does not have required privilege level")]
    InsufficientPrivilege,        // 0x1775
    #[msg("Multi-signature threshold not met")]
    MultisigThresholdFail,        // 0x1776
    #[msg("Operation disabled during emergency pause")]
    EmergencyLockActive,          // 0x1777

    /* Economic Model Errors (0x3000-0x30FF) */
    #[msg("Insufficient staking balance")]
    InsufficientStake,            // 0x1778
    #[msg("Payment token not whitelisted")]
    InvalidPaymentToken,          // 0x1779
    #[msg("Royalty distribution failed")]
    RoyaltyDistributionError,     // 0x177A
    #[msg("Slashing condition triggered")]
    SlashingConditionMet,         // 0x177B

    /* Version Control Errors (0x4000-0x40FF) */
    #[msg("Version history storage exhausted")]
    HistoryFull,                  // 0x177C
    #[msg("Duplicate model version detected")]
    DuplicateVersion,             // 0x177D
    #[msg("Parent version mismatch in lineage")]
    ParentMismatch,               // 0x177E
    #[msg("Cannot rollback beyond genesis version")]
    RollbackLimitExceeded,        // 0x177F

    /* ZK Circuit Errors (0x5000-0x50FF) */
    #[msg("ZK circuit compilation failed")]
    ZkCircuitCompileError,        // 0x1780
    #[msg("Trusted setup verification failed")]
    TrustedSetupError,            // 0x1781
    #[msg("Proof generation timeout")]
    ZkProofTimeout,               // 0x1782
    #[msg("Invalid elliptic curve parameters")]
    CurveParameterMismatch,       // 0x1783

    /* Governance Errors (0x6000-0x60FF) */
    #[msg("DAO proposal has expired")]
    ProposalExpired,              // 0x1784
    #[msg("Voting power insufficient for quorum")]
    QuorumNotMet,                 // 0x1785
    #[msg("Delegated voter revoked authority")]
    DelegationRevoked,            // 0x1786
    #[msg("Invalid governance parameter combination")]
    GovernanceConfigConflict,     // 0x1787

    /* Dependency Errors (0x7000-0x70FF) */
    #[msg("Runtime dependency hash mismatch")]
    DependencyConflict,           // 0x1788
    #[msg("Required dependency not found")]
    MissingDependency,            // 0x1789
    #[msg("Circular dependency detected")]
    CircularDependency,            // 0x178A
    #[msg("Unsupported dependency version")]
    UnsupportedDependencyVersion, // 0x178B

    /* System Errors (0x8000-0x80FF) */
    #[msg("Insufficient compute budget")]
    ComputeLimitExceeded,         // 0x178C
    #[msg("Cross-program invocation failed")]
    CPIExecutionError,            // 0x178D
    #[msg("Account rent exemption not satisfied")]
    RentExemptionFail,            // 0x178E
    #[msg("Invalid sysvar account address")]
    SysvarAccountMismatch,        // 0x178F

    /* Federated Learning Errors (0x9000-0x90FF) */
    #[msg("Contribution threshold not met")]
    ContributionThresholdFail,    // 0x1790
    #[msg("Differential privacy budget exhausted")]
    PrivacyBudgetExhausted,       // 0x1791
    #[msg("Gradient update validation failed")]
    InvalidGradientUpdate,        // 0x1792
    #[msg("Federated round timeout")]
    FederatedRoundTimeout,        // 0x1793

    /* Audit & Compliance Errors (0xA000-0xA0FF) */
    #[msg("Mandatory audit signature missing")]
    AuditSignatureRequired,       // 0x1794
    #[msg("Regulatory hold prevents modification")]
    LegalHoldActive,              // 0x1795
    #[msg("Data residency rule violation")]
    DataResidencyConflict,        // 0x1796
    #[msg("Sanctioned jurisdiction restriction")]
    SanctionedJurisdiction,       // 0x1797
}
