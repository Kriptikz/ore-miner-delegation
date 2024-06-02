use instruction::Instructions;
use processor::{process_mine, process_init_delegate_stake};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::processor::process_register_proof;


pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

// TODO: Update id with generated key
declare_id!("SWK6MtQGZ4NJaijbHw2UPgtuSAo3NgZoM1dGgQw2x7n");

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id.ne(&crate::id()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (instruction, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;


    let instruction =
        Instructions::try_from(*instruction).or(Err(ProgramError::InvalidInstructionData))?;

    match instruction {
        Instructions::RegisterProof => {
            process_register_proof(accounts, data)?;
        },
        Instructions::Mine => {
            process_mine(accounts, data)?;
        },
        Instructions::InitDelegateStake => {
            process_init_delegate_stake(accounts, data)?;
        }
    }

    Ok(())
}
