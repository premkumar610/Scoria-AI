// contracts/programs/model_registry/src/instructions/update.rs

use anchor_lang::prelude::*;
use solana_program::{sysvar::clock::Clock, system_instruction};
use crate::{
    state::*,
    utils::{crypto, dao, deposits},
    ModelRegistryError,
};

#[derive(Accounts)]
#[instruction(new_version_hash: [u8; 32], zk_circuit_hash: [u8; 32], deposit: u64)]
pub struct ProposeVersionUpdate<'info> {
    #[account(mut)]
    pub model: Account<'info, ModelAccount>,

    #[account(
        init,
        payer = submitter,
        space = 8 + VersionProposal::LEN,
        seeds = [
            b"version_proposal",
            model.key().as_ref(),
            &model.active_version.to_le_bytes()
        ],
        bump
    )]
    pub proposal: Account<'info, VersionProposal>,

    #[account(mut)]
    pub submitter: Signer<'info>,

    #[account(address = dao::GOVERNANCE_PROGRAM_ID)]
    pub dao_program: Program<'info, dao::program::DaoGovernance>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteVersionUpdate<'info> {
    #[account(mut)]
    pub proposal: Account<'info, VersionProposal>,

    #[account(
        mut,
        has_one = dao_authority @ ModelRegistryError::Unauthorized,
        seeds = [b"dao_authority"],
        bump
    )]
    pub dao_authority: Account<'info, dao::DaoAuthority>,

    #[account(mut)]
    pub model: Account<'info, ModelAccount>,

    #[account(address = dao::GOVERNANCE_PROGRAM_ID)]
    pub dao_program: Program<'info, dao::program::DaoGovernance>,
}

pub fn propose_update(
    ctx: Context<ProposeVersionUpdate>,
    new_version_hash: [u8; 32],
    zk_circuit_hash: [u8; 32],
    deposit: u64,
) -> Result<()> {
    // Validate submitter permissions
    require!(
        ctx.accounts.model.owner == *ctx.accounts.submitter.key
            || dao::is_authorized_updater(&ctx.accounts.submitter.key),
        ModelRegistryError::Unauthorized
    );

    // Verify cryptographic hashes
    require!(
        crypto::validate_model_hash(&new_version_hash),
        ModelRegistryError::InvalidHash
    );
    require!(
        crypto::zk_circuit_matches(&zk_circuit_hash, &ctx.accounts.model.zk_circuit),
        ModelRegistryError::CircuitMismatch
    );

    // Process security deposit
    let required_deposit = deposits::calculate_version_deposit(
        ctx.accounts.model.active_version,
        new_version_hash,
    )?;
    require!(deposit >= required_deposit, ModelRegistryError::InsufficientDeposit);

    // Initialize proposal
    let proposal = &mut ctx.accounts.proposal;
    proposal.new_version = new_version_hash;
    proposal.zk_proof_required = zk_circuit_hash;
    proposal.proposal_state = VersionState::Pending;
    proposal.submitter = *ctx.accounts.submitter.key;
    proposal.timestamp = Clock::get()?.unix_timestamp;
    proposal.deposit = deposit;

    // Transfer deposit
    let transfer_ix = system_instruction::transfer(
        ctx.accounts.submitter.key,
        &deposits::HOLDING_ACCOUNT,
        deposit,
    );
    anchor_lang::solana_program::program::invoke(
        &transfer_ix,
        &[
            ctx.accounts.submitter.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Create DAO vote
    dao::create_version_vote(
        &ctx.accounts.dao_program,
        &proposal.key(),
        ctx.accounts.submitter.key,
    )?;

    emit!(VersionProposed {
        model: ctx.accounts.model.key(),
        new_version: new_version_hash,
        proposal: proposal.key(),
    });

    Ok(())
}

pub fn execute_update(ctx: Context<ExecuteVersionUpdate>) -> Result<()> {
    // Verify DAO approval
    require!(
        dao::is_proposal_approved(&ctx.accounts.proposal),
        ModelRegistryError::ProposalNotApproved
    );

    // Update model version
    let model = &mut ctx.accounts.model;
    model.active_version += 1;
    model.model_hash = ctx.accounts.proposal.new_version;

    // Release deposit + reward
    deposits::process_update_reward(
        &ctx.accounts.proposal,
        &ctx.accounts.dao_authority,
    )?;

    // Archive old version
    VersionArchive::create(
        model.key(),
        model.active_version - 1,
        ctx.accounts.proposal.key(),
    )?;

    emit!(VersionUpdated {
        model: model.key(),
        new_version: model.active_version,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[account]
#[derive(Default)]
pub struct VersionProposal {
    pub new_version: [u8; 32],
    pub zk_proof_required: [u8; 32],
    pub proposal_state: VersionState,
    pub submitter: Pubkey,
    pub timestamp: i64,
    pub deposit: u64,
    pub bump: u8,
}

impl VersionProposal {
    pub const LEN: usize = 32 + 32 + 1 + 32 + 8 + 8 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum VersionState {
    Pending,
    Approved,
    Rejected,
    Archived,
}

#[event]
pub struct VersionProposed {
    pub model: Pubkey,
    pub new_version: [u8; 32],
    pub proposal: Pubkey,
}

#[event]
pub struct VersionUpdated {
    pub model: Pubkey,
    pub new_version: u64,
    pub timestamp: i64,
}

#[error_code]
pub enum ModelRegistryError {
    #[msg("Insufficient security deposit")]
    InsufficientDeposit,
    #[msg("ZK circuit doesn't match model requirements")]
    CircuitMismatch,
    #[msg("DAO proposal not approved")]
    ProposalNotApproved,
    #[msg("Version downgrade not allowed")]
    VersionDowngrade,
    // ... (previous errors)
}
