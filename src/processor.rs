use std::{mem::size_of, ops::{Div, Mul}};

use ore_utils::AccountDeserialize as _;
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, system_program, rent::Rent, sysvar::Sysvar, msg
};

use crate::{instruction::{DelegateStakeArgs, MineArgs, OpenManagedProofArgs}, state::{DelegatedStake, ManagedProof}, utils::{AccountDeserialize, Discriminator}};

pub fn process_open_managed_proof(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
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

    msg!("Open Proof Account");
    // Parse args
    let args = OpenManagedProofArgs::try_from_bytes(instruction_data)?;

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

    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", fee_payer.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", fee_payer.key.as_ref()], &crate::id());


    // CPI to create the proof account
    solana_program::program::invoke_signed(
        &ore_api::instruction::open(managed_proof_authority_pda.0, managed_proof_authority_pda.0, *fee_payer.key),
        &[
            managed_proof_authority_info.clone(),
            fee_payer.clone(),
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
    parsed_data.commission = args.commission;


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

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Need to use find_program_address here because I need the pda's bump.
    // Should store this in the managed_proof_account data.
    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", fee_payer.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", fee_payer.key.as_ref()], &crate::id());

    let balance_before = if let Ok(data)  = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore_api::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    // CPI to submit the solution
    //
    let solution = drillx::Solution::new(args.digest, args.nonce);
    solana_program::program::invoke_signed(
        &ore_api::instruction::mine(managed_proof_authority_pda.0, managed_proof_authority_pda.0, *ore_bus_account_info.key, solution),
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
        let ore_proof = ore_api::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };


    let miners_delegated_rewards = if let Ok(data) = managed_proof_account_info.data.try_borrow() {
        let managed_proof = crate::state::ManagedProof::try_from_bytes(&data)?;
        // Calculate the miners rewards subtracting the stakers commission
        let miner_rewards_earned = if let Some(difference) = balance_after.checked_sub(balance_before) {
            difference - ((difference as f64).mul(managed_proof.commission as f64 / 100.0)) as u64
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        };

        // Calculate the miners delegated_amount based on the delegated to actual balance ratio
        if managed_proof.total_delegated == balance_before {
            miner_rewards_earned
        } else {
            if let Some(amount) = (miner_rewards_earned as u128).checked_mul(managed_proof.total_delegated as u128) {
                if let Some(amount) = amount.checked_div(balance_before as u128) {
                    if let Ok(amount) = amount.try_into() {
                        amount
                    } else {
                        return Err(ProgramError::ArithmeticOverflow);
                    }
                } else {
                    return Err(ProgramError::ArithmeticOverflow);
                }
            } else {
                return Err(ProgramError::ArithmeticOverflow);
            }
        }
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };


    // Update the Miners DelegatedStake amount
    if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
        let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_stake.amount.checked_add(miners_delegated_rewards) {
            delegated_stake.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    }


    // Update the ManagedProof total delegated
    if let Ok(mut data) = managed_proof_account_info.data.try_borrow_mut() {
        let managed_proof = crate::state::ManagedProof::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = managed_proof.total_delegated.checked_add(miners_delegated_rewards) {
            managed_proof.total_delegated = new_total;
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
        fee_payer,
        miner_authority_info,
        managed_proof_authority_info,
        managed_proof_account_info,
        ore_config_account_info,
        ore_proof_account_info,
        delegated_stake_account_info,
        treasury,
        ore_program,
        token_program,
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = DelegateStakeArgs::try_from_bytes(instruction_data)?;

    if !managed_proof_account_info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    if managed_proof_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if ore_config_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if ore_proof_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if delegated_stake_account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    if treasury.data_is_empty() {
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
    let managed_proof_authority_pda = Pubkey::find_program_address(&[b"managed-proof-authority", miner_authority_info.key.as_ref()], &crate::id());
    let managed_proof_account_pda = Pubkey::find_program_address(&[b"managed-proof-account", miner_authority_info.key.as_ref()], &crate::id());
    if managed_proof_account_pda.0 != *managed_proof_account_info.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let proof_balance = if let Ok(data)  = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore_api::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    solana_program::program::invoke_signed(
        &ore_api::instruction::stake(managed_proof_authority_pda.0, *fee_payer.key, args.amount),
        &[
            managed_proof_authority_info.clone(),
            ore_proof_account_info.clone(),
            fee_payer.clone(),
            treasury.clone(),
            ore_program.clone(),
        ],
        &[&[b"managed-proof-authority", fee_payer.key.as_ref(), &[managed_proof_authority_pda.1]]],
    )?;

    if let Ok(mut data) = managed_proof_account_info.data.try_borrow_mut() {
        let managed_proof = crate::state::ManagedProof::try_from_bytes_mut(&mut data)?;

        let delegated_amount = if managed_proof.total_delegated == proof_balance {
            args.amount
        } else {
            if let Some(amount) = (args.amount as u128).checked_mul(managed_proof.total_delegated as u128) {
                if let Some(amount) = amount.checked_div(proof_balance as u128) {
                    if let Ok(amount) = amount.try_into() {
                        amount
                    } else {
                        return Err(ProgramError::ArithmeticOverflow);
                    }
                } else {
                    return Err(ProgramError::ArithmeticOverflow);
                }
            } else {
                return Err(ProgramError::ArithmeticOverflow);
            }
        };


        if let Some(new_total) = managed_proof.total_delegated.checked_add(delegated_amount) {
            managed_proof.total_delegated = new_total;
            if let Ok(mut data) = delegated_stake_account_info.data.try_borrow_mut() {
                let delegated_stake = crate::state::DelegatedStake::try_from_bytes_mut(&mut data)?;

                if let Some(new_total) = delegated_stake.amount.checked_add(delegated_amount) {
                    delegated_stake.amount = new_total;
                } else {
                    return Err(ProgramError::ArithmeticOverflow);
                }
            }
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    }
    Ok(())
}
