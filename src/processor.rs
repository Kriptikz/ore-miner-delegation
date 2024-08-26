use std::{mem::size_of, ops::{Div, Mul}};

use ore_api::loaders::{load_any_bus, load_config, load_proof_with_miner, load_treasury};
use ore_utils::{spl::transfer, AccountDeserialize as _};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, system_program, rent::Rent, sysvar::Sysvar, msg
};

use crate::{instruction::{DelegateStakeArgs, MineArgs}, loaders::{load_delegated_stake, load_managed_proof}, state::{DelegatedStake, ManagedProof}, utils::{AccountDeserialize, Discriminator}};

pub fn process_open_managed_proof(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        miner,
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

    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", miner.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", miner.key.as_ref()], &crate::id());


    // CPI to create the proof account
    solana_program::program::invoke_signed(
        &ore_api::instruction::open(managed_proof_authority_pda.0, managed_proof_authority_pda.0, *managed_proof_authority_info.key),
        &[
            managed_proof_authority_info.clone(),
            ore_proof_account_info.clone(),
            slothashes_sysvar.clone(),
            rent_sysvar.clone(),
            ore_program.clone(),
            system_program.clone(),
        ],
        &[&[b"managed-proof-authority", miner.key.as_ref(), &[managed_proof_authority_pda.1]]],
    )?;

    // Set the ManangedProof account data
    let rent = Rent::get()?;

    let space = 8 + size_of::<ManagedProof>();

    let cost = rent.minimum_balance(space);

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
        &[&[b"managed-proof-account", miner.key.as_ref(), &[managed_proof_account_pda.1]]],
    )?;

    let mut data = managed_proof_account_info.data.borrow_mut();
    
    data[0] = ManagedProof::discriminator() as u8;
    
    let parsed_data = ManagedProof::try_from_bytes_mut(&mut data)?;
    
    parsed_data.bump = managed_proof_account_pda.1;
    parsed_data.authority_bump = managed_proof_authority_pda.1;
    parsed_data.miner_authority = *miner.key;


    Ok(())
}

pub fn process_init_delegate_stake(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        fee_payer,
        miner,
        managed_proof_account_info,
        delegate_stake_account_info,
        rent_sysvar,
        system_program,
    ] =
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
        ore_program,
        system_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = MineArgs::try_from_bytes(instruction_data)?;

    load_managed_proof(managed_proof_account_info, fee_payer.key, true)?;
    load_any_bus(ore_bus_account_info, true)?;
    load_config(ore_config_account_info, false)?;
    load_proof_with_miner(ore_proof_account_info, managed_proof_authority_info.key, true)?;

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }


    let balance_before = if let Ok(data)  = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore_api::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    let managed_proof_data = managed_proof_account_info.data.borrow();
    let managed_proof = ManagedProof::try_from_bytes(&managed_proof_data)?;

    // CPI to submit the solution
    //
    let solution = drillx::Solution::new(args.digest, args.nonce);
    solana_program::program::invoke_signed(
        &ore_api::instruction::mine(*managed_proof_authority_info.key, *managed_proof_authority_info.key, *ore_bus_account_info.key, solution),
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
        &[&[b"managed-proof-authority", fee_payer.key.as_ref(), &[managed_proof.authority_bump]]],
    )?;

    let balance_after = if let Ok(data)  = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore_api::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    let miner_rewards_earned = if let Some(difference) = balance_after.checked_sub(balance_before) {
        difference
    } else {
        return Err(ProgramError::ArithmeticOverflow);
    };

    // Update the Miners DelegatedStake amount
    if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_stake.amount.checked_add(miner_rewards_earned) {
            delegated_stake.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }

    Ok(())
}

