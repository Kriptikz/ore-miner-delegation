use std::mem::size_of;

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
    system_program, sysvar::Sysvar,
};

use crate::{
    state::ManagedProof,
    utils::{AccountDeserializeV1, Discriminator},
};

pub fn process_open_managed_proof(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [miner, managed_proof_account_info, ore_proof_account_info, slothashes_sysvar, rent_sysvar, ore_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !miner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !managed_proof_account_info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if !managed_proof_account_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
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
        &ore_api::prelude::open(
            managed_proof_account_pda.0,
            managed_proof_account_pda.0,
            *miner.key,
        ),
        &[
            miner.clone(),
            managed_proof_account_info.clone(),
            ore_proof_account_info.clone(),
            slothashes_sysvar.clone(),
            rent_sysvar.clone(),
            ore_program.clone(),
            system_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof_account_pda.1],
        ]],
    )?;

    // Set the ManangedProof account data
    let rent = Rent::get()?;

    let space = 8 + size_of::<ManagedProof>();

    let cost = rent.minimum_balance(space);

    if managed_proof_account_info.lamports() > 0 {
        // cleanup any lamports that may have been sent before our program
        // created the account
        solana_program::program::invoke_signed(
            &solana_program::system_instruction::transfer(
                managed_proof_account_info.key,
                miner.key,
                managed_proof_account_info.lamports(),
            ),
            &[
                miner.clone(),
                managed_proof_account_info.clone(),
                system_program.clone(),
            ],
            &[&[
                crate::consts::MANAGED_PROOF,
                miner.key.as_ref(),
                &[managed_proof_account_pda.1],
            ]],
        )?;
    }

    solana_program::program::invoke_signed(
        &solana_program::system_instruction::create_account(
            miner.key,
            managed_proof_account_info.key,
            cost,
            space
                .try_into()
                .expect("failed to convert space usize to u64"),
            &crate::id(),
        ),
        &[
            miner.clone(),
            managed_proof_account_info.clone(),
            system_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof_account_pda.1],
        ]],
    )?;

    let mut data = managed_proof_account_info.data.borrow_mut();

    data[0] = ManagedProof::discriminator() as u8;

    let parsed_data = ManagedProof::try_from_bytes_mut(&mut data)?;

    parsed_data.bump = managed_proof_account_pda.1;
    parsed_data.miner_authority = *miner.key;

    Ok(())
}
