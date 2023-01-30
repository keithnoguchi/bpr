use anchor_lang::prelude::*;

declare_id!("3LuouAGwBeueVADEviTaKLsgwkrinvfXKCNKPWcmbAQX");

#[program]
pub mod anchor_multisig3 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
