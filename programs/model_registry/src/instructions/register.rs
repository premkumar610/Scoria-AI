// contracts/programs/model_registry/src/instructions/register.rs

use anchor_lang::prelude::*;
use solana_program::{system_instruction, sysvar::rent::Rent};
use crate::{state::*, utils::{crypto, fees}, ModelRegistryError};

#[derive(Accounts)]
#[instruction(model_hash: [u8; 32], zk_circuit_hash: [u8; 32], storage_fee: u64)]
pub struct RegisterModel<'info> {
    #[account(
        mut,
        has_one = admin_authority @ ModelRegistryError::Unauthorized,
        seeds = [b"admin"],
        bump = admin.bump
    )]
    pub admin: Account<'info, AdminAccount>,

    #[account(
        init,
        payer = payer,
        space = 8 + ModelAccount::LEN,
        seeds = [b"model", &model_hash],
        bump
    )]
    pub model_account: Account<'info, ModelAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(address = admin.authority)]
    pub admin_authority: Signer<'info>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

    #[account(address = sysvar::rent::ID)]
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<RegisterModel>,
    model_hash: [u8; 32],
    zk_circuit_hash: [u8; 32],
    storage_fee: u64,
) -> Result<()> {
    // Validate model hash format
    require!(
        crypto::is_valid_hash(&model_hash),
        ModelRegistryError::InvalidHash
    );

    // Check for existing registration
    require!(
        !ModelAccount::is_registered(&model_hash),
        ModelRegistryError::DuplicateModel
    );

    // Verify storage fee meets requirements
    let required_fee = fees::calculate_storage_fee(
        storage_fee,
        &ctx.accounts.rent
    )?;
    require!(
        storage_fee >= required_fee,
        ModelRegistryError::InsufficientFee
    );

    // Initialize model account
    let model = &mut ctx.accounts.model_account;
    model.model_hash = model_hash;
    model.zk_circuit = zk_circuit_hash;
    model.owner = *ctx.accounts.payer.key;
    model.timestamp = Clock::get()?.unix_timestamp;
    model.active_version = 1;
    model.storage_fee = storage_fee;
    model.bump = *ctx.bumps.get("model_account").unwrap();

    // Transfer storage fee
    let transfer_ix = system_instruction::transfer(
        ctx.accounts.payer.key,
        &crate::ID,
        storage_fee,
    );

    anchor_lang::solana_program::program::invoke(
        &transfer_ix,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Emit registration event
    emit!(ModelRegistered {
        model_hash,
        owner: model.owner,
        timestamp: model.timestamp,
        fee: storage_fee,
    });

    Ok(())
}

impl ModelAccount {
    pub const LEN: usize = 32   // model_hash
        + 32                    // zk_circuit
        + 32                    // owner
        + 8                     // timestamp
        + 8                     // active_version
        + 8                     // storage_fee
        + 1;                    // bump

    pub fn is_registered(model_hash: &[u8; 32]) -> bool {
        // Implementation would check on-chain state
        // Mocked for example purposes
        false
    }
}
