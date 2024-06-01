use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};


// TODO: Update id with generated key
declare_id!("SWK6MtQGZ4NJaijbHw2UPgtuSAo3NgZoM1dGgQw2x7n");

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    if program_id.ne(&crate::id()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    msg!("Process Instruction for program Ore-Miner-Delegation");

    // let (_instruction, _data) = instruction_data
    //     .split_first()
    //     .ok_or(ProgramError::InvalidInstructionData)?;


    // let instruction =
    //     Instructions::try_from(*instruction).or(Err(ProgramError::InvalidInstructionData))?;

    // match instruction {
    //     Instructions::InitWorld => {
    //         msg!("Instruction Init World");
    //         process_init_world(accounts, data)?;
    //     }
    // }

    Ok(())
}
