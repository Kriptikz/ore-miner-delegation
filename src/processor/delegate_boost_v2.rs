use steel::transfer;
use solana_program::{
    account_info::AccountInfo, clock::Clock, program_error::ProgramError, sysvar::Sysvar,
};

use crate::{
    error::OreDelegationError,
    instruction::DelegateBoostArgs,
    loaders::{load_delegated_boost_v2, load_managed_proof},
    state::ManagedProof,
    utils::AccountDeserializeV1,
};

pub fn process_delegate_boost_v2(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [
        staker,
        miner,
        managed_proof_account_info,
        managed_proof_account_token_account_info,
        delegate_boost_account_info,
        boost_account_info,
        token_mint_account_info,
        staker_token_account_info,
        boost_token_account_info,
        stake_account_info,
        ore_boost_program,
        token_program
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let clock = Clock::get()?;

    let current_timestamp = clock.unix_timestamp;

    if let Some(secs_passed_window) = current_timestamp.checked_rem(600) {
        // passed 5 mins
        if secs_passed_window > 300 {
            return Err(OreDelegationError::StakeWindowClosed.into());
        }
    } else {
        return Err(ProgramError::ArithmeticOverflow);
    }

    // Parse args
    let args = DelegateBoostArgs::try_from_bytes(instruction_data)?;
    let amount = u64::from_le_bytes(args.amount);

    if !staker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    load_managed_proof(managed_proof_account_info, miner.key, false)?;
    load_delegated_boost_v2(
        delegate_boost_account_info,
        staker.key,
        &managed_proof_account_info.key,
        token_mint_account_info.key,
        true,
    )?;

    if *ore_boost_program.key != ore_boost_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof = {
        let data = managed_proof_account_info.data.borrow();
        ManagedProof::try_from_bytes(&data)?.clone()
    };

    // transfer to miners token account
    transfer(
        staker,
        staker_token_account_info,
        managed_proof_account_token_account_info,
        token_program,
        amount,
    )?;

    // deposit into ore boost program
    solana_program::program::invoke_signed(
        &ore_boost_api::sdk::deposit(
            *managed_proof_account_info.key,
            *token_mint_account_info.key,
            amount,
        ),
        &[
            managed_proof_account_info.clone(),
            boost_account_info.clone(),
            boost_token_account_info.clone(),
            token_mint_account_info.clone(),
            managed_proof_account_token_account_info.clone(),
            stake_account_info.clone(),
            ore_boost_program.clone(),
            token_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof.bump],
        ]],
    )?;

    // increase delegate boost balance
    if let Ok(mut data) = delegate_boost_account_info.data.try_borrow_mut() {
        let delegated_boost = crate::state::DelegatedBoostV2::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_boost.amount.checked_add(amount) {
            delegated_boost.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    }

    Ok(())
}
