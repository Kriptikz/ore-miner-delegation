use solana_program::{
    account_info::AccountInfo, program_error::ProgramError
};

use crate::error::OreDelegationError;

pub fn process_delegate_stake(
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    return Err(OreDelegationError::InstructionRemoved.into());
}
