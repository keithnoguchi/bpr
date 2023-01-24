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
use anchor_lang::prelude::*;

declare_id!("EYg7btAzuDC6MoYeCN9YzZcWu3T25Xqt7SEhcTbdbnG2");

#[error_code]
pub enum Error {
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,
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
        tx_accounts: Vec<TransactionInfo>,
        tx_data: Vec<u8>,
    ) -> Result<()> {
        let owner_index = ctx
            .accounts
            .multisig
            .owners
            .iter()
            .position(|a| a == ctx.accounts.proposer.key)
            .ok_or(Error::InvalidOwner)?;

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
pub struct TransactionInfo {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
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
    pub accounts: Vec<TransactionInfo>,

    /// Instruction data for the transaction.
    pub data: Vec<u8>,

    /// Signers[index] is true if multisig.owners[index] signed
    /// the transaction.
    pub signers: Vec<bool>,

    /// Boolean ensuring one time execution.
    pub did_executed: bool,

    /// Owner set sequence number.
    pub owner_set_seqno: u32,
}
