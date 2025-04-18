use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use steel::{transfer_signed, transfer_signed_with_bump};

use crate::{
    instruction::UndelegateBoostArgs,
    loaders::{load_delegated_boost_v2, load_managed_proof},
    state::ManagedProof,
    utils::AccountDeserializeV1,
};

pub fn process_undelegate_boost_v2(
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
        boost_config_info,
        deposits_info,
        token_mint_account_info,
        config_proof_info,
        rewards_info,
        sender_info,
        staker_token_account_info,
        stake_account_info,
        ore_boost_program,
        ore_treasury_info,
        ore_treasury_tokens_info,
        ore_program,
        token_program
    ] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Parse args
    let args = UndelegateBoostArgs::try_from_bytes(instruction_data)?;
    let amount = u64::from_le_bytes(args.amount);

    if !staker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    load_managed_proof(managed_proof_account_info, miner.key, false)?;
    load_delegated_boost_v2(
        delegate_boost_account_info,
        staker.key,
        managed_proof_account_info.key,
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

    // decrease delegate boost balance
    if let Ok(mut data) = delegate_boost_account_info.data.try_borrow_mut() {
        let delegated_boost = crate::state::DelegatedBoostV2::try_from_bytes_mut(&mut data)?;

        if amount > delegated_boost.amount {
            return Err(ProgramError::InsufficientFunds);
        }

        if let Some(new_total) = delegated_boost.amount.checked_sub(amount) {
            delegated_boost.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    }

    // withdraw from boost program 
    solana_program::program::invoke_signed(
        &ore_boost_api::sdk::withdraw(
            *managed_proof_account_info.key,
            *token_mint_account_info.key,
            amount,
        ),
        &[
            managed_proof_account_info.clone(),
            boost_account_info.clone(),
            boost_config_info.clone(),
            deposits_info.clone(),
            token_mint_account_info.clone(),
            config_proof_info.clone(),
            rewards_info.clone(),
            sender_info.clone(),
            stake_account_info.clone(),
            ore_treasury_info.clone(),
            ore_treasury_tokens_info.clone(),
            ore_program.clone(),
            token_program.clone(),
        ],
        &[&[
            crate::consts::MANAGED_PROOF,
            miner.key.as_ref(),
            &[managed_proof.bump],
        ]],
    )?;

    let bump = managed_proof.bump;
    let seeds: Vec<&[u8]> = vec![
        crate::consts::MANAGED_PROOF,
        miner.key.as_ref(),
    ];

    let seeds: &[&[u8]] = &seeds;

    // transfer to stakers token account
    transfer_signed_with_bump(
        managed_proof_account_info,
        managed_proof_account_token_account_info,
        staker_token_account_info,
        token_program,
        amount,
        seeds,
        bump
    )?;

    Ok(())
}
