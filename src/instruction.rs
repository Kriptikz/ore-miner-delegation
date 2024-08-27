use bytemuck::{Zeroable, Pod};
use drillx::Solution;
use num_enum::TryFromPrimitive;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey, system_program, sysvar,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{impl_to_bytes, impl_instruction_from_bytes};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Instructions {
    OpenManagedProof,
    InitDelegateStake,
    Mine,
    DelegateStake,
    UndelegateStake,
}

impl Into<Vec<u8>> for Instructions {
    fn into(self) -> Vec<u8> {
        vec![self as u8]
    }
}

impl Instructions {
    pub fn to_vec(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

impl_to_bytes!(MineArgs);
impl_instruction_from_bytes!(MineArgs);


pub fn open_managed_proof(
    miner: Pubkey,
) -> Instruction {

    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", miner.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()], &ore_api::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(miner, true),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::OpenManagedProof.to_vec(),
    }
}

pub fn init_delegate_stake(
    payer: Pubkey,
    miner: Pubkey,
) -> Instruction {
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.as_ref()], &crate::id());

    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", payer.as_ref(), managed_proof_account.0.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(miner, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(delegated_stake_account.0, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::InitDelegateStake.into(),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MineArgs {
    pub digest: [u8; 16],
    pub nonce: [u8; 8],
}

pub fn mine(payer: Pubkey, bus: Pubkey, solution: Solution) -> Instruction {
    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", payer.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()], &ore_api::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", payer.as_ref()], &crate::id());

    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", payer.as_ref(), managed_proof_account.0.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(bus, false),
            AccountMeta::new_readonly(ore_api::consts::CONFIG_ADDRESS, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new(delegated_stake_account.0, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: [
            Instructions::Mine.to_vec(),
            MineArgs {
                digest: solution.d,
                nonce: solution.n,
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct DelegateStakeArgs {
    pub amount: [u8; 8],
}

impl_to_bytes!(DelegateStakeArgs);
impl_instruction_from_bytes!(DelegateStakeArgs);

pub fn delegate_stake(staker: Pubkey, miner: Pubkey, amount: u64) -> Instruction {
    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", miner.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()], &ore_api::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.as_ref()], &crate::id());

    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", staker.as_ref(), managed_proof_account.0.as_ref()], &crate::id());

    let staker_token_account = get_associated_token_address(&staker, &ore_api::consts::MINT_ADDRESS);
    let managed_proof_token_account = get_associated_token_address(&managed_proof_authority.0, &ore_api::consts::MINT_ADDRESS);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new(managed_proof_token_account, false),
            AccountMeta::new(staker_token_account, false),
            AccountMeta::new(delegated_stake_account.0, false),
            AccountMeta::new(ore_api::consts::TREASURY_ADDRESS, false),
            AccountMeta::new(ore_api::consts::TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: [
            Instructions::DelegateStake.to_vec(),
            DelegateStakeArgs {
                amount: amount.to_le_bytes(),
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct UndelegateStakeArgs {
    pub amount: [u8; 8],
}

impl_to_bytes!(UndelegateStakeArgs);
impl_instruction_from_bytes!(UndelegateStakeArgs);

pub fn undelegate_stake(staker: Pubkey, miner: Pubkey, beneficiary_token_account: Pubkey, amount: u64) -> Instruction {
    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", miner.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()], &ore_api::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.as_ref()], &crate::id());

    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", staker.as_ref(), managed_proof_account.0.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new(beneficiary_token_account, false),
            AccountMeta::new(delegated_stake_account.0, false),
            AccountMeta::new(ore_api::consts::TREASURY_ADDRESS, false),
            AccountMeta::new(ore_api::consts::TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: [
            Instructions::UndelegateStake.to_vec(),
            UndelegateStakeArgs {
                amount: amount.to_le_bytes(),
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}
