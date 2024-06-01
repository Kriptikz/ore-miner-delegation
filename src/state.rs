use bytemuck::{Pod, Zeroable};
use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{
    impl_account_from_bytes, impl_to_bytes,
    utils::{AccountDiscriminator, Discriminator, AccountDeserialize}, impl_account_from_account_info,
};

// ManagedProof
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ManagedProof {
    pub bump: u8,
}

impl Discriminator for ManagedProof {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::ManagedProof
    }
}

impl_to_bytes!(ManagedProof);
impl_account_from_bytes!(ManagedProof);
impl_account_from_account_info!(ManagedProof);
