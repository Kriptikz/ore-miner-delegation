use bytemuck::{Pod, Zeroable};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    impl_account_from_account_info, impl_account_from_bytes, impl_to_bytes,
    utils::{AccountDeserialize, AccountDiscriminator, Discriminator},
};

// ManagedProof
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ManagedProof {
    pub bump: u8,
    _pad: [u8; 7],
    pub miner_authority: Pubkey,
}

impl Discriminator for ManagedProof {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::ManagedProof
    }
}

impl_to_bytes!(ManagedProof);
impl_account_from_bytes!(ManagedProof);
impl_account_from_account_info!(ManagedProof);

// DelegatedStake
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DelegatedStake {
    pub bump: u8,
    _pad: [u8; 7],
    pub amount: u64,
}

impl Discriminator for DelegatedStake {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::DelegatedStake
    }
}

impl_to_bytes!(DelegatedStake);
impl_account_from_bytes!(DelegatedStake);
impl_account_from_account_info!(DelegatedStake);

// DelegatedBoost
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DelegatedBoost {
    pub bump: u8,
    _pad: [u8; 7],
    pub managed_proof_pubkey: Pubkey,
    pub amount: u64,
}

impl Discriminator for DelegatedBoost {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::DelegatedBoost
    }
}

impl_to_bytes!(DelegatedBoost);
impl_account_from_bytes!(DelegatedBoost);
impl_account_from_account_info!(DelegatedBoost);

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DelegatedBoostV2 {
    pub bump: u8,
    _pad: [u8; 7],
    pub managed_proof_pubkey: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

impl Discriminator for DelegatedBoostV2 {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::DelegatedBoostV2
    }
}

impl_to_bytes!(DelegatedBoostV2);
impl_account_from_bytes!(DelegatedBoostV2);
impl_account_from_account_info!(DelegatedBoostV2);
