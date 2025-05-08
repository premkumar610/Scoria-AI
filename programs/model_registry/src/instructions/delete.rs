// contracts/programs/model_registry/src/instructions/delete.rs

use anchor_lang::prelude::*;
use solana_program::{program::invoke, system_instruction};
use crate::{
    state::*,
    utils::{deposits, governance, crypto},
    ModelRegistryError,
};

#[derive(Accounts)]
#[instruction(force: bool)]
pub struct DeleteModel<'info> {
    #[account(
        mut,
        close = receiver,
        constraint = 
            model.owner == remover.key() || 
            governance::is_emergency_operator(&remover.key) ||
            (force && governance::is_authorized(&authority.key))
    )]
    pub model: Account<'info, ModelAccount>,

    #[account(
        mut,
        seeds = [b"model", &model.model_hash],
        bump = model.bump
    )]
    pub model_pda: AccountInfo<'info>,

    #[account(mut)]
    pub remover: Signer<'info>,

    #[account(
        mut,
        address = governance::get_treasury_address()
    )]
    /// CHECK: Verified via governance program
    pub receiver: AccountInfo<'info>,

    #[account(
        has_one = governance_program,
        seeds = [b"governance"],
        bump
    )]
    pub authority: Account<'info, governance::GovernanceAuthority>,

    #[account(address = governance::program::ID)]
    pub governance_program: Program<'info, governance::program::Governance>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DeleteModel>, force: bool) -> Result<()> {
    // Validate deletion conditions
    require!(
        validate_deletion_conditions(
            &ctx.accounts.model,
            &ctx.accounts.remover.key,
            force
        )?,
        ModelRegistryError::UnauthorizedDeletion
    );

    // Process refunds if not force-deleted
    if !force {
        let refund_amount = deposits::calculate_refund(
            ctx.accounts.model.storage_fee,
            Clock::get()?.unix_timestamp - ctx.accounts.model.timestamp
        )?;

        let refund_ix = system_instruction::transfer(
            &ctx.accounts.receiver.key(),
            &ctx.accounts.remover.key(),
            refund_amount,
        );

        invoke(
            &refund_ix,
            &[
                ctx.accounts.receiver.clone(),
                ctx.accounts.remover.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
    }

    // Close PDA account and reclaim rent
    let close_ix = system_instruction::close_account(
        ctx.accounts.model_pda.key,
        ctx.accounts.receiver.key,
        ctx.accounts.model_pda.key,
    );

    invoke(
        &close_ix,
        &[
            ctx.accounts.model_pda.clone(),
            ctx.accounts.receiver.clone(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Archive deletion record
    let archive_account = Archive::create(
        ctx.accounts.model.key(),
        ArchiveReason::UserDeleted,
        Clock::get()?.unix_timestamp,
        *ctx.accounts.remover.key,
    )?;

    emit!(ModelDeleted {
        model: ctx.accounts.model.key(),
        remover: *ctx.accounts.remover.key,
        timestamp: Clock::get()?.unix_timestamp,
        refund: if force { 0 } else { refund_amount },
        archived: archive_account.key(),
    });

    Ok(())
}

fn validate_deletion_conditions(
    model: &ModelAccount,
    remover: &Pubkey,
    force: bool,
) -> Result<bool> {
    // Normal user-initiated deletion
    if model.owner == *remover && !force {
        return Ok(true);
    }

    // Governance force deletion
    if force {
        return Ok(governance::is_authorized(remover));
    }

    // Emergency operator override
    Ok(governance::is_emergency_operator(remover))
}

#[event]
pub struct ModelDeleted {
    pub model: Pubkey,
    pub remover: Pubkey,
    pub timestamp: i64,
    pub refund: u64,
    pub archived: Pubkey,
}

#[account]
pub struct Archive {
    pub original_model: Pubkey,
    pub reason: ArchiveReason,
    pub timestamp: i64,
    pub archived_by: Pubkey,
    pub model_hash: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum ArchiveReason {
    UserDeleted,
    GovernanceRemoval,
    Emergency,
    Deprecated,
}

impl Archive {
    pub fn create(
        original_model: Pubkey,
        reason: ArchiveReason,
        timestamp: i64,
        archived_by: Pubkey,
    ) -> Result<Account<Self>> {
        // Implementation would initialize archive account
        // with model snapshot data
        Ok(Self {
            original_model,
            reason,
            timestamp,
            archived_by,
            model_hash: [0; 32], // Actual hash from original model
        })
    }
}

#[error_code]
pub enum ModelRegistryError {
    #[msg("Unauthorized model deletion attempt")]
    UnauthorizedDeletion,
    #[msg("Active model versions exist")]
    ActiveVersions,
    #[msg("Pending governance proposals")]
    PendingProposals,
    // ... (previous errors)
}
