use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, system_program
};

use crate::global_boost::register;

pub fn process_register_global_boost(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [miner, managed_proof_account_info, managed_proof, reservation, system_program, ore_global_boost_program] =
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

    if managed_proof_account_pda.0 != *managed_proof.key {
        return Err(ProgramError::InvalidAccountData);
    }
     
    if *ore_global_boost_program.key != crate::global_boost::GLOBAL_BOOST_ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    // CPI to register the proof account
    solana_program::program::invoke_signed(
        &register(
            managed_proof_account_pda.0,
            *miner.key,
            *managed_proof_account_info.key,
        ),
        &[
            managed_proof.clone(),
            miner.clone(),
            managed_proof_account_info.clone(),
            reservation.clone(),
            system_program.clone(),
            ore_global_boost_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof_account_pda.1],
        ]],
    )?;

    Ok(())
}
