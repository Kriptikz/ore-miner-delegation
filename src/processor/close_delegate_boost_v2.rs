use solana_program::{
    account_info::AccountInfo, program_error::ProgramError,
    system_program,
};

use crate::{
    error::OreDelegationError, loaders::{load_delegated_boost_v2, load_managed_proof}
};

pub fn process_close_delegate_boost_v2(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let [staker, miner, payer, managed_proof_account_info, delegate_boost_account_info, token_mint_account_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_managed_proof(managed_proof_account_info, miner.key, false)?;
    let delegate_boost_data = load_delegated_boost_v2(delegate_boost_account_info, staker.key, managed_proof_account_info.key, token_mint_account_info.key, true)?;

    if delegate_boost_data.amount != 0 {
        return Err(OreDelegationError::CannotCloseAccountWithBalance.into());
    }

    if delegate_boost_data.fee_payer != *payer.key {
        return Err(OreDelegationError::CloseAccountFeePayerMissmatch.into());
    }

    if *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    delegate_boost_account_info.realloc(0, true)?;

    **payer.lamports.borrow_mut() += delegate_boost_account_info.lamports();
    **delegate_boost_account_info.lamports.borrow_mut() = 0;

    Ok(())
}
