use num_enum::IntoPrimitive;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum OreDelegationError {
    #[error("Stake delegation window is currently closed")]
    StakeWindowClosed,
    #[error("Cannot close account with balance")]
    CannotCloseAccountWithBalance,
    #[error("Init account fee payer must match provided payer")]
    CloseAccountFeePayerMissmatch,
    #[error("Instruction has been removed")]
    InstructionRemoved,
}

impl From<OreDelegationError> for solana_program::program_error::ProgramError {
    fn from(e: OreDelegationError) -> Self {
        solana_program::program_error::ProgramError::Custom(e as u32)
    }
}
