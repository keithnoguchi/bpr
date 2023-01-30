//! A native SOL multisig wallet program.

use std::collections::HashSet;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::system_instruction;

declare_id!("3LuouAGwBeueVADEviTaKLsgwkrinvfXKCNKPWcmbAQX");

#[error_code]
pub enum Error {
    #[msg("Fund account is not writable")]
    FundAccountNotWritable,

    #[msg("Fund account data is not empty")]
    FundAccountIsNotEmpty,

    #[msg("Invalid fund bump seed")]
    InvalidFundBumpSeed,

    #[msg("Invalid fund account")]
    InvalidFundAddress,

    #[msg("Not enough signers given")]
    NotEnoughSigners,

    #[msg("Too many signers given")]
    TooManySigners,

    #[msg("Threshold too high")]
    ThresholdTooHigh,

    #[msg("Invalid signer")]
    InvalidSigner,

    #[msg("There is not enough fund remains")]
    NotEnoughFund,

    #[msg("Pending transfer is full")]
    TransferQueueFull,
}

/// A multisig data account.
#[account]
pub struct Multisig {
    /// A threshold.
    m: u8,

    /// A number of signers.
    n: u8,

    /// A PDA bump of the account.
    bump: u8,

    /// A fund PDA account bump.
    fund_bump: u8,

    /// A fund account, holding the native SOL.
    multisig_fund: Pubkey,

    /// Remaining fund in lamports.
    remaining_fund: u64,

    /// An array of approved atatus of the signers.
    approved: [bool; 5],

    /// An array of signers Pubkey.
    signers: [Pubkey; 5],

    /// An array of queued transactions.
    transfers: Vec<Pubkey>,
}

impl Multisig {
    /// A minimum signers.
    const MIN_SIGNERS: usize = 2;

    /// A maximum signers.
    const MAX_SIGNERS: usize = 5;

    /// A maximum transfers queued under the multisig.
    const MAX_TXS: usize = 100;

    /// A account space.
    const SPACE: usize =
        8 + 1 + 1 + 1 + 1 + 32 + 8 + 33 * Self::MAX_SIGNERS + 4 + 32 * Self::MAX_TXS;

    /// Validates the multisig fund account.
    fn validate_fund_account<'info>(
        multisig: &Account<'info, Self>,
        fund: &UncheckedAccount<'info>,
        bump: u8,
    ) -> Result<()> {
        if !fund.is_writable {
            Err(Error::FundAccountNotWritable)?;
        }
        if !fund.data_is_empty() {
            Err(Error::FundAccountIsNotEmpty)?;
        }
        let multisig_key = multisig.key();
        let seed = [b"fund", multisig_key.as_ref(), &[bump]];
        let pda = match Pubkey::create_program_address(&seed, &id()) {
            Err(_e) => Err(Error::InvalidFundBumpSeed)?,
            Ok(pda) => pda,
        };
        require_keys_eq!(pda, fund.key(), Error::InvalidFundAddress);

        Ok(())
    }

    /// Creates a fund account.
    fn create_fund_account<'info>(
        multisig: &Account<'info, Self>,
        fund: &UncheckedAccount<'info>,
        funder: &Signer<'info>,
        bump: u8,
    ) -> Result<()> {
        let lamports = Rent::get()?.minimum_balance(0);
        let ix = system_instruction::create_account(&funder.key(), &fund.key(), lamports, 0, &id());
        let multisig_key = multisig.key();
        let accounts = [funder.to_account_info(), fund.to_account_info()];
        let seed = [b"fund", multisig_key.as_ref(), &[bump]];

        // CPI.
        invoke_signed(&ix, &accounts, &[&seed])?;

        Ok(())
    }

    /// Withdraw fund.
    fn withdraw_fund<'info>(
        _multisig: &Account<'info, Self>,
        from: &mut AccountInfo<'info>,
        to: &mut AccountInfo<'info>,
        lamports: u64,
        _bump: u8,
    ) -> Result<()> {
        // The following code hit the runtime error, [`InstructionError::ExternalLamportSpend`].
        // Instead, we'll transfer the lamports natively,
        // as suggested by the [Solana cookbook].
        //
        // [`InstructionError::ExternalLamportSpend`]: https://docs.rs/solana-program/latest/solana_program/instruction/enum.InstructionError.html#variant.ExternalAccountLamportSpend
        // [solana cookbook]: https://solanacookbook.com/references/programs.html#how-to-transfer-sol-in-a-program

        /*
        let ix = system_instruction::transfer(from.key, &to.key, lamports);
        let accounts = [from, to];
        let multisig_key = multisig.key();
        let seed = [b"fund", multisig_key.as_ref(), &[bump]];
        invoke_signed(
            &ix,
            &accounts,
            &[&seed],
        )?;
        */
        **from.try_borrow_mut_lamports()? -= lamports;
        **to.try_borrow_mut_lamports()? += lamports;

        Ok(())
    }
}

