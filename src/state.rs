use bytemuck::{Pod, Zeroable};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    impl_account_from_bytes, impl_to_bytes,
    utils::{AccountDiscriminator, Discriminator, AccountDeserialize}, impl_account_from_account_info,
};

// ManagedProof
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ManagedProof {
    pub bump: u8,
    pub authority_bump: u8,
    pub commission: u8,
    _pad: [u8; 5],
    pub miner_authority: Pubkey,
    pub total_delegated: u64,
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
