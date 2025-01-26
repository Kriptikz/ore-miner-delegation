use ore_api::{consts::TREASURY_TOKENS_ADDRESS, state::proof_pda};
use solana_program::pubkey;
use steel::*;

use crate::pda::managed_proof_pda;

pub static GLOBAL_BOOST_ID: Pubkey = pubkey!("BoosTyJFPPtrqJTdi49nnztoEWDJXfDRhyb2fha6PPy");

pub const BOOST: &[u8] = b"boost";

/// The seed of the config PDA.
pub const CONFIG: &[u8] = b"config";

/// The seed of the stake PDA.
pub const STAKE: &[u8] = b"stake";

/// The seed of the directory PDA.
pub const DIRECTORY: &[u8] = b"directory";

/// The seed of the checkpoint PDA.
pub const CHECKPOINT: &[u8] = b"checkpoint";

/// The seed of the reservation PDA.
pub const RESERVATION: &[u8] = b"reservation";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[rustfmt::skip]
pub enum BoostInstruction {
    // User
    Claim = 0,
    Deposit = 1,
    Open = 2,
    Rebase = 3,
    Register = 4,
    Rotate = 5,
    Withdraw = 6,
    
    // Admin
    Activate = 100,
    Deactivate = 101,
    Initialize = 102,
    New = 103,
    UpdateAdmin = 104,
    UpdateBoost = 105,
}

impl BoostInstruction {
    pub fn to_vec(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Initialize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Register {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Rotate {}

instruction!(BoostInstruction, Initialize);
instruction!(BoostInstruction, Register);
instruction!(BoostInstruction, Rotate);



/// Fetch the PDA of the boost account.
pub fn boost_pda(mint: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[BOOST, mint.as_ref()], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the checkpoint account.
pub fn checkpoint_pda(boost: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CHECKPOINT, boost.as_ref()], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the config account.
pub fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the directory account.
pub fn directory_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DIRECTORY], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the reservation account.
pub fn reservation_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RESERVATION, authority.as_ref()], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the stake account.
pub fn stake_pda(authority: Pubkey, boost: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STAKE, authority.as_ref(), boost.as_ref()], &GLOBAL_BOOST_ID)
}

// Build initialize instruction.
pub fn initialize(signer: Pubkey) -> Instruction {
    let config_pda = config_pda();
    let directory_pda = directory_pda();
    Instruction {
        program_id: GLOBAL_BOOST_ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_pda.0, false),
            AccountMeta::new(directory_pda.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Initialize {}
        .to_bytes(),
    }
}

// Build register instruction for CPI.
pub fn register(signer: Pubkey, payer: Pubkey, proof: Pubkey) -> Instruction {
    let reservation_pda = reservation_pda(proof);
    Instruction {
        program_id: GLOBAL_BOOST_ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(proof, false),
            AccountMeta::new(reservation_pda.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Register {}.to_bytes(),
    }
}

// Build rotate instruction for CPI .
pub fn rotate(signer: Pubkey, proof: Pubkey) -> Instruction {
    let directory_pda = directory_pda();
    let reservation_pda = reservation_pda(proof);
    Instruction {
        program_id: GLOBAL_BOOST_ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(directory_pda.0, false),
            AccountMeta::new_readonly(proof, false),
            AccountMeta::new(reservation_pda.0, false),
            AccountMeta::new_readonly(TREASURY_TOKENS_ADDRESS, false),
        ],
        data: Rotate {}.to_bytes(),
    }
}

