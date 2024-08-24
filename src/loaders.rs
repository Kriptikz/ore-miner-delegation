use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{state::ManagedProof, utils::AccountDeserialize};

pub fn load_managed_proof<'a, 'info> (
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
