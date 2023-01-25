//! An anchor Multisig program.
//!
//! It's demonstrated in [coral-xyz] as well as the
//! [anchor tests].
//!
//! Those two files are almost identical, except that
//! the exported one, not the one under [anchor test],
//! has an added functionality, e.g. unique owner check.
//!
//! This example follows, e.g. steals, ;), the exported one.
//!
//! [coral-xyz]: https://github.com/coral-xyz/multisig/blob/master/programs/multisig/src/lib.rs
//! [anchor tests]: https://github.com/coral-xyz/anchor/blob/master/tests/multisig/programs/multisig/src/lib.rs

use std::ops::Deref;

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::instruction::Instruction;

declare_id!("EYg7btAzuDC6MoYeCN9YzZcWu3T25Xqt7SEhcTbdbnG2");

#[error_code]
pub enum Error {
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,

    #[msg("The transaction had been already executed.")]
    AlreadyExecuted,

    #[msg("There is not enough signers approved.")]
    NotEnoughSigners,
}

#[program]
pub mod anchor_multisig {
    use super::*;

    pub fn initialize_multisig(
        ctx: Context<InitializeMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
        bump: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;

        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.bump = bump;
        multisig.owner_set_seqno = 0;

        Ok(())
    }

    pub fn initialize_transaction(
        ctx: Context<InitializeTransaction>,
        tx_program_id: Pubkey,
        tx_accounts: Vec<TransactionMeta>,
        tx_data: Vec<u8>,
    ) -> Result<()> {
        // Signers vector, set `true` for the proposer.
        let signers: Vec<_> = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .map(|key| key == ctx.accounts.proposer.key)
            .collect();

        // Initialize the transaction account.
        let tx = &mut ctx.accounts.transaction;
        tx.program_id = tx_program_id;
        tx.accounts = tx_accounts;
        tx.data = tx_data;
        tx.signers = signers;
        tx.multisig = ctx.accounts.multisig.key();
        tx.executed = false;
        tx.owner_set_seqno = ctx.accounts.multisig.owner_set_seqno;

        Ok(())
    }

    pub fn approve_transaction(ctx: Context<ApproveTransaction>) -> Result<()> {
        if ctx.accounts.transaction.executed {
            return Err(Error::AlreadyExecuted.into());
        }

        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.owner.key)
            .ok_or(Error::InvalidOwner)?;

        ctx.accounts.transaction.signers[owner_index] = true;

        Ok(())
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        if ctx.accounts.transaction.executed {
            return Err(Error::AlreadyExecuted.into());
        }

        // check if we have enough approvers.
        let approved = ctx
            .accounts
            .transaction
            .signers
            .iter()
            .filter(|&approved| *approved)
            .count() as u64;
        if approved < ctx.accounts.multisig.threshold {
            return Err(Error::NotEnoughSigners.into());
        }

        // Execute the transaction signed by the multisig.
        let ix: Instruction = (*ctx.accounts.transaction).deref().into();
        let multisig_key = ctx.accounts.multisig.key();
        let seeds = &[multisig_key.as_ref(), &[ctx.accounts.multisig.bump]];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;

        solana_program::program::invoke_signed(&ix, accounts, signer)?;

        ctx.accounts.transaction.executed = true;

        Ok(())
    }

    pub fn set_owners(ctx: Context<Auth>, owners: Vec<Pubkey>) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;

        let owners_len = owners.len() as u64;
        if owners_len < multisig.threshold {
            multisig.threshold = owners_len;
        }
        multisig.owners = owners;
        multisig.owner_set_seqno += 1;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeMultisig<'info> {
    #[account(zero, signer)]
    multisig: Box<Account<'info, Multisig>>,
}

#[derive(Accounts)]
pub struct InitializeTransaction<'info> {
    /// A multisig account this transaction is under.
    multisig: Box<Account<'info, Multisig>>,

    /// A transaction account to be executed in the future.
    #[account(zero, signer)]
    transaction: Box<Account<'info, Transaction>>,

    /// One of the owners of the multisig account.
    proposer: Signer<'info>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TransactionMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<&TransactionMeta> for AccountMeta {
    fn from(tx: &TransactionMeta) -> Self {
        if tx.is_writable {
            Self::new(tx.pubkey, tx.is_signer)
        } else {
            Self::new_readonly(tx.pubkey, tx.is_signer)
        }
    }
}

#[derive(Accounts)]
pub struct ApproveTransaction<'info> {
    /// A multisig account to manage the transaction to.
    #[account(constraint = multisig.owner_set_seqno == transaction.owner_set_seqno)]
    multisig: Box<Account<'info, Multisig>>,

    /// A transaction to approve.
    #[account(mut, has_one = multisig)]
    transaction: Box<Account<'info, Transaction>>,

    /// One of the multisig owners.
    owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(constraint = multisig.owner_set_seqno == transaction.owner_set_seqno)]
    multisig: Box<Account<'info, Multisig>>,

    /// CHECK: multisig_signer is a PDA program signer.  Data is never read or written to.
    #[account(seeds = [multisig.key().as_ref()], bump = multisig.bump)]
    multisig_signer: UncheckedAccount<'info>,

    #[account(mut, has_one = multisig)]
    transaction: Box<Account<'info, Transaction>>,
}

#[derive(Accounts)]
pub struct Auth<'info> {
    #[account(mut)]
    multisig: Box<Account<'info, Multisig>>,
    #[account(seeds = [multisig.key().as_ref()], bump = multisig.bump)]
    multisig_signer: Signer<'info>,
}

#[account]
#[derive(Debug, Default)]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub bump: u8,
    pub owner_set_seqno: u32,
}

/// Transaction account, maintained by the `Multisig` account.
#[account]
pub struct Transaction {
    /// A multisig account this transaction belongs to.
    pub multisig: Pubkey,

    /// A target program ID to execute against.
    pub program_id: Pubkey,

    /// Accounts required for the transaction.
    pub accounts: Vec<TransactionMeta>,

    /// Instruction data for the transaction.
    pub data: Vec<u8>,

    /// Signers[index] is true if multisig.owners[index] signed
    /// the transaction.
    pub signers: Vec<bool>,

    /// Boolean ensuring one time execution.
    pub executed: bool,

    /// Owner set sequence number.
    pub owner_set_seqno: u32,
}

impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Self {
        Self {
            program_id: tx.program_id,
            accounts: tx.accounts.iter().map(From::from).collect(),
            data: tx.data.clone(),
        }
    }
}
