use ore_utils::spl::transfer;
use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    instruction::UndelegateStakeArgs,
    loaders::{load_delegated_stake, load_managed_proof},
    state::ManagedProof,
    utils::AccountDeserialize,
};

pub fn process_delegate_stake(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [staker, miner, managed_proof_account_info, ore_proof_account_info, managed_proof_account_token_account_info, staker_token_account_info, delegated_stake_account_info, treasury, treasury_tokens, ore_program, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = UndelegateStakeArgs::try_from_bytes(instruction_data)?;
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

    // transfer to miners token account
    transfer(
        staker,
        staker_token_account_info,
        managed_proof_account_token_account_info,
        token_program,
        amount,
    )?;

    // stake to ore program
    solana_program::program::invoke_signed(
        &ore_api::instruction::stake(
            *managed_proof_account_info.key,
            *managed_proof_account_token_account_info.key,
            amount,
        ),
        &[
            managed_proof_account_info.clone(),
            ore_proof_account_info.clone(),
            managed_proof_account_token_account_info.clone(),
            treasury.clone(),
            treasury_tokens.clone(),
            ore_program.clone(),
            token_program.clone(),
        ],
        &[&[
            b"managed-proof-account",
            miner.key.as_ref(),
            &[managed_proof.bump],
        ]],
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
