// governance/src/vote.rs

use anchor_lang::prelude::*;
use solana_program::{
    program_memory::sol_memcmp,
    pubkey::PUBKEY_BYTES
};
use crate::{
    crypto::{verify_schnorr, SchnorrSignature},
    state::{Proposal, VoteRecord, VotingConfig},
    error::GovernanceError
};

const MAX_CHOICES: usize = 8;
const VOTE_EXPIRATION: i64 = 604_800; // 7 days

#[derive(Accounts)]
#[instruction(proposal_id: u64, choices: Vec<String>)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = Proposal::LEN + (choices.len() * 32),
        seeds = [b"proposal", &proposal_id.to_le_bytes()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    #[account(
        seeds = [b"config"],
        bump = config.bump,
        constraint = config.voting_enabled @ GovernanceError::VotingDisabled
    )]
    pub config: Account<'info, VotingConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateProposal<'info> {
    pub fn create(
        &mut self,
        proposal_id: u64,
        title: String,
        description_hash: [u8; 32], // IPFS CID
        choices: Vec<String>,
        start_time: i64,
        end_time: i64,
    ) -> Result<()> {
        require!(
            choices.len() <= MAX_CHOICES,
            GovernanceError::TooManyChoices
        );
        
        self.proposal.set_inner(Proposal {
            id: proposal_id,
            author: self.authority.key(),
            title,
            description_hash,
            choices,
            start_time,
            end_time,
            total_votes: 0,
            votes_per_choice: vec![0; choices.len()],
            bump: self.proposal.bump,
            created_at: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(vote_choice: u8)]
pub struct CastVote<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    #[account(
        init,
        payer = voter,
        space = VoteRecord::LEN,
        seeds = [b"vote", proposal.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,
    #[account(mut)]
    pub voter: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CastVote<'info> {
    pub fn vote(
        &mut self,
        vote_choice: u8,
        weight: u64,
        proof: SchnorrSignature,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let proposal = &mut self.proposal;
        
        require!(
            clock.unix_timestamp >= proposal.start_time &&
            clock.unix_timestamp <= proposal.end_time,
            GovernanceError::VotingClosed
        );
        
        require!(
            (vote_choice as usize) < proposal.choices.len(),
            GovernanceError::InvalidChoice
        );
        
        verify_schnorr(
            &self.voter.key().to_bytes(),
            &proof,
            &proposal.key().to_bytes(),
        )?;

        let vote = &mut self.vote_record;
        vote.set_inner(VoteRecord {
            voter: self.voter.key(),
            proposal: proposal.key(),
            choice: vote_choice,
            weight,
            cast_at: clock.unix_timestamp,
            bump: self.vote_record.bump,
        });

        proposal.total_votes = proposal.total_votes
            .checked_add(weight)
            .ok_or(GovernanceError::ArithmeticOverflow)?;
            
        proposal.votes_per_choice[vote_choice as usize] = proposal
            .votes_per_choice[vote_choice as usize]
            .checked_add(weight)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct Proposal {
    pub id: u64,
    pub author: Pubkey,
    pub title: String,
    pub description_hash: [u8; 32],
    pub choices: Vec<String>,
    pub start_time: i64,
    pub end_time: i64,
    pub total_votes: u64,
    pub votes_per_choice: Vec<u64>,
    pub bump: u8,
    pub created_at: i64,
}

#[account]
#[derive(Default)]
pub struct VoteRecord {
    pub voter: Pubkey,
    pub proposal: Pubkey,
    pub choice: u8,
    pub weight: u64,
    pub cast_at: i64,
    pub bump: u8,
}

#[error_code]
pub enum GovernanceError {
    #[msg("Voting system disabled")]
    VotingDisabled,
    #[msg("Too many choices in proposal")]
    TooManyChoices,
    #[msg("Voting period has ended")]
    VotingClosed,
    #[msg("Invalid choice selected")]
    InvalidChoice,
    #[msg("Arithmetic overflow detected")]
    ArithmeticOverflow,
    #[msg("Invalid cryptographic proof")]
    InvalidProof,
}
