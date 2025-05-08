// contracts/programs/model_registry/src/lib.rs

#![cfg_attr(not(feature = "anchor-attributes"), forbid(unsafe_code))]
#![cfg_attr(feature = "anchor-attributes", allow(unused_attributes))]

use anchor_lang::prelude::*;
use solana_program::{entrypoint::ProgramResult, system_instruction};
use crate::{instructions::*, state::*, error::ModelRegistryError, utils::crypto};

declare_id!("SCRAxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");

#[program]
pub mod model_registry {
    use super::*;

    /// Initialize program first admin
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let admin = &mut ctx.accounts.admin;
        admin.authority = *ctx.accounts.payer.key;
        admin.bump = *ctx.bumps.get("admin").unwrap();
        Ok(())
    }

    /// Register new AI model (admin only)
    pub fn register_model(
        ctx: Context<RegisterModel>,
        model_hash: [u8; 32],
        zk_circuit_hash: [u8; 32],
        fee: u64,
    ) -> Result<()> {
        require!(ctx.accounts.admin.is_admin(), ModelRegistryError::Unauthorized);
        
        let model_account = &mut ctx.accounts.model_account;
        model_account.model_hash = model_hash;
        model_account.zk_circuit = zk_circuit_hash;
        model_account.timestamp = Clock::get()?.unix_timestamp;
        model_account.fee = fee;

        // Transfer storage fee
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                ctx.accounts.payer.key,
                &crate::ID,
                fee,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        emit!(ModelRegistered {
            model_hash,
            owner: *ctx.accounts.payer.key,
            timestamp: model_account.timestamp,
        });

        Ok(())
    }

    /// Submit inference request with ZKP
    pub fn request_inference(
        ctx: Context<RequestInference>,
        input_hash: [u8; 32],
        zk_proof: Vec<u8>,
    ) -> Result<()> {
        let model = &ctx.accounts.model_account;
        
        // Verify ZKP matches circuit
        require!(
            crypto::verify_zk_proof(
                &model.zk_circuit,
                &input_hash,
                &zk_proof
            ),
            ModelRegistryError::InvalidProof
        );

        let request = &mut ctx.accounts.inference_request;
        request.model = model.key();
        request.input_hash = input_hash;
        request.status = InferenceStatus::Pending;

        emit!(InferenceRequested {
            model: model.key(),
            request: request.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    /// Contribute data to federated learning pool
    pub fn contribute_data(
        ctx: Context<ContributeData>,
        encrypted_data: Vec<u8>,
        data_hash: [u8; 32],
    ) -> Result<()> {
        let contribution = &mut ctx.accounts.contribution_account;
        contribution.data = encrypted_data.clone();
        contribution.hash = data_hash;
        contribution.contributor = *ctx.accounts.contributor.key;
        contribution.timestamp = Clock::get()?.unix_timestamp;

        require!(
            contribution.data.len() <= MAX_CONTRIBUTION_SIZE,
            ModelRegistryError::StorageExceeded
        );

        emit!(DataContributed {
            contributor: contribution.contributor,
            data_hash,
            model: ctx.accounts.model_account.key(),
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + AdminAccount::LEN,
        seeds = [b"admin"],
        bump
    )]
    pub admin: Account<'info, AdminAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Default)]
pub struct AdminAccount {
    pub authority: Pubkey,
    pub bump: u8,
}

impl AdminAccount {
    pub const LEN: usize = 32 + 1;
    
    pub fn is_admin(&self, user: &Pubkey) -> bool {
        self.authority == *user
    }
}

// Events
#[event]
pub struct ModelRegistered {
    pub model_hash: [u8; 32],
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct InferenceRequested {
    pub model: Pubkey,
    pub request: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct DataContributed {
    pub contributor: Pubkey,
    pub data_hash: [u8; 32],
    pub model: Pubkey,
}

// Error codes
#[error_code]
pub enum ModelRegistryError {
    #[msg("Unauthorized access attempt")]
    Unauthorized,
    #[msg("Invalid zero-knowledge proof")]
    InvalidProof,
    #[msg("Model storage limit exceeded")]
    StorageExceeded,
    #[msg("Invalid cryptographic hash")]
    InvalidHash,
    #[msg("Insufficient storage fee")]
    InsufficientFee,
    #[msg("Invalid model version")]
    VersionMismatch,
}
