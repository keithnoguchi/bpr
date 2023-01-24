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
        bump: u8,
    ) -> Result<()> {
        let multisig = &mut ctx.accounts.multisig;
        multisig.owners = owners;
        multisig.threshold = threshold;
        multisig.bump = bump;
        multisig.owner_set_seqno = 0;
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
