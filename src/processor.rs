use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, system_program
};

pub fn process_register_proof(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        fee_payer,
        managed_proof_account_info,
        ore_proof_account_info,
        slothashes_sysvar,
        rent_sysvar,
        ore_program,
        system_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !managed_proof_account_info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if !managed_proof_account_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if *ore_program.key != ore::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_pda = Pubkey::find_program_address(&[b"managed-proof", fee_payer.key.as_ref()], &crate::id());

    // CPI to create the proof account
    solana_program::program::invoke_signed(
        &ore::instruction::register(managed_proof_pda.0),
        &[
            fee_payer.clone(),
            managed_proof_account_info.clone(),
            ore_proof_account_info.clone(),
            slothashes_sysvar.clone(),
            rent_sysvar.clone(),
            ore_program.clone(),
            system_program.clone(),
        ],
        &[&[b"managed-proof", fee_payer.key.as_ref(), &[managed_proof_pda.1]]],
    )?;

    // Set the ManangedProof account data

    Ok(())
}
