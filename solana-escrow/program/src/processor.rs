use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
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
}