pub fn process_delegate_stake(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        staker,
        miner,
        managed_proof_authority_info,
        managed_proof_account_info,
        ore_config_account_info,
        ore_proof_account_info,
        managed_proof_authority_token_account_info,
        staker_token_account_info,
        delegated_stake_account_info,
        treasury,
        treasury_tokens,
        ore_program,
        token_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = DelegateStakeArgs::try_from_bytes(instruction_data)?;
    let amount = u64::from_le_bytes(args.amount);

    if !staker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    load_managed_proof(managed_proof_account_info, miner.key, false)?;
    load_config(ore_config_account_info, false)?;
    load_treasury(treasury, true)?;
    load_delegated_stake(delegated_stake_account_info, staker.key, &managed_proof_account_info.key, true)?;

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_data = managed_proof_account_info.data.borrow();
    let managed_proof = ManagedProof::try_from_bytes(&managed_proof_data)?;

    // transfer to miners token account
    transfer(
        staker,
        staker_token_account_info,
        managed_proof_authority_token_account_info,
        token_program,
        amount,
    )?;

    // stake to ore program
    solana_program::program::invoke_signed(
        &ore_api::instruction::stake(*managed_proof_authority_info.key, *managed_proof_authority_token_account_info.key, amount),
        &[
            managed_proof_authority_info.clone(),
            ore_proof_account_info.clone(),
            managed_proof_authority_token_account_info.clone(),
            treasury.clone(),
            treasury_tokens.clone(),
            ore_program.clone(),
            token_program.clone(),
        ],
        &[&[b"managed-proof-authority", miner.key.as_ref(), &[managed_proof.authority_bump]]],
    )?;

    // increase delegate stake balance
    if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_stake.amount.checked_add(amount) {
            delegated_stake.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }
    Ok(())
}

pub fn process_claim(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        fee_payer,
        managed_proof_authority_info,
        managed_proof_account_info,
        beneficiary_token_account,
        ore_proof_account_info,
        delegated_stake_account_info,
        treasury_address,
        treasury_tokens,
        ore_program,
        token_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // TODO: verify this is the miners delegated stake account

    // Parse args
    let args = crate::instruction::ClaimArgs::try_from_bytes(instruction_data)?;
    let amount = u64::from_le_bytes(args.amount);

    // if managed_proof_account_info.data_is_empty() {
    //     return Err(ProgramError::UninitializedAccount);
    // }

    if ore_proof_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if delegated_stake_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if treasury_tokens.data_is_empty() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Need to use find_program_address here because I need the pda's bump.
    // Should store this in the managed_proof_account data.
    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", fee_payer.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", fee_payer.key.as_ref()], &crate::id());
    if managed_proof_account_pda.0 != *managed_proof_account_info.key {
        return Err(ProgramError::InvalidAccountData);
    }

    solana_program::program::invoke_signed(
        &ore_api::instruction::claim(managed_proof_authority_pda.0, *beneficiary_token_account.key, amount),
        &[
            managed_proof_authority_info.clone(),
            beneficiary_token_account.clone(),
            ore_proof_account_info.clone(),
            treasury_address.clone(),
            treasury_tokens.clone(),
            ore_program.clone(),
        ],
        &[&[b"managed-proof-authority", fee_payer.key.as_ref(), &[managed_proof_authority_pda.1]]],
    )?;

    // decrease delegate stake balance
    if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_stake.amount.checked_sub(amount) {
            delegated_stake.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }
    Ok(())
}

pub fn process_undelegate_stake(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        staker,
        miner,
        managed_proof_authority_info,
        managed_proof_account_info,
        ore_config_account_info,
        ore_proof_account_info,
        managed_proof_authority_token_account_info,
        staker_token_account_info,
        delegated_stake_account_info,
        treasury,
        treasury_tokens,
        ore_program,
        token_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = DelegateStakeArgs::try_from_bytes(instruction_data)?;
    let amount = u64::from_le_bytes(args.amount);

    if !staker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    load_managed_proof(managed_proof_account_info, miner.key, false)?;
    load_config(ore_config_account_info, false)?;
    load_treasury(treasury, true)?;
    load_delegated_stake(delegated_stake_account_info, staker.key, &managed_proof_account_info.key, true)?;

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof_data = managed_proof_account_info.data.borrow();
    let managed_proof = ManagedProof::try_from_bytes(&managed_proof_data)?;

    // stake to ore program
    solana_program::program::invoke_signed(
        &ore_api::instruction::claim(*managed_proof_authority_info.key, *managed_proof_authority_token_account_info.key, amount),
        &[
            managed_proof_authority_info.clone(),
            ore_proof_account_info.clone(),
            managed_proof_authority_token_account_info.clone(),
            treasury.clone(),
            treasury_tokens.clone(),
            ore_program.clone(),
        ],
        &[&[b"managed-proof-authority", miner.key.as_ref(), &[managed_proof.authority_bump]]],
    )?;

    // decrease delegate stake balance
    if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

        if amount > delegated_stake.amount {
            return Err(ProgramError::InsufficientFunds);
        }

        if let Some(new_total) = delegated_stake.amount.checked_sub(amount) {
            delegated_stake.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }

    // transfer from miner token account
    transfer(
        staker,
        staker_token_account_info,
        managed_proof_authority_token_account_info,
        token_program,
        amount,
    )?;

    Ok(())
}
