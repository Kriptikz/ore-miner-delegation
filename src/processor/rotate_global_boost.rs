use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey
};

use crate::global_boost::rotate;

pub fn process_rotate_global_boost(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [miner, managed_proof_account_info, managed_proof, directory, reservation, treasury_tokens_address, ore_global_boost_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !miner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
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
        &rotate(
            managed_proof_account_pda.0,
            *managed_proof_account_info.key,
        ),
        &[
            managed_proof.clone(),
            directory.clone(),
            managed_proof_account_info.clone(),
            reservation.clone(),
            treasury_tokens_address.clone(),
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
