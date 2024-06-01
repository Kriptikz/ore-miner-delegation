use solana_program::{
    account_info::AccountInfo, program_error::ProgramError
};

pub fn process_register_proof(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        _fee_payer,
        managed_proof_account_info,
        _ore_proof_account_info,
        _ore_program
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

    // CPI to create the proof account


    // Set the ManangedProof account data

    Ok(())
}
