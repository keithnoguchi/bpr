use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod calc {
    use super::*;

    pub fn create(ctx: Context<Create>, greeting: String) -> Result<()> {
        let calc = &mut ctx.accounts.calculator;
        calc.greeting = greeting;
        Ok(())
    }

    pub fn add(ctx: Context<CalcCtx>, a: i64, b: i64) -> Result<()> {
        let calc = &mut ctx.accounts.calculator;
        calc.result = a + b;
        Ok(())
    }

    pub fn sub(ctx: Context<CalcCtx>, a: i64, b: i64) -> Result<()> {
        let calc = &mut ctx.accounts.calculator;
        calc.result = a - b;
        Ok(())
    }

    pub fn mul(ctx: Context<CalcCtx>, a: i64, b: i64) -> Result<()> {
        let calc = &mut ctx.accounts.calculator;
        calc.result = a * b;
        Ok(())
    }

    pub fn div(ctx: Context<CalcCtx>, a: i64, b: i64) -> Result<()> {
        let calc = &mut ctx.accounts.calculator;
        calc.result = a / b;
        let rem = a % b;
        calc.remainder = if rem < 0 { -rem } else { rem };
        Ok(())
    }
}

#[account]
pub struct Calculator {
    pub greeting: String,
    pub result: i64,
    pub remainder: i64,
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init, payer = user, space = 264)] // 256 + 8?
    pub calculator: Account<'info, Calculator>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CalcCtx<'info> {
    #[account(mut)]
    pub calculator: Account<'info, Calculator>,
}
