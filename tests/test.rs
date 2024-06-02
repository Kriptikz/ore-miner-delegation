use drillx::equix;
use ore::{utils::AccountDeserialize as _, BUS_ADDRESSES};
use ore_miner_delegation::utils::AccountDeserialize as _;
use solana_program::{clock::Clock, pubkey::Pubkey, rent::Rent, system_instruction};
use solana_program_test::{processor, read_file, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account, compute_budget::ComputeBudgetInstruction, signer::Signer,
    transaction::Transaction,
};

#[tokio::test]
async fn test_register_proof() {
    let context = init_program().await;

    let mut banks_client = context.banks_client;
    let payer = context.payer;

    let managed_proof_authority = Pubkey::find_program_address(
        &[b"managed-proof-authority", payer.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );
    let managed_proof_account = Pubkey::find_program_address(
        &[b"managed-proof-account", payer.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );
    let ore_proof_account = Pubkey::find_program_address(
        &[ore::PROOF, managed_proof_authority.0.as_ref()],
        &ore::id(),
    );

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

    assert!(
        ore_proof.is_ok(),
        "should get account info from banks_client"
    );
    let ore_proof = ore_proof.unwrap();
    assert!(ore_proof.is_some(), "ore proof account should exist now");

    let ore_proof_account_info = ore_proof.unwrap();

    let ore_proof = ore::state::Proof::try_from_bytes(&ore_proof_account_info.data);
    assert!(ore_proof.is_ok());

    let ore_proof = ore_proof.unwrap();

    assert_eq!(
        0, ore_proof.balance,
        "Newly created proof account balance should be 0"
    );

    // Verify ManagedProof data
    let managed_proof = banks_client.get_account(managed_proof_account.0).await;

    assert!(
        managed_proof.is_ok(),
        "should get account info from banks_client"
    );
    let managed_proof = managed_proof.unwrap();
    assert!(
        managed_proof.is_some(),
        "ore proof account should exist now"
    );

    let managed_proof_account_info = managed_proof.unwrap();

    let managed_proof =
        ore_miner_delegation::state::ManagedProof::try_from_bytes(&managed_proof_account_info.data);
    assert!(managed_proof.is_ok());

    let managed_proof = managed_proof.unwrap();
    assert_eq!(
        managed_proof_account.1, managed_proof.bump,
        "ManagedProof account created with invalid bump"
    );
    assert_eq!(
        1, managed_proof.total_delegated,
        "ManagedProof account created with invalid total delegated amount"
    );
    assert_eq!(
        payer.pubkey(),
        managed_proof.miner_authority,
        "ManagedProof account created with wrong miner authority"
    );
}

#[tokio::test]
pub async fn test_mine() {
    let mut context = init_program().await;

    let managed_proof_authority = Pubkey::find_program_address(
        &[b"managed-proof-authority", context.payer.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );
    // let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", payer.pubkey().as_ref()], &ore_miner_delegation::id());
    let ore_proof_account = Pubkey::find_program_address(
        &[ore::PROOF, managed_proof_authority.0.as_ref()],
        &ore::id(),
    );

    // TODO: move transfer into register_proof program ix
    let ix0 = system_instruction::transfer(
        &context.payer.pubkey(),
        &managed_proof_authority.0,
        100000000,
    );
    let ix = ore_miner_delegation::instruction::register_proof(context.payer.pubkey());

    let mut tx = Transaction::new_with_payer(&[ix0, ix], Some(&context.payer.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&context.payer], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    // Verify ore::Proof data
    let ore_proof = context
        .banks_client
        .get_account(ore_proof_account.0)
        .await
        .unwrap()
        .unwrap();
    let ore_proof = ore::state::Proof::try_from_bytes(&ore_proof.data).unwrap();

    let proof = ore_proof.clone();

    let mut memory = equix::SolverMemory::new();

    let mut nonce: u64 = 0;
    let hash;

    loop {
        // Create hash
        if let Ok(hx) =
            drillx::hash_with_memory(&mut memory, &proof.challenge, &nonce.to_le_bytes())
        {
            let new_difficulty = hx.difficulty();
            if new_difficulty.ge(&ore::MIN_DIFFICULTY) {
                hash = hx;
                nonce = nonce;

                break;
            }
        }

        // Increment nonce
        nonce += 1;
    }

    // Update clock to be 60 seconds after proof
    let new_clock = solana_program::clock::Clock {
        slot: 0,
        epoch_start_timestamp: proof.last_hash_at + 60,
        epoch: 140,
        leader_schedule_epoch: 141,
        unix_timestamp: proof.last_hash_at + 60,
    };

    context.set_sysvar::<Clock>(&new_clock);

    // Submit solution
    let solution = drillx::Solution::new(hash.d, nonce.to_le_bytes());

    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(500000);
    let ix0 = ore::instruction::reset(context.payer.pubkey());

    let ix =
        ore_miner_delegation::instruction::mine(context.payer.pubkey(), BUS_ADDRESSES[0], solution);

    let mut tx =
        Transaction::new_with_payer(&[cu_limit_ix, ix0, ix], Some(&context.payer.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&context.payer], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    // Verify proof account balance is updated

    // Verify managed proof account total_delegated

    // Verify miner's delegate stake account mount
}

pub async fn init_program() -> ProgramTestContext {
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

    let mut context = program_test.start_with_context().await;

    // Initialize Ore Program
    // TODO: initialize can only be called by the AUTHORIZED_INITIALIZER.
    // Will need to create the necessary accounts directly instead of using
    // the initialize instruction.
    let ix = ore::instruction::initialize(context.payer.pubkey());
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let res = context.banks_client.process_transaction(tx).await;
    assert!(res.is_ok());

    context
}
