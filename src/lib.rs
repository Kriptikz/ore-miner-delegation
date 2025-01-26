use instruction::Instructions;
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

mod processor;

pub mod consts;
pub mod error;
pub mod instruction;
pub mod loaders;
pub mod pda;
pub mod state;
pub mod utils;
pub mod global_boost;

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
            processor::open_managed_proof::process_open_managed_proof(accounts, data)?;
        }
        Instructions::Mine => {
            processor::mine::process_mine(accounts, data)?;
        }
        Instructions::InitDelegateStake => {
            processor::init_delegate_stake::process_init_delegate_stake(accounts, data)?;
        }
        Instructions::DelegateStake => {
            processor::delegate_stake::process_delegate_stake(accounts, data)?;
        }
        Instructions::UndelegateStake => {
            processor::undelegate_stake::process_undelegate_stake(accounts, data)?;
        }
        Instructions::OpenManagedProofBoost => {
            processor::open_managed_proof_boost::process_open_managed_proof_boost(accounts, data)?;
        }
        Instructions::InitDelegateBoost => {
            processor::init_delegate_boost::process_init_delegate_boost(accounts, data)?;
        }
        Instructions::DelegateBoost => {
            processor::delegate_boost::process_delegate_boost(accounts, data)?;
        }
        Instructions::UndelegateBoost => {
            processor::undelegate_boost::process_undelegate_boost(accounts, data)?;
        }
        Instructions::InitDelegateBoostV2 => {
            processor::init_delegate_boost_v2::process_init_delegate_boost_v2(accounts, data)?;
        }
        Instructions::DelegateBoostV2 => {
            processor::delegate_boost_v2::process_delegate_boost_v2(accounts, data)?;
        }
        Instructions::UndelegateBoostV2 => {
            processor::undelegate_boost_v2::process_undelegate_boost_v2(accounts, data)?;
        }
        Instructions::MigrateDelegateBoostToV2 => {
            processor::migrate_delegate_boost_to_v2::process_migrate_delegate_boost_v2(accounts, data)?;
        }
        Instructions::CloseDelegateBoostV2 => {
            processor::close_delegate_boost_v2::process_close_delegate_boost_v2(accounts, data)?;
        }
        Instructions::RegisterGlobalBoost => {
            processor::register_global_boost::process_register_global_boost(accounts, data)?;
        }
        Instructions::RotateGlobalBoost => {
            processor::rotate_global_boost::process_rotate_global_boost(accounts, data)?;
        }
        Instructions::UpdateMiningAuthority => {
            processor::update_miner_authority::process_update_miner_authority(accounts, data)?;
        }
    }

    Ok(())
}
