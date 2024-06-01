use num_enum::TryFromPrimitive;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey, system_program, sysvar,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Instructions {
    RegisterProof,
}

impl Into<Vec<u8>> for Instructions {
    fn into(self) -> Vec<u8> {
        vec![self as u8]
    }
}

pub fn register_proof(
    payer: Pubkey,
) -> Instruction {

    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof", payer.as_ref()], &crate::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore::PROOF, managed_proof_account.0.as_ref()], &ore::id());

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
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
