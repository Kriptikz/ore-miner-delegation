use drillx::equix;
use ore_api::consts::{BUS_ADDRESSES, NOOP_PROGRAM_ID};
use ore_utils::AccountDeserialize as _;
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
        &[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()],
        &ore_api::id(),
    );

    // TODO: move transfer into register_proof program ix
    let ix0 = system_instruction::transfer(&payer.pubkey(), &managed_proof_authority.0, 100000000);
    let commission: u8 = 10;
    let ix = ore_miner_delegation::instruction::open_managed_proof(payer.pubkey(), commission);

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

    let ore_proof = ore_api::state::Proof::try_from_bytes(&ore_proof_account_info.data);
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
        managed_proof.commission, 10,
        "Managed proof should have set 10 commision rate"
    );
    assert_eq!(
        0, managed_proof.total_delegated,
        "ManagedProof account created with invalid total delegated amount"
    );
    assert_eq!(
        payer.pubkey(),
        managed_proof.miner_authority,
        "ManagedProof account created with wrong miner authority"
    );
}

#[tokio::test]
pub async fn test_init_delegate_stake_account() {
    let mut context = init_program().await;

    let managed_proof_authority = Pubkey::find_program_address(
        &[b"managed-proof-authority", context.payer.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", context.payer.pubkey().as_ref()], &ore_miner_delegation::id());
    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", context.payer.pubkey().as_ref(), managed_proof_account.0.as_ref()], &ore_miner_delegation::id());


    // TODO: move transfer into register_proof program ix
    let ix0 = system_instruction::transfer(
        &context.payer.pubkey(),
        &managed_proof_authority.0,
        100000000,
    );

    let ix = ore_miner_delegation::instruction::open_managed_proof(context.payer.pubkey(), 10);

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


    // Create the DelegatedStake Accont
    let ix =
        ore_miner_delegation::instruction::init_delegate_stake(context.payer.pubkey(), context.payer.pubkey());

    let mut tx =
        Transaction::new_with_payer(&[ix], Some(&context.payer.pubkey()));

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


    // Verify miner's delegate stake account amount
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await;

    assert!(
        delegated_stake.is_ok(),
        "should get account info from banks_client"
    );
    let delegated_stake = delegated_stake.unwrap();
    assert!(
        delegated_stake.is_some(),
        "delegated_stake account should exist now"
    );

    let delegated_stake_account_info = delegated_stake.unwrap();

    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake_account_info.data);
    assert!(delegated_stake.is_ok());

    let delegated_stake = delegated_stake.unwrap();
    assert_eq!(
        delegated_stake_account.1, delegated_stake.bump,
        "DelegatedStake account created with invalid bump"
    );

    assert_eq!(0, delegated_stake.amount);
}


#[tokio::test]
pub async fn test_mine() {
    let mut context = init_program().await;

    let managed_proof_authority = Pubkey::find_program_address(
        &[b"managed-proof-authority", context.payer.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );
    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", context.payer.pubkey().as_ref()], &ore_miner_delegation::id());
    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", context.payer.pubkey().as_ref(), managed_proof_account.0.as_ref()], &ore_miner_delegation::id());
    let ore_proof_account = Pubkey::find_program_address(
        &[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()],
        &ore_api::id(),
    );

    // TODO: move transfer into register_proof program ix
    let ix0 = system_instruction::transfer(
        &context.payer.pubkey(),
        &managed_proof_authority.0,
        100000000,
    );
    let ix = ore_miner_delegation::instruction::open_managed_proof(context.payer.pubkey(), 10);

    let ix_delegate_stake =
        ore_miner_delegation::instruction::init_delegate_stake(context.payer.pubkey(), context.payer.pubkey());
    let mut tx = Transaction::new_with_payer(&[ix0, ix, ix_delegate_stake], Some(&context.payer.pubkey()));

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
    let ore_proof = ore_api::state::Proof::try_from_bytes(&ore_proof.data).unwrap();

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
            if new_difficulty.gt(&ore_api::consts::INITIAL_MIN_DIFFICULTY) {
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


    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(700000);
    let ix0 = ore_api::instruction::reset(context.payer.pubkey());
    
    // Set ix1 to be the proof declaration authentication
    let proof_declaration = ore_api::instruction::auth(ore_proof_account.0);

    let ix =
        ore_miner_delegation::instruction::mine(context.payer.pubkey(), BUS_ADDRESSES[0], solution);

    let mut tx =
        Transaction::new_with_payer(&[cu_limit_ix, proof_declaration, ix0, ix], Some(&context.payer.pubkey()));

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
    let ore_proof = context
        .banks_client
        .get_account(ore_proof_account.0)
        .await
        .unwrap()
        .unwrap();
    let ore_proof = ore_api::state::Proof::try_from_bytes(&ore_proof.data).unwrap();
    assert!(ore_proof.balance > 0);

    // Verify managed proof account total_delegated
    let managed_proof = context
        .banks_client
        .get_account(managed_proof_account.0)
        .await
        .unwrap()
        .unwrap();
    let managed_proof = ore_miner_delegation::state::ManagedProof::try_from_bytes(&managed_proof.data).unwrap();
    assert!(managed_proof.total_delegated > 0);

    // Verify miner's delegate stake account mount
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
}

pub async fn init_program() -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "ore_miner_delegation",
        ore_miner_delegation::id(),
        processor!(ore_miner_delegation::process_instruction),
    );

    // Add Noop Program
    let data = read_file(&"tests/buffers/noop.so");
    program_test.add_account(
        NOOP_PROGRAM_ID,
        Account {
            lamports: Rent::default().minimum_balance(data.len()).max(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
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
        ore_api::id(),
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
    let ix = ore_api::instruction::initialize(context.payer.pubkey());
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
