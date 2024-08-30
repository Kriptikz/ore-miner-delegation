use bytemuck::{Pod, Zeroable};
use drillx::Solution;
use num_enum::TryFromPrimitive;
use ore_api::state::proof_pda;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{impl_instruction_from_bytes, impl_to_bytes, pda::{delegated_stake_pda, managed_proof_pda}};

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

pub fn open_managed_proof(miner: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(miner, true),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(ore_proof_address.0, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(ore_api::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::OpenManagedProof.to_vec(),
    }
}

pub fn init_delegate_stake(staker: Pubkey, miner: Pubkey, payer: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_stake_address = delegated_stake_pda(miner, staker);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, false),
            AccountMeta::new(miner, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(delegated_stake_address.0, false),
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

pub fn mine(miner: Pubkey, bus: Pubkey, solution: Solution) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);
    let delegated_stake_address = delegated_stake_pda(miner, miner);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(miner, true),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(bus, false),
            AccountMeta::new_readonly(ore_api::consts::CONFIG_ADDRESS, false),
            AccountMeta::new(ore_proof_address.0, false),
            AccountMeta::new(delegated_stake_address.0, false),
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
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);
    let delegated_stake_address = delegated_stake_pda(miner, staker);

    let staker_token_account =
        get_associated_token_address(&staker, &ore_api::consts::MINT_ADDRESS);
    let managed_proof_token_account =
        get_associated_token_address(&managed_proof_address.0, &ore_api::consts::MINT_ADDRESS);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(ore_proof_address.0, false),
            AccountMeta::new(managed_proof_token_account, false),
            AccountMeta::new(staker_token_account, false),
            AccountMeta::new(delegated_stake_address.0, false),
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

pub fn undelegate_stake(
    staker: Pubkey,
    miner: Pubkey,
    beneficiary_token_account: Pubkey,
    amount: u64,
) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);
    let delegated_stake_address = delegated_stake_pda(miner, staker);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(ore_proof_address.0, false),
            AccountMeta::new(beneficiary_token_account, false),
            AccountMeta::new(delegated_stake_address.0, false),
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
