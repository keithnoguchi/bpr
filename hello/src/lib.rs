//! [Rust Program Quickstart]
//!
//! [rust program quickstart]: https://docs.solana.com/getstarted/rust

#![forbid(missing_docs, missing_debug_implementations)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult as Result,
    msg,
    program_error::ProgramError as Error,
    pubkey::Pubkey,
};

/// On-chain counter account.
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Counter {
    /// Number of greetings it received.
    pub count: u8,
}

// Declares and export the program's entrypoint.
entrypoint!(process_instruction);

/// Implements the program's entrypoint.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result {
    // get the data account info, which is the first account.
    let iter = &mut accounts.iter();

    // Get the `AccountInfo` of the `Counter` data account.
    let counter_info = next_account_info(iter)?;
    if counter_info.owner != program_id {
        msg!("Counter account should be owned by the counter program account.");
        return Err(Error::IncorrectProgramId);
    }
    if counter_info.executable {
        msg!("Counter account should not be executable.");
        return Err(Error::InvalidAccountData);
    }
    if !counter_info.is_writable {
        msg!("Counter account should be writable.");
        return Err(Error::InvalidAccountData);
    }
    if counter_info.is_signer {
        msg!("Counter account should not be the signer");
        return Err(Error::InvalidAccountData);
    }

    // Increments and store the number of times the account has been greeted.
    let mut counter = Counter::try_from_slice(&counter_info.data.borrow())?;
    counter.count += 1;
    counter.serialize(&mut &mut counter_info.data.borrow_mut()[..])?;

    // I think this macro returns from the function, because
    // nothing happen, at least no counter update, if this
    // line is above the counter update code above.
    //
    // I'll come back later about this macro, though.
    //
    // And also, there is a buffer limit to dump all the `counter_info`.
    //msg!("counter.count={}: {counter_info:?}", counter.count);
    msg!("counter.count={}", counter.count);

    Ok(())
}
