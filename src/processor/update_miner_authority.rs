use steel::{AccountDeserialize as _, Pubkey};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, system_program};

use crate::{
    loaders::{load_delegated_stake, load_managed_proof},
    state::ManagedProof,
    utils::AccountDeserialize,
};

pub fn process_update_miner_authority(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), ProgramError> {
    let [miner, managed_proof_account_info, new_miner_info, ore_proof_account_info, ore_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !miner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    load_managed_proof(managed_proof_account_info, miner.key, true)?;
    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_account_pda = Pubkey::find_program_address(
        &[crate::consts::MANAGED_PROOF, miner.key.as_ref()],
        &crate::id(),
    );

    // Update the Miners Authority
    solana_program::program::invoke_signed(
        &ore_api::sdk::update(
            *managed_proof_account_info.key,
            *new_miner_info.key,
        ),
        &[
            managed_proof_account_info.clone(),
            new_miner_info.clone(),
            ore_proof_account_info.clone()
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof_account_pda.1],
        ]],
    )?;

    Ok(())
}
