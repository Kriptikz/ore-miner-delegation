use ore_api::{consts::TREASURY_TOKENS_ADDRESS, state::proof_pda};
use solana_program::pubkey;
use steel::*;

use crate::pda::managed_proof_pda;

pub static GLOBAL_BOOST_ID: Pubkey = pubkey!("BoostzzkNfCA9D1qNuN5xZxB5ErbK4zQuBeTHGDpXT1");

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
    Close = 1,
    Deposit = 2,
    Open = 3,
    Rotate = 4,
    Withdraw = 5,
    
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
pub struct Activate {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Claim {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Close {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Deactivate {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Deposit {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Initialize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct New {
    pub expires_at: [u8; 8],
    pub multiplier: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Open {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Rotate {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct UpdateAdmin {
    pub new_admin: Pubkey,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct UpdateBoost {
    pub expires_at: [u8; 8],
    pub multiplier: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Withdraw {
    pub amount: [u8; 8],
}

instruction!(BoostInstruction, Activate);
instruction!(BoostInstruction, Claim);
instruction!(BoostInstruction, Close);
instruction!(BoostInstruction, Deactivate);
instruction!(BoostInstruction, Deposit);
instruction!(BoostInstruction, Initialize);
instruction!(BoostInstruction, New);
instruction!(BoostInstruction, Open);
instruction!(BoostInstruction, Rotate);
instruction!(BoostInstruction, UpdateAdmin);
instruction!(BoostInstruction, UpdateBoost);
instruction!(BoostInstruction, Withdraw);




/// Fetch the PDA of the boost account.
pub fn boost_pda(mint: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[BOOST, mint.as_ref()], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the checkpoint account.
pub fn checkpoint_v3_pda(boost: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CHECKPOINT, boost.as_ref()], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the config account.
pub fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the directory account.
pub fn directory_v3_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DIRECTORY], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the reservation account.
pub fn reservation_v3_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RESERVATION, authority.as_ref()], &GLOBAL_BOOST_ID)
}

/// Fetch the PDA of the stake account.
pub fn stake_pda(authority: Pubkey, boost: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STAKE, authority.as_ref(), boost.as_ref()], &GLOBAL_BOOST_ID)
}

// Build initialize instruction.
pub fn initialize_v3(signer: Pubkey) -> Instruction {
    let config_pda = config_pda();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_pda.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Initialize {}.to_bytes(),
    }
}

pub fn withdraw(signer: Pubkey, mint: Pubkey, amount: u64) -> Instruction {
    let boost_address = boost_pda(mint).0;
    let boost_proof_address = proof_pda(boost_address).0;
    let boost_deposits_address =
        spl_associated_token_account::get_associated_token_address(&boost_address, &mint);
    let boost_rewards_address = spl_associated_token_account::get_associated_token_address(
        &boost_address,
        &ore_api::consts::MINT_ADDRESS,
    );
    let beneficiary_address =
        spl_associated_token_account::get_associated_token_address(&signer, &mint);
    let stake_address = stake_pda(signer, boost_address).0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(beneficiary_address, false),
            AccountMeta::new(boost_address, false),
            AccountMeta::new(boost_deposits_address, false),
            AccountMeta::new(boost_proof_address, false),
            AccountMeta::new(boost_rewards_address, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new_readonly(ore_api::consts::TREASURY_ADDRESS, false),
            AccountMeta::new(ore_api::consts::TREASURY_TOKENS_ADDRESS, false),
            AccountMeta::new_readonly(ore_api::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data: Withdraw {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}