/// A transfer transaction queued under the Multisig account.
#[account]
pub struct Transfer {
    /// An creator of the transfer, one of the multisig
    /// signers.
    creator: Pubkey,

    /// A recipient of the transfer.
    recipient: Pubkey,

    /// A lamports to transfer.
    lamports: u64,
}

impl Transfer {
    const SPACE: usize = 8 + 32 + 32 + 8;
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Create<'info> {
    /// A funder of the multisig account.
    #[account(mut)]
    pub funder: Signer<'info>,

    /// A multisig state account.
    #[account(
        init,
        payer = funder,
        space = Multisig::SPACE,
        seeds = [b"multisig", funder.key.as_ref()],
        bump,
    )]
    pub multisig: Account<'info, Multisig>,

    /// A multisig fund account.
    ///
    /// CHECK: Checked by the handler.
    #[account(mut)]
    pub multisig_fund: UncheckedAccount<'info>,

    /// The system program to create a multisig PDA accounts.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Fund<'info> {
    /// A funder of the account.
    ///
    /// The funding is only allowed by the multisig account creator.
    #[account(mut)]
    pub funder: Signer<'info>,

    /// A multisig state account.
    #[account(mut, seeds = [b"multisig", funder.key.as_ref()], bump)]
    pub multisig: Box<Account<'info, Multisig>>,

    /// A multisig fund account.
    ///
    /// CHECK: Checked by the handler.
    #[account(mut)]
    pub multisig_fund: UncheckedAccount<'info>,

    /// The system program to make the transfer of the funds.
    pub system_program: Program<'info, System>,
}

/// Create and queue the new transfer under the multisig account.
#[derive(Accounts)]
pub struct CreateTransfer<'info> {
    /// An initiator of the fund transfer.
    ///
    /// It should be one of the signers of the multisig account.
    #[account(mut)]
    pub creator: Signer<'info>,

    /// A multisig state account.
    #[account(mut)]
    pub multisig: Box<Account<'info, Multisig>>,

    /// A multisig fund account.
    ///
    /// CHECK: Checked by the handler.
    #[account(mut)]
    pub multisig_fund: UncheckedAccount<'info>,

    /// A transfer account to keep the queued transfer info.
    #[account(init, payer = creator, space = Transfer::SPACE)]
    pub transfer: Account<'info, Transfer>,

    /// The system program to create a transfer account.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Close<'info> {
    /// An original funder of the multisig account.
    #[account(mut)]
    pub funder: Signer<'info>,

    /// A multisig account.
    #[account(mut, close = funder, seeds = [b"multisig", funder.key.as_ref()], bump)]
    pub multisig: Box<Account<'info, Multisig>>,

    /// A multisig fund account.
    ///
    /// CHECK: Checked by the handler.
    #[account(mut)]
    pub multisig_fund: UncheckedAccount<'info>,

    /// The system program to transfer back the fund.
    pub system_program: Program<'info, System>,
}

#[program]
pub mod anchor_multisig3 {
    use super::*;

