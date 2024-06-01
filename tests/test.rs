use std::str::FromStr;

use ore::utils::AccountDeserialize as _;
use ore_miner_delegation::utils::AccountDeserialize as _;
use solana_program::{
    pubkey::Pubkey,
    rent::Rent, system_instruction,
};
use solana_program_test::{processor, read_file, BanksClient, ProgramTest};
use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_register_proof() {
    let (mut banks_client, payer) = init_program().await;

    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", payer.pubkey().as_ref()], &ore_miner_delegation::id());
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", payer.pubkey().as_ref()], &ore_miner_delegation::id());
    let ore_proof_account = Pubkey::find_program_address(&[ore::PROOF, managed_proof_authority.0.as_ref()], &ore::id());

    // TODO: move transfer into register_proof program ix
    let ix0 = system_instruction::transfer(&payer.pubkey(), &managed_proof_authority.0, 100000000);
    let ix = ore_miner_delegation::instruction::register_proof(payer.pubkey());

    let mut tx = Transaction::new_with_payer(&[ix0, ix], Some(&payer.pubkey()));

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
    let ore_proof = banks_client.get_account(ore_proof_account.0).await;

    assert!(ore_proof.is_ok(), "should get account info from banks_client");
    let ore_proof = ore_proof.unwrap();
    assert!(ore_proof.is_some(), "ore proof account should exist now");

    let ore_proof_account_info = ore_proof.unwrap();

    let ore_proof = ore::state::Proof::try_from_bytes(&ore_proof_account_info.data);
    assert!(ore_proof.is_ok());

    let ore_proof = ore_proof.unwrap();

    assert_eq!(0, ore_proof.balance, "Newly created proof account balance should be 0");

    // Verify ManagedProof data
    let managed_proof = banks_client.get_account(managed_proof_account.0).await;

    assert!(managed_proof.is_ok(), "should get account info from banks_client");
    let managed_proof = managed_proof.unwrap();
    assert!(managed_proof.is_some(), "ore proof account should exist now");

    let managed_proof_account_info = managed_proof.unwrap();

    let managed_proof = ore_miner_delegation::state::ManagedProof::try_from_bytes(&managed_proof_account_info.data);
    assert!(managed_proof.is_ok());

    let managed_proof = managed_proof.unwrap();
    assert_eq!(managed_proof_account.1, managed_proof.bump, "ManagedProof account created with invalid bump");
    assert_eq!(1, managed_proof.total_delegated, "ManagedProof account created with invalid total delegated amount");
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
        ore::id(),
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
