use std::convert::TryInto;
use std::fmt::{self, Debug};

use solana_program::{msg, program_error::ProgramError};

use crate::error::EscrowError::InvalidInstruction;

/// Instructions of the escrow program.
pub(crate) enum Instruction {
    /// Start the trade by creating and populating an escrow account and
    /// transferring authority of the given temp token account to the PDA.
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]`   The account of the person initiating the escrow.
    /// 1. `[writable]` Temporary token account that should be created
    ///                 prior to this instruction and authorized by the
    ///                 initializer., e.g. Alice.
    /// 2. `[]`         The initializer's token account for the token
    ///                 they will receive should the trade go through.
    /// 3. `[writable]` The escrow account, it will hold all necesssary
    ///                 info about the trade.
    /// 4. `[]`         The rent sysvar.
    /// 5. `[]`         The token program.
    InitEscrow {
        /// The amount party A expects to receive of token *Y*.
        amount: u64,
    },
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InitEscrow { amount } => f
                .debug_struct("Instruction: InitEscrow")
                .field("amount", &amount)
                .finish(),
        }
    }
}

impl Instruction {
    pub(crate) fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        let ix = match tag {
            0 => Self::InitEscrow {
                amount: Self::unpack_amount(rest)?,
            },
            _ => return Err(InvalidInstruction.into()),
        };
        msg!("{:?}", ix);
        Ok(ix)
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        let amount = input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;

        Ok(amount)
    }
}