    /// Creates the multisig account.
    ///
    /// It's restricted one multisig account to each funder Pubkey,
    /// as it's used for the multisig PDA address generation.
    pub fn create(
        ctx: Context<Create>,
        m: u8,
        signers: Vec<Pubkey>,
        bump: u8,
        fund_bump: u8,
    ) -> Result<()> {
        let funder = &mut ctx.accounts.funder;
        let multisig = &mut ctx.accounts.multisig;
        let multisig_fund = &mut ctx.accounts.multisig_fund;

        // Validate the multisig fund account.
        Multisig::validate_fund_account(&multisig, &multisig_fund, fund_bump)?;

        // Checks the signers.
        let mut signers: HashSet<_> = signers.into_iter().collect();
        signers.insert(funder.key());
        require_gte!(
            signers.len(),
            Multisig::MIN_SIGNERS,
            Error::NotEnoughSigners
        );
        require!(
            signers.len() <= Multisig::MAX_SIGNERS,
            Error::TooManySigners
        );
        let threshold = m as usize;
        require_gte!(signers.len(), threshold, Error::ThresholdTooHigh);

        // Creates a fund account.
        Multisig::create_fund_account(&multisig, &multisig_fund, &funder, fund_bump)?;

        // Initializes the multisig state account.
        multisig.m = m;
        multisig.n = signers.len() as u8;
        multisig.bump = bump;
        multisig.fund_bump = fund_bump;
        multisig.multisig_fund = multisig_fund.key();
        multisig.remaining_fund = 0;
        multisig
            .approved
            .iter_mut()
            .for_each(|approved| *approved = false);
        signers.into_iter().enumerate().for_each(|(index, signer)| {
            multisig.signers[index] = signer;
        });

        Ok(())
    }

    /// Funds lamports to the multisig account.
    ///
    /// The funding is only allowed to the multisig funder.
    pub fn fund(ctx: Context<Fund>, lamports: u64, _bump: u8, fund_bump: u8) -> Result<()> {
        let funder = &ctx.accounts.funder;
        let multisig = &mut ctx.accounts.multisig;
        let multisig_fund = &mut ctx.accounts.multisig_fund;

        // Validate the multisig fund account.
        Multisig::validate_fund_account(&multisig, &multisig_fund, fund_bump)?;

        // CPI to transfer fund to the multisig fund account.
        let ix = system_instruction::transfer(&funder.key(), &multisig_fund.key(), lamports);
        let accounts = [funder.to_account_info(), multisig_fund.to_account_info()];
        invoke(&ix, &accounts)?;

        // Update the remaining fund.
        multisig.remaining_fund += lamports;

        Ok(())
    }

    pub fn create_transfer(
        ctx: Context<CreateTransfer>,
        recipient: Pubkey,
        lamports: u64,
        fund_bump: u8,
    ) -> Result<()> {
        let creator = &ctx.accounts.creator;
        let multisig = &mut ctx.accounts.multisig;
        let multisig_fund = &mut ctx.accounts.multisig_fund;
        let transfer = &mut ctx.accounts.transfer;

        // Validate the multisig fund account.
        Multisig::validate_fund_account(&multisig, &multisig_fund, fund_bump)?;

        // Checks the creator validity.
        let creator_key = creator.key();
        let signers = &multisig.signers[..multisig.n as usize];
        require!(signers.contains(&creator_key), Error::InvalidSigner);

        // Check the current transfer queue.
        require_gt!(
            Multisig::MAX_TXS,
            multisig.transfers.len(),
            Error::TransferQueueFull
        );

        // Checks the multisig fund balance.
        require_gte!(multisig.remaining_fund, lamports, Error::NotEnoughFund);

        // Giving back the rent fee to the creator.
        let rent = transfer.to_account_info().lamports();
        Multisig::withdraw_fund(
            &multisig,
            &mut multisig_fund.to_account_info(),
            &mut creator.to_account_info(),
            rent,
            fund_bump,
        )?;

        // Initializes the transfer account, and
        // queue it under multisig account for the
        // future transfer execution.
        transfer.creator = creator_key;
        transfer.recipient = recipient;
        transfer.lamports = lamports;
        multisig.transfers.push(transfer.key());
        multisig.remaining_fund -= lamports;

        Ok(())
    }

    pub fn close(ctx: Context<Close>, _bump: u8, fund_bump: u8) -> Result<()> {
        let funder = &mut ctx.accounts.funder;
        let multisig = &mut ctx.accounts.multisig;
        let multisig_fund = &mut ctx.accounts.multisig_fund;

        // Validate the multisig fund account.
        Multisig::validate_fund_account(&multisig, &multisig_fund, fund_bump)?;

        // Close the multisig fund account by transfering all the lamports
        // back to the funder.
        let lamports = multisig_fund.lamports();
        Multisig::withdraw_fund(
            &multisig,
            &mut multisig_fund.to_account_info(),
            &mut funder.to_account_info(),
            lamports,
            fund_bump,
        )?;

        Ok(())
    }
}
