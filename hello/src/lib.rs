//! [Rust Program Quickstart]
//!
//! [rust program quickstart]: https://docs.solana.com/getstarted/rust

#![forbid(missing_docs, missing_debug_implementations)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// On-chain greeting data account.
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Greeting {
    /// Number of greetings it received.
    pub counter: u8,
}

// Declares and export the program's entrypoint.
entrypoint!(process_instruction);

/// Implements the program's entrypoint.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // get the program account info, which is the first account.
    let iter = &mut accounts.iter();
    let program_account = next_account_info(iter)?;

    if program_account.owner != program_id {
        msg!("Greeting account does not have the correct program ID");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Increment and store the number of times the account has been greeted.
    let mut greeting_account = Greeting::try_from_slice(&program_account.data.borrow())?;
    greeting_account.counter += 1;
    greeting_account.serialize(&mut &mut program_account.data.borrow_mut()[..])?;

    // I think this macro returns from the function, because
    // nothing happen, at least no counter update, if this
    // line is above the counter update code above.
    //
    // I'll come back later about this macro, though.
    msg!("Greeted {} time(s)!", greeting_account.counter);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::clock::Epoch;
    use std::mem;

    #[test]
    fn test_process_instruction() {
        let program_id = Pubkey::default();
        let key = Pubkey::default();
        let mut lamports = 0;
        let mut data = vec![0; mem::size_of::<u8>()];
        let owner = Pubkey::default();
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            Epoch::default(),
        );
        let instruction_data = Vec::<u8>::new();
        let accounts = vec![account];

        // deserialization check.
        assert_eq!(
            Greeting::try_from_slice(&accounts[0].data.borrow())
                .unwrap()
                .counter,
            0,
        );

        // check if it increments the counter.
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(
            Greeting::try_from_slice(&accounts[0].data.borrow())
                .unwrap()
                .counter,
            1,
        );

        // once more.
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(
            Greeting::try_from_slice(&accounts[0].data.borrow())
                .unwrap()
                .counter,
            2,
        );
    }
}
