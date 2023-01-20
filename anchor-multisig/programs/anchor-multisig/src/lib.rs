use anchor_lang::prelude::*;

declare_id!("EYg7btAzuDC6MoYeCN9YzZcWu3T25Xqt7SEhcTbdbnG2");

#[program]
pub mod anchor_multisig {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
