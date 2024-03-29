//! A multisig program.

use std::collections::HashSet;

use anchor_lang::prelude::*;

declare_id!("6ihHMp67G1RVdkSUC7ZgFccbLA5Ar19hn7wst11RjnQu");

/// Custom errors of the program.
#[error_code]
pub enum Error {
    #[msg("Invalid signer is provided")]
    InvalidSigner,

    #[msg("Invalid transaction is provided")]
    InvalidTransaction,

    #[msg("At least two signers required")]
    NotEnoughSigners,

    #[msg("The threshold, m, is too high for the signers")]
    ThresholdTooHigh,

    #[msg("Too many signers for the multisig account")]
    TooManySigners,

    #[msg("The transaction queue is full")]
    TransactionQueueFull,
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

    /// Current queued transactions.
    tx_queued: u8,

    /// An array of queued transactions.
    txs: [Pubkey; 10], // [Pubkey; Self::MAX_TRANSACTIONS]

    /// An array of signer's Pubkey.
    signers: [Pubkey; 11], // [Pubkey; Self::MAX_SIGNERS]
}

impl Multisig {
    /// A minimum signers required for the account.
    const MIN_SIGNERS: usize = 2;

    /// A maximum signers allowed to managed by the account.
    const MAX_SIGNERS: usize = 11;

    /// A maximum pending transactions.
    const MAX_TRANSACTIONS: usize = 10;

    /// A space of the [`Multisig`] account.
    const SPACE: usize = 8 + 1 + 1 + 1 + 1 + 32 * Self::MAX_SIGNERS + 32 * Self::MAX_TRANSACTIONS;
}

/// A transaction account managed by Multisig account.
#[account]
pub struct Transaction {
    /// A multisig account.
    pub multisig: Pubkey,

    /// Indices of the signers.
    pub signers: [bool; 11],

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
#[instruction(bump: u8)]
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

    /// The SystemProgram to create a multisig PDA account.
    pub system_program: Program<'info, System>,
}

/// Accounts required for the [`anchor_multisig2::enqueue`] instruction to enqueue transaction.
#[derive(Accounts)]
pub struct Enqueue<'info> {
    /// The payer of this enqueue operation.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The multisig account to be enqueued under.
    #[account(mut)]
    pub multisig: Account<'info, Multisig>,

    /// The transaction to be enqueued to the multisig account.
    #[account(zero, signer)]
    pub transaction: Box<Account<'info, Transaction>>,
}

/// Approves the transaction managed under multisig account.
#[derive(Accounts)]
pub struct Approve<'info> {
    /// The approver.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// A multisig account the transaction had been queued.
    #[account(mut)]
    pub multisig: Box<Account<'info, Multisig>>,

    /// A transaction to approve.
    #[account(mut, has_one = multisig)]
    pub transaction: Box<Account<'info, Transaction>>,
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
        close = payer,
        seeds = [b"multisig", payer.key().as_ref()],
        bump
    )]
    pub multisig: Account<'info, Multisig>,

    /// The SystemProgram to transfer the `lamports` back to.
    pub system_program: Program<'info, System>,
}

/// Program instructions.
#[program]
pub mod anchor_multisig2 {
    use super::*;

    /// Creates new Multisig account.
    pub fn open(ctx: Context<Open>, bump: u8, m: u8, signers: Vec<Pubkey>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let payer = &ctx.accounts.payer;

        // Checks duplicate signers.
        let mut signers: HashSet<_> = signers.into_iter().collect();
        signers.insert(payer.key());

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
        multisig.tx_queued = 0;

        Ok(())
    }

    /// Enqueues new Transaction under the Multisig account.
    ///
    /// Once it's approved by [`approve`] instruction, it will
    /// be executed with the required multiple signatures.
    pub fn enqueue(
        ctx: Context<Enqueue>,
        tx_program_id: Pubkey,
        tx_accounts: Vec<TransactionMeta>,
        tx_data: Vec<u8>,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let payer = &ctx.accounts.payer;

        // The payer of the transaction should be one of
        // the Multisig account this transaction belongs to.
        let index = match multisig
            .signers
            .iter()
            .position(|pubkey| *pubkey == payer.key())
        {
            None => return Err(Error::InvalidSigner.into()),
            Some(index) => index,
        };

        // The queue should not be full.
        let tx_queued = multisig.tx_queued as usize;
        require!(
            tx_queued < Multisig::MAX_TRANSACTIONS,
            Error::TransactionQueueFull,
        );

        // Initialize the transaction and enqueue
        // the tx pubkey to multisig account.
        let tx = &mut ctx.accounts.transaction;
        tx.multisig = multisig.key();
        tx.program_id = tx_program_id;
        tx.accounts = tx_accounts;
        tx.data = tx_data;
        tx.signers[index] = true;
        multisig.txs[tx_queued] = tx.key();
        multisig.tx_queued += 1;

        Ok(())
    }

    /// Approves transaction queued in Multisig account.
    pub fn approve(ctx: Context<Approve>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        let payer = &ctx.accounts.payer;

        // The payer of the transaction should be one of
        // the Multisig account this transaction belongs to.
        let index = match multisig
            .signers
            .iter()
            .position(|pubkey| *pubkey == payer.key())
        {
            None => return Err(Error::InvalidSigner.into()),
            Some(index) => index,
        };

        // The transaction should be managed under the
        // multisig account.
        let tx = &mut ctx.accounts.transaction;
        require!(multisig.txs.contains(&tx.key()), Error::InvalidTransaction);

        // Nothing to do if it's already approved by the
        // same signer.
        if tx.signers[index] == true {
            return Ok(());
        }
        tx.signers[index] = true;

        // Counts the signers and executes the transaction
        // if it got the enough signatures.
        let signers = tx.signers.iter().filter(|&signer| *signer).count();
        let threshold = multisig.m as usize;
        if signers < threshold {
            return Ok(());
        }

        Ok(())
    }

    /// Closes the multisig account.
    ///
    /// It requires `m - 1` signers to approve this operation.
    pub fn close(_ctx: Context<Close>) -> Result<()> {
        Ok(())
    }
}
