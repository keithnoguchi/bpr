//! A native SOL multisig wallet program.

use std::collections::HashSet;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::system_instruction;

declare_id!("3LuouAGwBeueVADEviTaKLsgwkrinvfXKCNKPWcmbAQX");

#[error_code]
pub enum Error {
    #[msg("Not enough signers given")]
    NotEnoughSigners,

    #[msg("Too many signers given")]
    TooManySigners,

    #[msg("Threshold too high")]
    ThresholdTooHigh,
}

/// A native SOL multisig wallet.
#[account]
pub struct Multisig {
    /// A PDA bump of the account.
    bump: u8,

    /// A threshold.
    m: u8,

    /// A number of signers.
    n: u8,

    /// An array of approved atatus.
    approved: [bool; 5],

    /// An array of signers.
    signers: [Pubkey; 5],

    /// An array of queued transactions.
    txs: Vec<Pubkey>,
}

impl Multisig {
    /// A minimum signers.
    const MIN_SIGNERS: usize = 2;

    /// A maximum signers.
    const MAX_SIGNERS: usize = 5;

    /// A maximum transactions to be queued.
    const MAX_TXS: usize = 100;

    /// A account space.
    const SPACE: usize = 8 + 1 + 1 + 1 + 33 * Self::MAX_SIGNERS + 4 + 32 * Self::MAX_TXS;
}

/// An initiated transfer transaction.
#[account]
pub struct Transfer {
    /// An initiator of the transfer, one of the multisig
    /// signers.
    initiator: Pubkey,

    /// A recipient of the lamports.
    recipient: Pubkey,

    /// A lamports to transfer.
    lamports: u64,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Open<'info> {
    /// A funder of the multisig account.
    #[account(mut)]
    pub funder: Signer<'info>,

    /// A multisig account.
    #[account(
        init,
        payer = funder,
        space = Multisig::SPACE,
        seeds = [b"multisig", funder.key.as_ref()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    /// The system program to create a multisig PDA account.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Fund<'info> {
    /// A funder of the account.
    ///
    /// The funding is only allowed by the multisig account creator.
    #[account(mut)]
    pub funder: Signer<'info>,

    /// A multisig account.
    #[account(mut, seeds = [b"multisig", funder.key.as_ref()], bump)]
    pub multisig: Box<Account<'info, Multisig>>,

    /// The system program to make the transfer of the funds.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitiateTransfer<'info> {
    /// An initiator of the fund transfer.
    ///
    /// It should be one of the signers of the multisig account.
    pub initiator: Signer<'info>,

    /// A multisig account to take fund from.
    pub multisig: Box<Account<'info, Multisig>>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Close<'info> {
    /// An original funder of the multisig account.
    #[account(mut)]
    pub funder: Signer<'info>,

    /// A multisig account.
    #[account(mut, close = funder, seeds = [b"multisig", funder.key.as_ref()], bump)]
    pub multisig: Box<Account<'info, Multisig>>,

    /// The system program to transfer back the fund.
    pub system_program: Program<'info, System>,
}

#[program]
pub mod anchor_multisig3 {
    use super::*;

    pub fn open(ctx: Context<Open>, bump: u8, m: u8, signers: Vec<Pubkey>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let funder = &ctx.accounts.funder;

        // Drops the duplicate signers.
        let mut signers: HashSet<_> = signers.into_iter().collect();

        // Make sure the funder is part of the signers.
        signers.insert(funder.key());

        // Makes sure we have a valid number of sighers,
        // as well as the valid threshold, m <= signers.len().
        require_gte!(
            signers.len(),
            Multisig::MIN_SIGNERS,
            Error::NotEnoughSigners
        );
        require!(
            signers.len() <= Multisig::MAX_SIGNERS,
            Error::TooManySigners
        );
        let threshold = m as usize;
        require_gte!(signers.len(), threshold, Error::ThresholdTooHigh);

        // Initializes the multisig PDA account.
        multisig.bump = bump;
        multisig.m = m;
        multisig.n = signers.len() as u8;
        signers
            .into_iter()
            .enumerate()
            .for_each(|(i, signer)| multisig.signers[i] = signer);

        Ok(())
    }

    pub fn fund(ctx: Context<Fund>, _bump: u8, lamports: u64) -> Result<()> {
        let multisig = &ctx.accounts.multisig;
        let funder = &ctx.accounts.funder;

        // CPI to transfer fund to the multisig account.
        let ix = system_instruction::transfer(&funder.key(), &multisig.key(), lamports);
        let accounts = [funder.to_account_info(), multisig.to_account_info()];
        invoke(&ix, &accounts)?;

        Ok(())
    }

    pub fn close(_ctx: Context<Close>, _bump: u8) -> Result<()> {
        Ok(())
    }
}
