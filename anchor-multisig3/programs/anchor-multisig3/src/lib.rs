//! A native SOL multisig wallet program.

use std::collections::HashSet;

use anchor_lang::prelude::*;

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
    const MAX_TXS: usize = 10;

    /// A account space.
    const SPACE: usize = 8 + 1 + 1 + 1 + 32 * Self::MAX_SIGNERS + 4 + 32 * Self::MAX_TXS;
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

        // Checks duplicate signers.
        let mut signers: HashSet<_> = signers.into_iter().collect();
        signers.insert(funder.key());

        // Makes sure we have a valid number of sighers,
        // as well as the valid threshold, m <= signers.len().
        require_gte!(
            signers.len(),
            Multisig::MIN_SIGNERS,
            Error::NotEnoughSigners
        );
        require!(signers.len() < Multisig::MAX_SIGNERS, Error::TooManySigners);
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

    pub fn close(_ctx: Context<Close>, _bump: u8) -> Result<()> {
        Ok(())
    }
}
