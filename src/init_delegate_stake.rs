use std::mem::size_of;

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
    system_program, sysvar::Sysvar,
};

use crate::{
    loaders::load_managed_proof,
    state::DelegatedStake,
    utils::{AccountDeserialize, Discriminator},
};

pub fn process_init_delegate_stake(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [signer, miner, managed_proof_account_info, delegate_stake_account_info, rent_sysvar, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_managed_proof(managed_proof_account_info, miner.key, false)?;

    if !delegate_stake_account_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if *rent_sysvar.key != solana_program::sysvar::rent::id() {
        return Err(ProgramError::UnsupportedSysvar);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let delegated_stake_pda = Pubkey::find_program_address(
        &[
            crate::consts::DELEGATED_STAKE,
            signer.key.as_ref(),
            managed_proof_account_info.key.as_ref(),
        ],
        &crate::id(),
    );

    let rent = Rent::get()?;

    let space = 8 + size_of::<DelegatedStake>();

    let cost = rent.minimum_balance(space);

    if delegate_stake_account_info.lamports() > 0 {
        // cleanup any lamports that may have been sent before our program
        // created the account
        solana_program::program::invoke_signed(
            &solana_program::system_instruction::transfer(
                delegate_stake_account_info.key,
                signer.key,
                delegate_stake_account_info.lamports(),
            ),
            &[
                signer.clone(),
                delegate_stake_account_info.clone(),
                system_program.clone(),
            ],
            &[&[
                crate::consts::DELEGATED_STAKE,
                signer.key.as_ref(),
                managed_proof_account_info.key.as_ref(),
                &[delegated_stake_pda.1],
            ]],
        )?;
    }

    solana_program::program::invoke_signed(
        &solana_program::system_instruction::create_account(
            signer.key,
            delegate_stake_account_info.key,
            cost,
            space
                .try_into()
                .expect("failed to convert space usize to u64"),
            &crate::id(),
        ),
        &[
            signer.clone(),
            delegate_stake_account_info.clone(),
            system_program.clone(),
        ],
        &[&[
            crate::consts::DELEGATED_STAKE,
            signer.key.as_ref(),
            managed_proof_account_info.key.as_ref(),
            &[delegated_stake_pda.1],
        ]],
    )?;

    // Set the DelegatedStake initial data
    if let Ok(mut data) = delegate_stake_account_info.data.try_borrow_mut() {
        data[0] = DelegatedStake::discriminator() as u8;

        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;
        delegated_stake.bump = delegated_stake_pda.1;
        delegated_stake.amount = 0;
    }

    Ok(())
}
