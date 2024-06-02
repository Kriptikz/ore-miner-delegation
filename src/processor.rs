use std::mem::size_of;

use ore::utils::AccountDeserialize as _;
// use crate::utils::AccountDeserialize as _;
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, system_program, rent::Rent, sysvar::Sysvar, msg
};

use crate::{state::{ManagedProof, DelegatedStake}, utils::{AccountDeserialize, Discriminator}, instruction::MineArgs};

pub fn process_register_proof(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        fee_payer,
        managed_proof_authority_info,
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

    msg!("Register Proof");

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

    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", fee_payer.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", fee_payer.key.as_ref()], &crate::id());


    // CPI to create the proof account
    solana_program::program::invoke_signed(
        &ore::instruction::register(managed_proof_authority_pda.0),
        &[
            managed_proof_authority_info.clone(),
            ore_proof_account_info.clone(),
            slothashes_sysvar.clone(),
            rent_sysvar.clone(),
            ore_program.clone(),
            system_program.clone(),
        ],
        &[&[b"managed-proof-authority", fee_payer.key.as_ref(), &[managed_proof_authority_pda.1]]],
    )?;

    // Set the ManangedProof account data
    let rent = Rent::get()?;

    let space = 8 + size_of::<ManagedProof>();

    let cost = rent.minimum_balance(space);

    solana_program::program::invoke_signed(
        &solana_program::system_instruction::create_account(
            fee_payer.key,
            managed_proof_account_info.key,
            cost,
            space
                .try_into()
                .expect("failed to convert space usize to u64"),
            &crate::id(),
        ),
        &[
            fee_payer.clone(),
            managed_proof_account_info.clone(),
            system_program.clone(),
        ],
        &[&[b"managed-proof-account", fee_payer.key.as_ref(), &[managed_proof_account_pda.1]]],
    )?;

    let mut data = managed_proof_account_info.data.borrow_mut();
    
    data[0] = ManagedProof::discriminator() as u8;
    
    let parsed_data = ManagedProof::try_from_bytes_mut(&mut data)?;
    
    parsed_data.bump = managed_proof_account_pda.1;
    parsed_data.authority_bump = managed_proof_authority_pda.1;
    parsed_data.total_delegated = 0;
    parsed_data.miner_authority = *fee_payer.key;


    Ok(())
}

pub fn process_init_delegate_stake(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        fee_payer,
        miner,
        managed_proof_authority_info,
        managed_proof_account_info,
        ore_proof_account_info,
        delegate_stake_account_info,
        slothashes_sysvar,
        instructions_sysvar,
        rent_sysvar,
        ore_program,
        system_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    msg!("Init Delegate Stake");

    if !managed_proof_account_info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if managed_proof_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if !delegate_stake_account_info.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if *rent_sysvar.key != solana_program::sysvar::rent::id() {
        return Err(ProgramError::UnsupportedSysvar);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", miner.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", miner.key.as_ref()], &crate::id());

    let delegated_stake_pda = Pubkey::find_program_address(&[b"delegated-stake", fee_payer.key.as_ref(), managed_proof_account_info.key.as_ref()], &crate::id());

    let rent = Rent::get()?;

    let space = 8 + size_of::<DelegatedStake>();

    let cost = rent.minimum_balance(space);

    solana_program::program::invoke_signed(
        &solana_program::system_instruction::create_account(
            fee_payer.key,
            delegate_stake_account_info.key,
            cost,
            space
                .try_into()
                .expect("failed to convert space usize to u64"),
            &crate::id(),
        ),
        &[
            fee_payer.clone(),
            delegate_stake_account_info.clone(),
            system_program.clone(),
        ],
        &[&[b"delegated-stake", fee_payer.key.as_ref(), managed_proof_account_info.key.as_ref(), &[delegated_stake_pda.1]]],
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

pub fn process_mine(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        fee_payer,
        managed_proof_authority_info,
        managed_proof_account_info,
        ore_bus_account_info,
        ore_config_account_info,
        ore_proof_account_info,
        delegated_stake_account_info,
        slothashes_sysvar,
        instructions_sysvar,
        rent_sysvar,
        ore_program,
        system_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = MineArgs::try_from_bytes(instruction_data)?;

    if !managed_proof_account_info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if managed_proof_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if ore_bus_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if ore_config_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if ore_proof_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if *ore_program.key != ore::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", fee_payer.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", fee_payer.key.as_ref()], &crate::id());

    let balance_before = if let Ok(data)  = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    // CPI to submit the solution
    //
    let solution = drillx::Solution::new(args.digest, args.nonce);
    solana_program::program::invoke_signed(
        &ore::instruction::mine(managed_proof_authority_pda.0, *ore_bus_account_info.key, solution),
        &[
            managed_proof_authority_info.clone(),
            ore_proof_account_info.clone(),
            slothashes_sysvar.clone(),
            ore_bus_account_info.clone(),
            ore_config_account_info.clone(),
            instructions_sysvar.clone(),
            ore_program.clone(),
            system_program.clone(),
        ],
        &[&[b"managed-proof-authority", fee_payer.key.as_ref(), &[managed_proof_authority_pda.1]]],
    )?;

    let balance_after = if let Ok(data)  = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    let balance_diff = if let Some(difference) = balance_after.checked_sub(balance_before) {
        difference
    } else {
        return Err(ProgramError::ArithmeticOverflow);
    };

    // Update the ManagedProof total delegated
    if let Ok(mut data) = managed_proof_account_info.data.try_borrow_mut() {
        let managed_proof = crate::state::ManagedProof::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = managed_proof.total_delegated.checked_add(balance_diff) {
            managed_proof.total_delegated = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }

    // Update the Miners DelegatedStake amount
    if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_stake.amount.checked_add(balance_diff) {
            delegated_stake.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }

    Ok(())
}
