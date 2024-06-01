use std::str::FromStr;

use ore::utils::AccountDeserialize;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    rent::Rent,
};
use solana_program_test::{processor, read_file, BanksClient, ProgramTest};
use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction, hash::Hash};

#[tokio::test]
// Initializing the World account needs to occur only once.
// This should create the Land token mint, as well as store the associated
// token Mint required to mint the land tokens via the Create Land ix.
async fn test_init() {
    let (mut banks_client, payer, blockhash) = init_program().await;

    // Initialize Ore Program
    let ix = ore::instruction::initialize(payer.pubkey());
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks_client.process_transaction(tx).await;
    assert!(res.is_ok());

    // Test bus state
    for i in 0..ore::BUS_COUNT {
        let bus_account = banks_client
            .get_account(ore::BUS_ADDRESSES[i])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bus_account.owner, ore::id());
        let bus = ore::state::Bus::try_from_bytes(&bus_account.data).unwrap();
        assert_eq!(bus.id as u8, i as u8);
        assert_eq!(bus.rewards, 0);
    }

    let ix = Instruction {
        program_id: ore_miner_delegation::id(),
        accounts: vec![AccountMeta::new(payer.pubkey(), true)],
        data: [].into(),
    };

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
}

pub async fn init_program() -> (BanksClient, Keypair, Hash) {
    let mut program_test = ProgramTest::new(
        "ore_miner_delegation",
        ore_miner_delegation::id(),
        processor!(ore_miner_delegation::process_instruction),
    );

    // Setup metadata program
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

    // Setup ore program
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

    program_test.start().await
}
