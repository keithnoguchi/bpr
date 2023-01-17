use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

use crate::error::EscrowError;
use crate::instruction::Instruction;
use crate::state::Escrow;

/// A processor to handle the incoming transactions.
pub(crate) struct Processor;

impl Processor {
    /// Process the instruction.
    ///
    /// This is the entry point of all the transactions to this program.
    pub(crate) fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        match Instruction::unpack(instruction_data)? {
            Instruction::InitEscrow { amount } => {
                Self::process_init_escrow(accounts, amount, program_id)
            }
            Instruction::Exchange { amount } => {
                Self::process_exchange(accounts, amount, program_id)
            }
        }
    }

    /// `Instruction::InitEscrow` processor.
    fn process_init_escrow(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let initializer = next_account_info(accounts_iter)?;
        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // temp_token_account owner will be checked when we
        // transfer the token to the escrow account below.
        let temp_token_account = next_account_info(accounts_iter)?;
        let token_to_receive_account = next_account_info(accounts_iter)?;
        if *token_to_receive_account.owner != spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let escrow_account = next_account_info(accounts_iter)?;
        let rent = &Rent::from_account_info(next_account_info(accounts_iter)?)?;
        if !rent.is_exempt(escrow_account.lamports(), escrow_account.data_len()) {
            return Err(EscrowError::NotRentExempt.into());
        }

        // Populates the escrow account as PDA.
        let mut escrow_info = Escrow::unpack_unchecked(&escrow_account.try_borrow_data()?)?;
        if escrow_info.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        escrow_info.is_initialized = true;
        escrow_info.initializer_pubkey = *initializer.key;
        escrow_info.temp_token_account_pubkey = *temp_token_account.key;
        escrow_info.initializer_token_to_receive_account_pubkey = *token_to_receive_account.key;
        escrow_info.expected_amount = amount;

        Escrow::pack(escrow_info, &mut escrow_account.try_borrow_mut_data()?)?;

        // Gets the [PDA]'s address.
        //
        // [pda]: https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/#pdas-part-2
        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"escrow"], program_id);

        msg!("escrow PDA address: {}", pda);

        // Transfers the temp token's authority from initializer to PDA,
        // as the example of [CPI].
        //
        // [cpi]: https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/#cpis-part-1
        let token_program = next_account_info(accounts_iter)?;
        let token_authority_change_ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_token_account.key,
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            initializer.key,
            &[initializer.key],
        )?;
        invoke(
            &token_authority_change_ix,
            &[
                temp_token_account.clone(),
                initializer.clone(),
                token_program.clone(),
            ],
        )?;

        Ok(())
    }

    /// `Instruction::Exchnage` processor.
    fn process_exchange(
        accounts: &[AccountInfo],
        amount_expected_by_taker: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        // Preparing the taker side, e.g. Bob, of the accounts.
        let taker = next_account_info(accounts_iter)?;
        if !taker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let takers_sending_token_account = next_account_info(accounts_iter)?;
        let takers_token_to_receive_account = next_account_info(accounts_iter)?;

        // Making sure the temp token account holds the exact same amount
        // as the taker's expected value.
        let pdas_temp_token_account = next_account_info(accounts_iter)?;
        let pdas_temp_token_account_info =
            spl_token::state::Account::unpack(&pdas_temp_token_account.try_borrow_data()?)?;
        if pdas_temp_token_account_info.amount != amount_expected_by_taker {
            return Err(EscrowError::ExpectedAmountMismatch.into());
        }

        // Making sure the account info matches to the escrow state.
        let initializers_main_account = next_account_info(accounts_iter)?;
        let initializers_token_to_receive_account = next_account_info(accounts_iter)?;
        let escrow_account = next_account_info(accounts_iter)?;
        let escrow_info = Escrow::unpack(&escrow_account.try_borrow_data()?)?;
        if *pdas_temp_token_account.key != escrow_info.temp_token_account_pubkey {
            return Err(ProgramError::InvalidAccountData);
        }
        if *initializers_main_account.key != escrow_info.initializer_pubkey {
            return Err(ProgramError::InvalidAccountData);
        }
        if *initializers_token_to_receive_account.key
            != escrow_info.initializer_token_to_receive_account_pubkey
        {
            return Err(ProgramError::InvalidAccountData);
        }

        // Initiates CPI to [transfer] Y tokens from Bob to Alice.
        //
        // [transfer]: https://docs.rs/spl-token/latest/spl_token/instruction/fn.transfer.html
        let token_program = next_account_info(accounts_iter)?;
        let transfer_to_initializer_ix = spl_token::instruction::transfer(
            token_program.key,
            takers_sending_token_account.key,
            initializers_token_to_receive_account.key,
            taker.key,
            &[taker.key],
            escrow_info.expected_amount,
        )?;
        invoke(
            &transfer_to_initializer_ix,
            &[
                takers_sending_token_account.clone(),
                initializers_token_to_receive_account.clone(),
                taker.clone(),
                token_program.clone(),
            ],
        )?;

        // Initiates CPI to [transfer] X tokens from Alice, escrow, to Bob.
        //
        // [transfer]: https://docs.rs/spl-token/latest/spl_token/instruction/fn.transfer.html
        let pda_account = next_account_info(accounts_iter)?;
        let (pda, bump) = Pubkey::find_program_address(&[b"escrow"], program_id);
        let transfer_to_taker_ix = spl_token::instruction::transfer(
            token_program.key,
            pdas_temp_token_account.key,
            takers_token_to_receive_account.key,
            &pda,
            &[&pda],
            pdas_temp_token_account_info.amount,
        )?;
        invoke_signed(
            &transfer_to_taker_ix,
            &[
                pdas_temp_token_account.clone(),
                takers_token_to_receive_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"escrow"[..], &[bump]]],
        )?;

        // Close the temporary PDA account.
        let close_pdas_temp_acct_ix = spl_token::instruction::close_account(
            token_program.key,
            pdas_temp_token_account.key,
            initializers_main_account.key,
            &pda,
            &[&pda],
        )?;
        invoke_signed(
            &close_pdas_temp_acct_ix,
            &[
                pdas_temp_token_account.clone(),
                initializers_main_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"escrow"[..], &[bump]]],
        )?;

        // Finally, close the escrow state account and retrun
        // back the rent lamports back to the initializer.
        **initializers_main_account.try_borrow_mut_lamports()? = initializers_main_account
            .lamports()
            .checked_add(escrow_account.lamports())
            .ok_or(EscrowError::AmountOverflow)?;
        **escrow_account.try_borrow_mut_lamports()? = 0;
        *escrow_account.try_borrow_mut_data()? = &mut [];

        Ok(())
    }
}
