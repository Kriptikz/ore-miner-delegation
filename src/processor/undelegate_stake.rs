use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    instruction::DelegateStakeArgs,
    loaders::{load_delegated_stake, load_managed_proof},
    state::ManagedProof,
    utils::AccountDeserializeV1,
};

pub fn process_undelegate_stake(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [staker, miner, managed_proof_account_info, ore_proof_account_info, beneficiary_token_account_info, delegated_stake_account_info, treasury, treasury_tokens, ore_program, token_program] =
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
    load_delegated_stake(
        delegated_stake_account_info,
        staker.key,
        &managed_proof_account_info.key,
        true,
    )?;

    if *ore_program.key != ore_api::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let managed_proof = {
        let data = managed_proof_account_info.data.borrow();
        ManagedProof::try_from_bytes(&data)?.clone()
    };

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
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    }

    // stake to ore program
    solana_program::program::invoke_signed(
        &ore_api::prelude::claim(
            *managed_proof_account_info.key,
            *beneficiary_token_account_info.key,
            amount,
        ),
        &[
            managed_proof_account_info.clone(),
            ore_proof_account_info.clone(),
            beneficiary_token_account_info.clone(),
            treasury.clone(),
            treasury_tokens.clone(),
            ore_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof.bump],
        ]],
    )?;

    Ok(())
}
