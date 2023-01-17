//! An anchor counter program.
use anchor_lang::prelude::*;

declare_id!("pXTg1SQB2e2kSAyUhAbYoAL4ubEdYAx6uJmSYMt8wHg");

/// An anchor counter program.
#[program]
pub mod anchor_counter {
    use super::*;

    /// Initialize the counter `State` for the specified address.
    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    /// Increment the counter `State` by one.
    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        ctx.accounts.state.count += 1;
        Ok(())
    }
}

/// An initialization instruction accounts to initialize a
/// counter program `State` data account.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// A state data account of the counter program.
    #[account(init, payer = authority, space = State::SPACE)]
    state: Account<'info, State>,

    /// An authority of the counter `State` account, who
    /// pays the rent and the transaction fees.
    #[account(mut)]
    authority: Signer<'info>,

    /// System program to create a state data account.
    system_program: Program<'info, System>,
}

/// An increment instruction to counts up the `State::count`
/// by one.
#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut)]
    state: Account<'info, State>,
}

/// A state of the counter program.
#[account]
pub struct State {
    /// Keep track of the `increment` instruction calls.
    pub count: u8,
}

impl State {
    /// 8 bytes for anchor and one byte for `count` member.
    const SPACE: usize = 8 + 1;
}
