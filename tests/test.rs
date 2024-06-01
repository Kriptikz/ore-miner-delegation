use std::str::FromStr;

use solana_program::{
    pubkey::Pubkey,
    rent::Rent,
};
use solana_program_test::{processor, read_file, BanksClient, ProgramTest};
use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_register_proof() {
    let (mut banks_client, payer) = init_program().await;

    let ix = ore_miner_delegation::instruction::register_proof(payer.pubkey());

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    let blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&payer], blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");


    // Verify ore::Proof data

    // Verify ManagedProof data
}

pub async fn init_program() -> (BanksClient, Keypair) {
    let mut program_test = ProgramTest::new(
        "ore_miner_delegation",
        ore_miner_delegation::id(),
        processor!(ore_miner_delegation::process_instruction),
    );

    // Add Metadata Program account
    let data = read_file(&"tests/buffers/metadata_program.so");
    program_test.add_account(
        mpl_token_metadata::ID,
        Account {
            lamports: Rent::default().minimum_balance(data.len()).max(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    // Add Ore Program account
    let data = read_file(&"tests/buffers/ore.so");
    program_test.add_account(
        Pubkey::from_str("oreFHcE6FvJTrsfaYca4mVeZn7J7T6oZS9FAvW9eg4q").unwrap(),
        Account {
            lamports: Rent::default().minimum_balance(data.len()).max(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    let (mut banks_client, payer, blockhash) = program_test.start().await;

    // Initialize Ore Program
    let ix = ore::instruction::initialize(payer.pubkey());
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks_client.process_transaction(tx).await;
    assert!(res.is_ok());

    (banks_client, payer)
}
