// contracts/programs/model_registry/src/state/model.rs

use anchor_lang::prelude::*;
use solana_program::{pubkey::Pubkey, clock::Clock};
use std::collections::BTreeMap;

#[account]
#[derive(Default)]
pub struct ModelAccount {
    // Core Metadata
    pub model_hash: [u8; 32],      // SHA3-256 of model binary
    pub zk_circuit: [u8; 32],      // Poseidon hash of ZK circuit
    pub owner: Pubkey,             // Original uploader
    pub timestamp: i64,            // Unix epoch seconds
    pub storage_fee: u64,          // Lamports paid per epoch

    // Version Control
    pub active_version: u64,       // Currently deployed version
    pub last_update: i64,          // Last version change time
    pub version_history: Vec<[u8; 32]>, // Merkle tree of past hashes

    // Access Control
    pub acl: BTreeMap<Pubkey, AccessLevel>, // Permission levels
    pub is_public: bool,            // Open inference access
    
    // Federated Learning
    pub contributors: Vec<Pubkey>, // Data providers
    pub contribution_threshold: u64, // Min stake to participate
    
    // Governance
    pub governance_model: GovernanceType,
    pub dao: Option<Pubkey>,        // Associated DAO
    
    // Security
    pub emergency_pause: bool,
    pub audit_signatures: Vec<[u8; 64]>, // Auditor Ed25519 sigs
    
    // PDA Metadata
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AccessLevel {
    NoAccess,
    InferenceOnly,
    Contributor,
    Administrator,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GovernanceType {
    OwnerControlled,
    DaoGoverned,
    FederatedVoting,
}

impl ModelAccount {
    pub const MAX_CONTRIBUTORS: usize = 100;
    pub const VERSION_HISTORY_DEPTH: usize = 256;
    pub const ACL_ENTRY_SIZE: usize = 32 + 1; // Pubkey + AccessLevel

    /// Space calculation for account initialization
    pub fn space() -> usize {
        8 + // Anchor discriminant
        32 + // model_hash
        32 + // zk_circuit
        32 + // owner
        8 +  // timestamp
        8 +  // storage_fee
        8 +  // active_version
        8 +  // last_update
        (Self::VERSION_HISTORY_DEPTH * 32) + // version_history
        (Self::MAX_CONTRIBUTORS * 32) + // contributors
        8 +  // contribution_threshold
        1 +  // is_public
        1 +  // governance_model (enum tag)
        32 + // dao (Option)
        1 +  // emergency_pause
        (3 * 64) + // audit_signatures (3 auditors max)
        1 +  // bump
        (Self::MAX_CONTRIBUTORS * Self::ACL_ENTRY_SIZE) // acl
    }

    /// Validate model owner or authorized delegate
    pub fn check_access(&self, user: &Pubkey, required_level: AccessLevel) -> Result<()> {
        require!(
            self.emergency_pause == false,
            ModelRegistryError::ModelPaused
        );

        let access = self.acl.get(user).unwrap_or(&AccessLevel::NoAccess);
        if user == &self.owner || access >= &required_level {
            Ok(())
        } else {
            err!(ModelRegistryError::UnauthorizedAccess)
        }
    }

    /// Add version to merkleized history
    pub fn record_version(&mut self, new_hash: [u8; 32]) -> Result<()> {
        require!(
            self.version_history.len() < Self::VERSION_HISTORY_DEPTH,
            ModelRegistryError::HistoryFull
        );

        if let Some(last) = self.version_history.last() {
            require!(
                new_hash != *last,
                ModelRegistryError::DuplicateVersion
            );
        }

        self.version_history.push(new_hash);
        self.active_version += 1;
        self.last_update = Clock::get()?.unix_timestamp;
        Ok(())
    }

    /// Add contributor with stake verification
    pub fn add_contributor(&mut self, contributor: Pubkey, stake: u64) -> Result<()> {
        require!(
            stake >= self.contribution_threshold,
            ModelRegistryError::InsufficientStake
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
        Ok(())
    }
}

#[event]
pub struct ModelStateChanged {
    pub model: Pubkey,
    pub field: ModelField,
    pub old_value: Vec<u8>,
    pub new_value: Vec<u8>,
    pub changed_by: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum ModelField {
    ModelHash,
    ZkCircuit,
    AccessControl,
    GovernanceModel,
    PauseStatus,
}

#[error_code]
pub enum ModelRegistryError {
    #[msg("Unauthorized access attempt")]
    UnauthorizedAccess,
    #[msg("Model currently paused")]
    ModelPaused,
    #[msg("Version history storage exhausted")]
    HistoryFull,
    #[msg("Duplicate model version detected")]
    DuplicateVersion,
    #[msg("Contributor stake below threshold")]
    InsufficientStake,
    #[msg("Maximum contributors reached")]
    ContributorLimit,
    #[msg("Contributor already exists")]
    DuplicateContributor,
    // ... (previous errors)
}
