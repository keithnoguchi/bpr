use solana_program::program_error::ProgramError;

/// An escrow errors.
pub(crate) enum EscrowError {
    InvalidInstruction,
    NotRentExempt,
    ExpectedAmountMismatch,
    AmountOverflow,
}

/// Converts the escrow errors into native `ProgramError`.
impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> ProgramError {
        ProgramError::Custom(e as u32)
    }
}
