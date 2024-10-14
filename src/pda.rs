use solana_program::pubkey::Pubkey;

pub fn managed_proof_pda(miner: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[crate::consts::MANAGED_PROOF, miner.as_ref()],
        &crate::id(),
    )
}

pub fn delegated_stake_pda(miner: Pubkey, staker: Pubkey) -> (Pubkey, u8) {
    let managed_proof_pda = managed_proof_pda(miner);

    Pubkey::find_program_address(
        &[
            crate::consts::DELEGATED_STAKE,
            staker.as_ref(),
            managed_proof_pda.0.as_ref(),
        ],
        &crate::id(),
    )
}
