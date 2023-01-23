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

#[program]
pub mod anchor_multisig {
    use super::*;

    pub fn initialize_multisig(
        ctx: Context<InitializeMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
        nonce: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.nonce = nonce;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeMultisig<'info> {
    #[account(zero, signer)]
    multisig: Box<Account<'info, Multisig>>,
}

#[account]
pub struct Multisig {
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub nonce: u8,
    pub owner_set_seqno: u32,
}
