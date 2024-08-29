use ore_utils::AccountDeserialize as _;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, system_program};

use crate::{
    instruction::MineArgs, loaders::{load_delegated_stake, load_managed_proof}, state::ManagedProof,
    utils::AccountDeserialize,
};

pub fn process_mine(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), ProgramError> {
    let [miner, managed_proof_account_info, ore_bus_account_info, ore_config_account_info, ore_proof_account_info, delegated_stake_account_info, slothashes_sysvar, instructions_sysvar, ore_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = MineArgs::try_from_bytes(instruction_data)?;

    if !miner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    load_managed_proof(managed_proof_account_info, miner.key, true)?;
    load_delegated_stake(delegated_stake_account_info, miner.key, managed_proof_account_info.key, true)?;

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let balance_before = if let Ok(data) = ore_proof_account_info.data.try_borrow() {
        let ore_proof = ore_api::state::Proof::try_from_bytes(&data)?;
        ore_proof.balance
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    };

    let managed_proof = {
        let data = managed_proof_account_info.data.borrow();
        ManagedProof::try_from_bytes(&data)?.clone()
    };

    // CPI to submit the solution
    let solution = drillx::Solution::new(args.digest, args.nonce);
    solana_program::program::invoke_signed(
        &ore_api::instruction::mine(
            *managed_proof_account_info.key,
            *managed_proof_account_info.key,
            *ore_bus_account_info.key,
            solution,
        ),
        &[
            managed_proof_account_info.clone(),
            ore_proof_account_info.clone(),
            slothashes_sysvar.clone(),
            ore_bus_account_info.clone(),
            ore_config_account_info.clone(),
            instructions_sysvar.clone(),
            ore_program.clone(),
            system_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof.bump],
        ]],
    )?;

    let balance_after = if let Ok(data) = ore_proof_account_info.data.try_borrow() {
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
