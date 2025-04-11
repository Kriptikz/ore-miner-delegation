use bytemuck::{Pod, Zeroable};
use drillx::Solution;
use num_enum::TryFromPrimitive;
use ore_api::{consts::TREASURY_TOKENS_ADDRESS, state::proof_pda};
use ore_boost_api::state::{boost_pda, stake_pda};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{
    global_boost::{directory_pda, reservation_pda, GLOBAL_BOOST_ID}, impl_instruction_from_bytes, impl_to_bytes, pda::{delegated_boost_pda, delegated_boost_v2_pda, delegated_stake_pda, managed_proof_pda}
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Instructions {
    OpenManagedProof,
    InitDelegateStake,
    Mine,
    DelegateStake,
    UndelegateStake,
    OpenManagedProofBoost,
    DelegateBoost,
    UndelegateBoost,
    InitDelegateBoost,
    DelegateBoostV2,
    UndelegateBoostV2,
    InitDelegateBoostV2,
    MigrateDelegateBoostToV2,
    CloseDelegateBoostV2,
    RegisterGlobalBoost,
    RotateGlobalBoost,
    UpdateMiningAuthority,
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

pub fn mine_with_boost(miner: Pubkey, bus: Pubkey, solution: Solution) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);
    let delegated_stake_address = delegated_stake_pda(miner, miner);
    let boost_config = ore_boost_api::state::config_pda();
    let boost_proof = ore_api::state::proof_pda(boost_config.0);

    let accounts = vec![
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
        AccountMeta::new_readonly(boost_config.0, false),
        AccountMeta::new(boost_proof.0, false)
    ];

    Instruction {
        program_id: crate::id(),
        accounts,
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

pub fn open_managed_proof_boost(miner: Pubkey, mint: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let (boost_pda, _) = ore_boost_api::state::boost_pda(mint);
    let (stake_pda, _) = ore_boost_api::state::stake_pda(managed_proof_address.0, boost_pda);


    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(miner, true),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new_readonly(boost_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(stake_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(ore_boost_api::id(), false),

        ],
        data: Instructions::OpenManagedProofBoost.into(),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct DelegateBoostArgs {
    pub amount: [u8; 8],
}

impl_to_bytes!(DelegateBoostArgs);
impl_instruction_from_bytes!(DelegateBoostArgs);

pub fn delegate_boost(staker: Pubkey, miner: Pubkey, mint: Pubkey, amount: u64) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_pda(miner, staker, mint);

    let staker_token_account =
        get_associated_token_address(&staker, &mint);
    let managed_proof_token_account =
        get_associated_token_address(&managed_proof_address.0, &mint);

    let boost_pda = boost_pda(mint);
    let boost_tokens_address =
        spl_associated_token_account::get_associated_token_address(&boost_pda.0, &mint);
    let stake_pda = stake_pda(managed_proof_address.0, boost_pda.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(managed_proof_token_account, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new(boost_pda.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(staker_token_account, false),
            AccountMeta::new(boost_tokens_address, false),
            AccountMeta::new(stake_pda.0, false),
            AccountMeta::new_readonly(ore_boost_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: [
            Instructions::DelegateBoost.to_vec(),
            DelegateBoostArgs {
                amount: amount.to_le_bytes(),
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}

pub fn init_delegate_boost(staker: Pubkey, miner: Pubkey, payer: Pubkey, mint: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_pda(miner, staker, mint);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, false),
            AccountMeta::new(miner, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::InitDelegateBoost.into(),
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct UndelegateBoostArgs {
    pub amount: [u8; 8],
}

impl_to_bytes!(UndelegateBoostArgs);
impl_instruction_from_bytes!(UndelegateBoostArgs);

pub fn undelegate_boost(staker: Pubkey, miner: Pubkey, mint: Pubkey, amount: u64) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_pda(miner, staker, mint);

    let staker_token_account =
        get_associated_token_address(&staker, &mint);
    let managed_proof_token_account =
        get_associated_token_address(&managed_proof_address.0, &mint);

    let boost_pda = boost_pda(mint);
    let boost_tokens_address =
        spl_associated_token_account::get_associated_token_address(&boost_pda.0, &mint);
    let stake_pda = stake_pda(managed_proof_address.0, boost_pda.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(managed_proof_token_account, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new(boost_pda.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(staker_token_account, false),
            AccountMeta::new(boost_tokens_address, false),
            AccountMeta::new(stake_pda.0, false),
            AccountMeta::new_readonly(ore_boost_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: [
            Instructions::UndelegateBoost.to_vec(),
            UndelegateBoostArgs {
                amount: amount.to_le_bytes(),
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}

pub fn init_delegate_boost_v2(staker: Pubkey, miner: Pubkey, payer: Pubkey, mint: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_v2_pda(miner, staker, mint);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, false),
            AccountMeta::new(miner, false),
            AccountMeta::new(payer, true),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::InitDelegateBoostV2.into(),
    }
}

pub fn delegate_boost_v2(staker: Pubkey, miner: Pubkey, mint: Pubkey, amount: u64) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_v2_pda(miner, staker, mint);

    let staker_token_account =
        get_associated_token_address(&staker, &mint);
    let managed_proof_token_account =
        get_associated_token_address(&managed_proof_address.0, &mint);

    let boost_pda = boost_pda(mint);
    let boost_tokens_address =
        spl_associated_token_account::get_associated_token_address(&boost_pda.0, &mint);
    let stake_pda = stake_pda(managed_proof_address.0, boost_pda.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(managed_proof_token_account, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new(boost_pda.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(staker_token_account, false),
            AccountMeta::new(boost_tokens_address, false),
            AccountMeta::new(stake_pda.0, false),
            AccountMeta::new_readonly(ore_boost_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: [
            Instructions::DelegateBoostV2.to_vec(),
            DelegateBoostArgs {
                amount: amount.to_le_bytes(),
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}

pub fn undelegate_boost_v2(staker: Pubkey, miner: Pubkey, mint: Pubkey, amount: u64) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_v2_pda(miner, staker, mint);

    let staker_token_account =
        get_associated_token_address(&staker, &mint);
    let managed_proof_token_account =
        get_associated_token_address(&managed_proof_address.0, &mint);

    let boost_pda = boost_pda(mint);
    let boost_tokens_address =
        spl_associated_token_account::get_associated_token_address(&boost_pda.0, &mint);
    let stake_pda = stake_pda(managed_proof_address.0, boost_pda.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(managed_proof_token_account, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new(boost_pda.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(staker_token_account, false),
            AccountMeta::new(boost_tokens_address, false),
            AccountMeta::new(stake_pda.0, false),
            AccountMeta::new_readonly(ore_boost_api::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: [
            Instructions::UndelegateBoostV2.to_vec(),
            UndelegateBoostArgs {
                amount: amount.to_le_bytes(),
            }
            .to_bytes()
            .to_vec(),
        ]
        .concat(),
    }
}

pub fn migrate_boost_to_v2(staker: Pubkey, miner: Pubkey, mint: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_pda(miner, staker, mint);
    let delegated_boost_address_v2 = delegated_boost_v2_pda(miner, staker, mint);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, true),
            AccountMeta::new_readonly(miner, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new(delegated_boost_address_v2.0, false),
            AccountMeta::new_readonly(mint, false),
        ],
        data: Instructions::MigrateDelegateBoostToV2.to_vec(),
    }
}

pub fn close_delegate_boost_v2(staker: Pubkey, miner: Pubkey, payer: Pubkey, mint: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let delegated_boost_address = delegated_boost_v2_pda(miner, staker, mint);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(staker, false),
            AccountMeta::new(miner, false),
            AccountMeta::new(payer, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(delegated_boost_address.0, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Instructions::CloseDelegateBoostV2.into(),
    }
}

pub fn register_global_boost(miner: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);
    let reservation = reservation_pda(ore_proof_address.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(miner, true),
            AccountMeta::new_readonly(ore_proof_address.0, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new(reservation.0, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(GLOBAL_BOOST_ID, false),
        ],
        data: Instructions::RegisterGlobalBoost.into(),
    }
}

pub fn rotate_global_boost(miner: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);
    let directory = directory_pda();
    let reservation = reservation_pda(ore_proof_address.0);

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(miner, true),
            AccountMeta::new_readonly(ore_proof_address.0, false),
            AccountMeta::new(managed_proof_address.0, false),
            AccountMeta::new_readonly(directory.0, false),
            AccountMeta::new(reservation.0, false),
            AccountMeta::new_readonly(TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(GLOBAL_BOOST_ID, false),
        ],
        data: Instructions::RotateGlobalBoost.into(),
    }
}

pub fn update_miner_authority(miner: Pubkey, new_miner_auth: Pubkey) -> Instruction {
    let managed_proof_address = managed_proof_pda(miner);
    let ore_proof_address = proof_pda(managed_proof_address.0);

    let accounts = vec![
        AccountMeta::new(miner, true),
        AccountMeta::new(managed_proof_address.0, false),
        AccountMeta::new(new_miner_auth, false),
        AccountMeta::new(ore_proof_address.0, false),
        AccountMeta::new_readonly(ore_api::id(), false),
    ];

    Instruction {
        program_id: crate::id(),
        accounts,
        data: Instructions::UpdateMiningAuthority.into(),
    }
}

