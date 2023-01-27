//! A multisig program.

use anchor_lang::prelude::*;

declare_id!("6ihHMp67G1RVdkSUC7ZgFccbLA5Ar19hn7wst11RjnQu");

/// Custom errors of the program.
#[error_code]
pub enum Error {
    #[msg("Exceeding the maximum number of signers")]
    TooManySigners,
}

/// A Multisig PDA account.
#[account]
pub struct Multisig {
    /// PDA bump of the account.
    bump: u8,

    /// threshold, e.g. `m` of `m/n` Multisig.
    m: u8,

    /// Number of signers in `signers` array.
    n: u8,

    /// [`Pubkey`] of the signers, representing
    /// `n` part of `m/n` multisig.
    ///
    /// There is an anchor IDL issue to parse the const
    /// value, e.g. Self::Multisig below.
    ///
    /// Until it's fixed/handled, the actual value below.
    signers: [Pubkey; 11], // [Pubkey; Self::MAX_SIGNERS]

    /// Pubkeys of the pending transactions.
    txs: [Pubkey; 10], // [Pubkey; Self::MAX_TRANSACTIONS]
}

impl Multisig {
    /// A maximum signers allowed to managed by the account.
    const MAX_SIGNERS: usize = 11;

    /// A maximum pending transactions.
    const MAX_TRANSACTIONS: usize = 10;

    /// A space of the [`Multisig`] account.
    const SPACE: usize = 8 + 1 + 1 + 1
        + 32 * Self::MAX_SIGNERS
        + 32 * Self::MAX_TRANSACTIONS;
}

/// A Transaction PDA account.
#[account]
pub struct Transaction {
    /// A target program ID.
    pub program_id: Pubkey,

    /// Accounts for the the transaction.
    pub accounts: Vec<TransactionMeta>,

    /// An instruction data.
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct TransactionMeta {
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}

impl From<TransactionMeta> for AccountMeta {
    fn from(meta: TransactionMeta) -> Self {
        Self {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        }
    }
}

/// Accounts required for the [`anchor_multisig2::open`] instruction.
#[derive(Accounts)]
pub struct Open<'info> {
    /// A [`Multisig`] account payer, as well as the signer
    /// of the [`Transaction`]s.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// A [`Multisig`] account.
    #[account(
        init,
        payer = payer,
        space = Multisig::SPACE,
        seeds = [b"multisig", payer.key.as_ref()],
        bump
    )]
    pub multisig: Account<'info, Multisig>,

    /// The SystemProgram to create a PDA account.
    pub system_program: Program<'info, System>,
}

/// Accounts required for the [`anchor_multisig2::close`] instruction.
#[derive(Accounts)]
pub struct Close<'info> {
    /// The original payer of the [`Multisig`] account.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The [`Multisig`] account to be closed.
    #[account(
        mut,
        seeds = [b"multisig", payer.key().as_ref()],
        bump
    )]
    pub multisig: Account<'info, Multisig>,

    /// The SystemProgram to transfer the `lamports`.
    pub system_program: Program<'info, System>,
}

#[program]
pub mod anchor_multisig2 {
    use super::*;

    pub fn open(ctx: Context<Open>, m: u8, signers: Vec<Pubkey>) -> Result<()> {
        // The signers should be below the [`Multisig::MAX_SIGNERS`]
        // as the payer is also added to the signers.
        require!(signers.len() < Multisig::MAX_SIGNERS, Error::TooManySigners);

        // Initializes the multisig PDA account.
        let multisig = &mut ctx.accounts.multisig;
        multisig.m = m;
        multisig.n = signers.len() as u8 + 1;
        multisig.signers[0] = *ctx.accounts.payer.key;
        signers
            .into_iter()
            .enumerate()
            .for_each(|(i, signers)| multisig.signers[i + 1] = signers);
        multisig.bump = *ctx.bumps.get("multisig").unwrap();

        Ok(())
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let lamports = multisig.to_account_info().lamports();

        // transfer the lamports to close the [`Multisig`] PDA account.
        let payer = &mut ctx.accounts.payer;
        **payer.to_account_info().lamports.borrow_mut() += lamports;
        **multisig.to_account_info().lamports.borrow_mut() -= lamports;

        Ok(())
    }
}
