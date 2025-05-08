// contracts/programs/model_registry/src/state/version.rs

use anchor_lang::prelude::*;
use solana_program::{clock::Clock, pubkey::Pubkey};
use std::collections::{BTreeMap, HashMap};

#[account]
#[derive(Default)]
pub struct VersionMetadata {
    // Core Identity
    pub version: u64,                  // Monotonic version counter
    pub model_hash: [u8; 32],           // SHA3-256 of model binary
    pub parent_version: Option<u64>,    // Previous version in lineage

    // Provenance Tracking
    pub timestamp: i64,                 // Creation time (Unix epoch)
    pub contributors: Vec<Pubkey>,      // Addresses contributing to this version
    pub dependencies: HashMap<String, [u8; 32]>, // External dependency hashes
    
    // Cryptographic Proofs
    pub zk_proof_hash: [u8; 32],        // Poseidon hash of ZK circuit
    pub audit_signature: Option<[u8; 64]>, // Auditor's Ed25519 signature
    
    // Compatibility
    pub compatible_with: Vec<u64>,      // Previous compatible versions
    pub deprecated: bool,               // Marked for phase-out
    pub deprecation_time: i64,          // Scheduled sunset timestamp

    // Economic Layer
    pub reward_shares: BTreeMap<Pubkey, u8>, // Contributor profit shares (percentage)
    pub security_deposit: u64,          // Lamports locked for version integrity

    // PDA Metadata
    pub bump: u8,
}

impl VersionMetadata {
    pub const MAX_CONTRIBUTORS: usize = 50;
    pub const MAX_DEPENDENCIES: usize = 20;
    pub const MAX_COMPATIBLE_VERSIONS: usize = 10;

    /// Calculate required account space
    pub fn space() -> usize {
        8 +  // Anchor discriminant
        8 +  // version
        32 + // model_hash
        1 + 8 + // parent_version (Option)
        8 +  // timestamp
        (Self::MAX_CONTRIBUTORS * 32) + // contributors
        (Self::MAX_DEPENDENCIES * (32 + 32)) + // dependencies (String len + hash)
        32 + // zk_proof_hash
        1 + 64 + // audit_signature (Option)
        1 + 8 + // deprecated + deprecation_time
        (Self::MAX_COMPATIBLE_VERSIONS * 8) + // compatible_with
        (Self::MAX_CONTRIBUTORS * (32 + 1)) + // reward_shares
        8 +  // security_deposit
        1    // bump
    }

    /// Validate version lineage consistency
    pub fn validate_lineage(&self, parent: &VersionMetadata) -> Result<()> {
        require!(
            parent.version < self.version,
            ModelRegistryError::VersionOrderViolation
        );
        
        require!(
            self.parent_version == Some(parent.version),
            ModelRegistryError::ParentMismatch
        );

        // Verify ZK circuit upgrade compatibility
        if self.zk_proof_hash != parent.zk_proof_hash {
            require!(
                self.audit_signature.is_some(),
                ModelRegistryError::UnauditedCircuitChange
            );
        }

        Ok(())
    }

    /// Check dependency compatibility
    pub fn check_dependencies(&self, runtime_deps: &HashMap<String, [u8; 32]>) -> Result<()> {
        for (name, expected_hash) in &self.dependencies {
            match runtime_deps.get(name) {
                Some(actual) => require!(
                    actual == expected_hash,
                    ModelRegistryError::DependencyConflict
                ),
                None => require!(
                    self.deprecated,  // Allow missing deps only in deprecated versions
                    ModelRegistryError::MissingDependency
                ),
            }
        }
        Ok(())
    }

    /// Add contributor with share allocation
    pub fn add_contributor(&mut self, contributor: Pubkey, share: u8) -> Result<()> {
        require!(
            (self.reward_shares.values().sum::<u8>() + share) <= 100,
            ModelRegistryError::InvalidProfitShare
        );
        
        require!(
            !self.contributors.contains(&contributor),
            ModelRegistryError::DuplicateContributor
        );

        require!(
            self.contributors.len() < Self::MAX_CONTRIBUTORS,
            ModelRegistryError::ContributorLimit
        );

        self.contributors.push(contributor);
        self.reward_shares.insert(contributor, share);
        Ok(())
    }

    /// Schedule deprecation with sunset policy
    pub fn schedule_deprecation(&mut self, sunset_period: i64) -> Result<()> {
        require!(
            !self.deprecated,
            ModelRegistryError::AlreadyDeprecated
        );

        self.deprecated = true;
        self.deprecation_time = Clock::get()?.unix_timestamp + sunset_period;
        Ok(())
    }
}

#[event]
pub struct VersionCreated {
    pub version: u64,
    pub model: Pubkey,
    pub lineage_depth: u32,
    pub dependency_count: u8,
}

#[event]
pub struct VersionDeprecated {
    pub version: u64,
    pub sunset_time: i64,
    pub migration_target: Option<u64>,
}

#[error_code]
pub enum ModelRegistryError {
    #[msg("Version numbering must be monotonic")]
    VersionOrderViolation,
    #[msg("Parent version does not match lineage")]
    ParentMismatch,
    #[msg("ZK circuit changes require audit signature")]
    UnauditedCircuitChange,
    #[msg("Runtime dependency hash mismatch")]
    DependencyConflict,
    #[msg("Required dependency not found")]
    MissingDependency,
    #[msg("Total profit shares exceed 100%")]
    InvalidProfitShare,
    #[msg("Version already marked deprecated")]
    AlreadyDeprecated,
    // ... (previous errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::clock::Epoch;

    #[test]
    fn test_version_validation() {
        let mut parent = VersionMetadata::default();
        parent.version = 1;
        parent.zk_proof_hash = [0; 32];

        let mut child = VersionMetadata::default();
        child.version = 2;
        child.parent_version = Some(1);
        child.zk_proof_hash = [1; 32]; // Changed without audit

        assert_eq!(
            child.validate_lineage(&parent),
            Err(ModelRegistryError::UnauditedCircuitChange.into())
        );
    }

    #[test]
    fn test_dependency_check() {
        let mut version = VersionMetadata::default();
        version.dependencies.insert("lib_ai".to_string(), [1; 32]);

        let mut runtime = HashMap::new();
        runtime.insert("lib_ai".to_string(), [2; 32]);

        assert_eq!(
            version.check_dependencies(&runtime),
            Err(ModelRegistryError::DependencyConflict.into())
        );
    }
}
