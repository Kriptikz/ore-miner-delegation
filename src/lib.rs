use instruction::{init_delegate_boost, undelegate_boost, Instructions};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod consts;
pub mod delegate_stake;
pub mod error;
pub mod init_delegate_stake;
pub mod instruction;
pub mod loaders;
pub mod mine;
pub mod open_managed_proof;
pub mod pda;
pub mod state;
pub mod undelegate_stake;
pub mod utils;
pub mod open_managed_proof_boost;
pub mod delegate_boost;
pub mod undelegate_boost;
pub mod init_delegate_boost;
pub mod init_delegate_boost_v2;
pub mod delegate_boost_v2;
pub mod undelegate_boost_v2;
pub mod migrate_delegate_boost_to_v2;

declare_id!("J6XAzG8S5KmoBM8GcCFfF8NmtzD7U3QPnbhNiYwsu9we");

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id.ne(&crate::id()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (instruction, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let instruction =
        Instructions::try_from(*instruction).or(Err(ProgramError::InvalidInstructionData))?;

    match instruction {
        Instructions::OpenManagedProof => {
            open_managed_proof::process_open_managed_proof(accounts, data)?;
        }
        Instructions::Mine => {
            mine::process_mine(accounts, data)?;
        }
        Instructions::InitDelegateStake => {
            init_delegate_stake::process_init_delegate_stake(accounts, data)?;
        }
        Instructions::DelegateStake => {
            delegate_stake::process_delegate_stake(accounts, data)?;
        }
        Instructions::UndelegateStake => {
            undelegate_stake::process_undelegate_stake(accounts, data)?;
        }
        Instructions::OpenManagedProofBoost => {
            open_managed_proof_boost::process_open_managed_proof_boost(accounts, data)?;
        }
        Instructions::InitDelegateBoost => {
            init_delegate_boost::process_init_delegate_boost(accounts, data)?;
        }
        Instructions::DelegateBoost => {
            delegate_boost::process_delegate_boost(accounts, data)?;
        }
        Instructions::UndelegateBoost => {
            undelegate_boost::process_undelegate_boost(accounts, data)?;
        }
        Instructions::InitDelegateBoostV2 => {
            init_delegate_boost_v2::process_init_delegate_boost_v2(accounts, data)?;
        }
        Instructions::DelegateBoostV2 => {
            delegate_boost_v2::process_delegate_boost_v2(accounts, data)?;
        }
        Instructions::UndelegateBoostV2 => {
            undelegate_boost_v2::process_undelegate_boost_v2(accounts, data)?;
        }
        Instructions::MigrateDelegateBoostToV2 => {
            migrate_delegate_boost_to_v2::process_migrate_delegate_boost_v2(accounts, data)?;
        }
    }

    Ok(())
}
