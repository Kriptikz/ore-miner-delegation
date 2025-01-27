use solana_program::{
    account_info::AccountInfo, program_error::ProgramError
};

use crate::{
    loaders::{load_delegated_boost, load_delegated_boost_v2, load_managed_proof}, utils::AccountDeserializeV1
};

pub fn process_migrate_delegate_boost_v2(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [staker, miner, managed_proof_account_info, delegate_boost_account_info, delegate_boost_v2_account_info, token_mint_account_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_managed_proof(managed_proof_account_info, miner.key, false)?;
    load_delegated_boost(
        delegate_boost_account_info,
        staker.key,
        managed_proof_account_info.key,
        token_mint_account_info.key,
        true,
    )?;
    load_delegated_boost_v2(
        delegate_boost_v2_account_info,
        staker.key,
        managed_proof_account_info.key,
        token_mint_account_info.key,
        true,
    )?;

    // decrease from delegate boost v1
    let transfer_amount;
    if let Ok(mut data) = delegate_boost_account_info.data.try_borrow_mut() {
        let delegated_boost = crate::state::DelegatedBoost::try_from_bytes_mut(&mut data)?;

        if delegated_boost.amount == 0 {
            return Err(ProgramError::InsufficientFunds);
        }

        transfer_amount = delegated_boost.amount;

        if let Some(new_total) = delegated_boost.amount.checked_sub(transfer_amount) {
            delegated_boost.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    }

    // increase delegate v2 boost balance
    if let Ok(mut data) = delegate_boost_v2_account_info.data.try_borrow_mut() {
        let delegated_boost = crate::state::DelegatedBoostV2::try_from_bytes_mut(&mut data)?;

        if let Some(new_total) = delegated_boost.amount.checked_add(transfer_amount) {
            delegated_boost.amount = new_total;
        } else {
            return Err(ProgramError::ArithmeticOverflow);
        }
    } else {
        return Err(ProgramError::AccountBorrowFailed);
    }

    Ok(())
}
