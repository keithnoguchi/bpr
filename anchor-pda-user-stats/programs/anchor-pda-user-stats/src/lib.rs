use anchor_lang::prelude::*;

declare_id!("3EQwhZWFzX1MCUbYpckbErrrLmt5n9PEQdjHUpXc8as7");

#[error_code]
pub enum Error {
    #[msg("User name is too long")]
    NameTooLong,
}

#[program]
pub mod anchor_pda_user_stats {
    use super::*;

    pub fn open(ctx: Context<Open>, name: String) -> Result<()> {
        let user_stats = &mut ctx.accounts.user_stats;

        if name.as_bytes().len() > UserStats::NAME_MAX {
            Err(Error::NameTooLong)?;
        }
        user_stats.name = name;
        user_stats.bump = *ctx.bumps.get("user_stats").unwrap();

        Ok(())
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        let user_stats = &mut ctx.accounts.user_stats;
        let lamports = **user_stats.to_account_info().lamports.borrow();

        // Transfer the lamports back to the user to close the PDA account.
        let user = &mut ctx.accounts.user;
        **user.to_account_info().lamports.borrow_mut() += lamports;
        **user_stats.to_account_info().lamports.borrow_mut() -= lamports;
        Ok(())
    }
}

#[account]
pub struct UserStats {
    /// A user name, 32 bytes max.
    name: String,

    /// A PDA bump.
    bump: u8,
}

impl UserStats {
    /// A space for the UserStats
    const SPACE: usize = Self::DESCRIMINATOR + 4 + Self::NAME_MAX + 1;
    const DESCRIMINATOR: usize = 8;
    const NAME_MAX: usize = 32;
}

#[derive(Accounts)]
pub struct Open<'info> {
    /// A user, who pays for the `UserStats` account.
    #[account(mut)]
    pub user: Signer<'info>,

    /// A `UserStats` PDA account.
    #[account(
        init,
        payer = user,
        space = UserStats::SPACE,
        seeds = [b"user-stats", user.key().as_ref()],
        bump
    )]
    pub user_stats: Account<'info, UserStats>,

    /// SystemProgram to create PDA account.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Close<'info> {
    /// A user account to get the rent back.
    #[account(mut)]
    pub user: Signer<'info>,

    /// A PDA account to be closed.
    #[account(
        mut,
        seeds = [b"user-stats", user.key().as_ref()],
        bump
    )]
    pub user_stats: Account<'info, UserStats>,

    /// SystemProgram to close the PDA account.
    pub system_program: Program<'info, System>,
}
