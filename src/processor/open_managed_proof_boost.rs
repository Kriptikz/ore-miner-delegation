use solana_program::{
    account_info::AccountInfo, log::sol_log, program_error::ProgramError, pubkey::Pubkey, system_program
};

pub fn process_open_managed_proof_boost(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [miner, managed_proof_account_info, boost_account_info, token_mint_account_info, stake_boost_account_info, system_program, ore_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !miner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_account_pda = Pubkey::find_program_address(
        &[crate::consts::MANAGED_PROOF, miner.key.as_ref()],
        &crate::id(),
    );

    // CPI to create the proof account
    solana_program::program::invoke_signed(
        &ore_boost_api::sdk::open(
            managed_proof_account_pda.0,
            *miner.key,
            *token_mint_account_info.key,
        ),
        &[
            managed_proof_account_info.clone(),
            miner.clone(),
            boost_account_info.clone(),
            token_mint_account_info.clone(),
            stake_boost_account_info.clone(),
            system_program.clone(),
            ore_boost_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof_account_pda.1],
        ]],
    )?;

    Ok(())
}
