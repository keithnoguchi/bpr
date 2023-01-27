use anchor_lang::prelude::*;

declare_id!("3EQwhZWFzX1MCUbYpckbErrrLmt5n9PEQdjHUpXc8as7");

#[program]
pub mod anchor_pda_user_stats {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
