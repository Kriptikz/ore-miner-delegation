use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    state::{DelegatedBoost, DelegatedBoostV2, DelegatedStake, ManagedProof},
    utils::AccountDeserialize,
};

pub fn load_managed_proof<'a, 'info>(
    info: &'a AccountInfo<'info>,
    miner: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let managed_proof_data = info.data.borrow();
    let managed_proof = ManagedProof::try_from_bytes(&managed_proof_data)?;

    if managed_proof.miner_authority != *miner {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

pub fn load_delegated_stake<'a, 'info>(
    info: &'a AccountInfo<'info>,
    delegate_authority: &Pubkey,
    managed_proof: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let delegated_stake_data = info.data.borrow();
    let delegated_stake = DelegatedStake::try_from_bytes(&delegated_stake_data)?;

    let delegated_stake_pda = Pubkey::create_program_address(
        &[
            crate::consts::DELEGATED_STAKE,
            delegate_authority.as_ref(),
            managed_proof.as_ref(),
            &[delegated_stake.bump],
        ],
        &crate::id(),
    )?;

    if *info.key != delegated_stake_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

pub fn load_delegated_boost<'a, 'info>(
    info: &'a AccountInfo<'info>,
    delegate_authority: &Pubkey,
    managed_proof: &Pubkey,
    mint: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let delegated_boost_data = info.data.borrow();
    let delegated_boost = DelegatedBoost::try_from_bytes(&delegated_boost_data)?;

    let delegated_boost_pda = Pubkey::create_program_address(
        &[
            crate::consts::DELEGATED_BOOST,
            delegate_authority.as_ref(),
            mint.as_ref(),
            managed_proof.as_ref(),
            &[delegated_boost.bump],
        ],
        &crate::id(),
    )?;

    if *info.key != delegated_boost_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

pub fn load_delegated_boost_v2<'a, 'info>(
    info: &'a AccountInfo<'info>,
    delegate_authority: &Pubkey,
    managed_proof: &Pubkey,
    mint: &Pubkey,
    is_writable: bool,
) -> Result<(), ProgramError> {
    if info.owner.ne(&crate::id()) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let delegated_boost_data = info.data.borrow();
    let delegated_boost = DelegatedBoostV2::try_from_bytes(&delegated_boost_data)?;

    let delegated_boost_pda = Pubkey::create_program_address(
        &[
            crate::consts::DELEGATED_BOOST_V2,
            delegate_authority.as_ref(),
            mint.as_ref(),
            managed_proof.as_ref(),
            &[delegated_boost.bump],
        ],
        &crate::id(),
    )?;

    if *info.key != delegated_boost_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    if is_writable && !info.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

pub fn load_program<'a, 'info>(
    info: &'a AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if info.key.ne(&program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !info.executable {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}
