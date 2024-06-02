use bytemuck::{Zeroable, Pod};
use drillx::Solution;
use num_enum::TryFromPrimitive;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey, system_program, sysvar,
};

use crate::{impl_to_bytes, impl_instruction_from_bytes};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Instructions {
    RegisterProof,
    InitDelegateStake,
    Mine,
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



pub fn register_proof(
    payer: Pubkey,
) -> Instruction {

    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", payer.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore::PROOF, managed_proof_authority.0.as_ref()], &ore::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", payer.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(ore::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::RegisterProof.into(),
    }
}

pub fn init_delegate_stake(
    payer: Pubkey,
    miner: Pubkey,
) -> Instruction {
    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", miner.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore::PROOF, managed_proof_authority.0.as_ref()], &ore::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.as_ref()], &crate::id());

    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", payer.as_ref(), managed_proof_account.0.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(miner, false),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new(delegated_stake_account.0, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(ore::id(), false),
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
    let ore_proof_account = Pubkey::find_program_address(&[ore::PROOF, managed_proof_authority.0.as_ref()], &ore::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", payer.as_ref()], &crate::id());

    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", payer.as_ref(), managed_proof_account.0.as_ref()], &crate::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(managed_proof_authority.0, false),
            AccountMeta::new(managed_proof_account.0, false),
            AccountMeta::new(bus, false),
            AccountMeta::new_readonly(ore::CONFIG_ADDRESS, false),
            AccountMeta::new(ore_proof_account.0, false),
            AccountMeta::new(delegated_stake_account.0, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(ore::id(), false),
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
