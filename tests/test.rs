use drillx::equix;
use ore_api::consts::{BUS_ADDRESSES, NOOP_PROGRAM_ID};
use ore_boost_api::state::{boost_pda, stake_pda, BoostAccount};
use ore_miner_delegation::{
    pda::{delegated_boost_pda, delegated_boost_v2_pda, delegated_stake_pda, managed_proof_pda}, utils::AccountDeserializeV1
};
use solana_program::{clock::Clock, pubkey::Pubkey, rent::Rent, system_instruction};
use solana_program_test::{processor, read_file, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account, compute_budget::ComputeBudgetInstruction, program_pack::Pack,
    signature::Keypair, signer::Signer, transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use steel::AccountDeserialize as _;

#[tokio::test]
pub async fn test_init() {
    init_program().await;
}

#[tokio::test]
pub async fn test_mine_with_boost() {
    let context = init_program().await;

    let miner = Keypair::new();
    let staker = Keypair::new();

    // Send miner and staker sol
    let ix0 = system_instruction::transfer(&context.payer.pubkey(), &miner.pubkey(), 1000000000);
    let ix1 = system_instruction::transfer(&context.payer.pubkey(), &staker.pubkey(), 1000000000);

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    let mut tx = Transaction::new_with_payer(&[ix0, ix1], Some(&context.payer.pubkey()));
    tx.sign(&[&context.payer], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    let managed_proof_account = Pubkey::find_program_address(
        &[
            ore_miner_delegation::consts::MANAGED_PROOF,
            miner.pubkey().as_ref(),
        ],
        &ore_miner_delegation::id(),
    );
    let delegated_stake_account = Pubkey::find_program_address(
        &[
            ore_miner_delegation::consts::DELEGATED_STAKE,
            miner.pubkey().as_ref(),
            managed_proof_account.0.as_ref(),
        ],
        &ore_miner_delegation::id(),
    );
    let ore_proof_account = Pubkey::find_program_address(
        &[ore_api::consts::PROOF, managed_proof_account.0.as_ref()],
        &ore_api::id(),
    );

    let ix = ore_miner_delegation::instruction::open_managed_proof(miner.pubkey());

    let ix_delegate_stake = ore_miner_delegation::instruction::init_delegate_stake(
        miner.pubkey(),
        miner.pubkey(),
        miner.pubkey(),
    );
    let mut tx = Transaction::new_with_payer(&[ix, ix_delegate_stake], Some(&miner.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

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

    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(550000);
    let ix0 = ore_api::prelude::reset(miner.pubkey());

    // Set ix1 to be the proof declaration authentication
    let proof_declaration = ore_api::prelude::auth(ore_proof_account.0);

    let ix = ore_miner_delegation::instruction::mine_with_boost(miner.pubkey(), BUS_ADDRESSES[0], solution);

    let mut tx = Transaction::new_with_payer(
        &[cu_limit_ix, proof_declaration, ix0, ix],
        Some(&miner.pubkey()),
    );

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

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

    // Verify miner's delegate stake account amount
    let delegated_stake = context
        .banks_client
        .get_account(delegated_stake_account.0)
        .await
        .unwrap()
        .unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
    assert_eq!(ore_proof.balance, delegated_stake.amount);
}


#[tokio::test]
pub async fn test_open_managed_proof_boost_stake_and_unstake_v4() {
    let context = init_program().await;

    let miner = Keypair::new();
    let staker = Keypair::new();

    // Send miner and staker sol
    let ix0 = system_instruction::transfer(&context.payer.pubkey(), &miner.pubkey(), 1000000000);
    let ix1 = system_instruction::transfer(&context.payer.pubkey(), &staker.pubkey(), 1000000000);

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    let mut tx = Transaction::new_with_payer(&[ix0, ix1], Some(&context.payer.pubkey()));
    tx.sign(&[&context.payer], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    let managed_proof_account = Pubkey::find_program_address(
        &[
            ore_miner_delegation::consts::MANAGED_PROOF,
            miner.pubkey().as_ref(),
        ],
        &ore_miner_delegation::id(),
    );
    let delegated_stake_account = Pubkey::find_program_address(
        &[
            ore_miner_delegation::consts::DELEGATED_STAKE,
            miner.pubkey().as_ref(),
            managed_proof_account.0.as_ref(),
        ],
        &ore_miner_delegation::id(),
    );
    let ore_proof_account = Pubkey::find_program_address(
        &[ore_api::consts::PROOF, managed_proof_account.0.as_ref()],
        &ore_api::id(),
    );

    let ix = ore_miner_delegation::instruction::open_managed_proof(miner.pubkey());

    let ix_delegate_stake = ore_miner_delegation::instruction::init_delegate_stake(
        miner.pubkey(),
        miner.pubkey(),
        miner.pubkey(),
    );
    let mut tx = Transaction::new_with_payer(&[ix, ix_delegate_stake], Some(&miner.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

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

    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(550000);
    let ix0 = ore_api::prelude::reset(miner.pubkey());

    // Set ix1 to be the proof declaration authentication
    let proof_declaration = ore_api::prelude::auth(ore_proof_account.0);

    let ix = ore_miner_delegation::instruction::mine_with_boost(miner.pubkey(), BUS_ADDRESSES[0], solution);

    let mut tx = Transaction::new_with_payer(
        &[cu_limit_ix, proof_declaration, ix0, ix],
        Some(&miner.pubkey()),
    );

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

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

    // Verify miner's delegate stake account amount
    let delegated_stake = context
        .banks_client
        .get_account(delegated_stake_account.0)
        .await
        .unwrap()
        .unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
    assert_eq!(ore_proof.balance, delegated_stake.amount);

    let staker_token_account_addr = spl_associated_token_account::get_associated_token_address(
        &staker.pubkey(),
        &ore_api::consts::MINT_ADDRESS,
    );

    // create stakers ata
    let ix_2 = create_associated_token_account(
        &miner.pubkey(),
        &staker.pubkey(),
        &ore_api::consts::MINT_ADDRESS,
        &spl_token::id(),
    );

    // Claim from the delegated balance
    let ix = ore_miner_delegation::instruction::undelegate_stake(
        miner.pubkey(),
        miner.pubkey(),
        staker_token_account_addr,
        ore_proof.balance,
    );

    let mut tx = Transaction::new_with_payer(&[ix_2, ix], Some(&miner.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    let ore_proof = context
        .banks_client
        .get_account(ore_proof_account.0)
        .await
        .unwrap()
        .unwrap();
    let ore_proof = ore_api::state::Proof::try_from_bytes(&ore_proof.data).unwrap();
    assert_eq!(ore_proof.balance, 0);

    // Verify miner's delegate stake account amount
    let delegated_stake = context
        .banks_client
        .get_account(delegated_stake_account.0)
        .await
        .unwrap()
        .unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert_eq!(ore_proof.balance, delegated_stake.amount);

    let staker_token_account = context
        .banks_client
        .get_account(staker_token_account_addr)
        .await
        .unwrap()
        .unwrap();
    let staker_token_account =
        spl_token::state::Account::unpack(&staker_token_account.data).unwrap();
    let initial_claimed_amount = staker_token_account.amount;


    let ix = ore_miner_delegation::instruction::open_managed_proof_boost(miner.pubkey(), ore_api::consts::MINT_ADDRESS);
    let mut tx = Transaction::new_with_payer(&[ix], Some(&miner.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    // Init delegate boost account for staker
    let ix = ore_miner_delegation::instruction::init_delegate_boost_v2(
        staker.pubkey(),
        miner.pubkey(),
        miner.pubkey(),
        ore_api::consts::MINT_ADDRESS,
    );

    let ix1 = create_associated_token_account(
        &miner.pubkey(),
        &managed_proof_account.0,
        &ore_api::consts::MINT_ADDRESS,
        &spl_token::id(),
    );

    let mut tx = Transaction::new_with_payer(&[ix, ix1], Some(&miner.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    let new_clock = solana_program::clock::Clock {
        slot: 0,
        epoch_start_timestamp: proof.last_hash_at + 60,
        epoch: 140,
        leader_schedule_epoch: 141,
        unix_timestamp: 7201,
    };

    context.set_sysvar::<Clock>(&new_clock);

    // Delegate Boost
    let ix = ore_miner_delegation::instruction::delegate_boost_v2(staker.pubkey(), miner.pubkey(), ore_api::consts::MINT_ADDRESS, initial_claimed_amount);
    let mut tx = Transaction::new_with_payer(&[ix], Some(&miner.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&miner, &staker], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    let staker_token_account = context
        .banks_client
        .get_account(staker_token_account_addr)
        .await
        .unwrap()
        .unwrap();
    let staker_token_account =
        spl_token::state::Account::unpack(&staker_token_account.data).unwrap();
    assert_eq!(staker_token_account.amount, 0);

    let ix = ore_miner_delegation::instruction::undelegate_boost_v2(
        staker.pubkey(),
        miner.pubkey(),
        ore_api::consts::MINT_ADDRESS,
        initial_claimed_amount,
    );

    let mut tx = Transaction::new_with_payer(&[ix], Some(&staker.pubkey()));

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    tx.sign(&[&staker], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");

    let staker_token_account_data = context
        .banks_client
        .get_account(staker_token_account_addr)
        .await
        .unwrap()
        .unwrap();
    let staker_token_account =
        spl_token::state::Account::unpack(&staker_token_account_data.data).unwrap();
    let staker_token_balance = staker_token_account.amount;

    assert_eq!(staker_token_balance, initial_claimed_amount);
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
    let data = read_file(&"tests/buffers/ore-latest.so");
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

    let data = read_file(&"tests/buffers/boost_v4.so");
    program_test.add_account(
        ore_boost_api::id(),
        Account {
            lamports: Rent::default().minimum_balance(data.len()).max(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    let context = program_test.start_with_context().await;

    // Note: Programs are customized to remove auth signer for easy init
    let mut ixs = Vec::new();
    // IX: Initialize Ore Program
    let ix = ore_api::prelude::initialize(context.payer.pubkey());
    ixs.push(ix);
    // IX: Initialize ore-boost program
    let ix2 = ore_boost_api::prelude::initialize(context.payer.pubkey());
    ixs.push(ix2);
    // IX: Create the boost for ore tokens
    let ix3 = ore_boost_api::prelude::new(context.payer.pubkey(), ore_api::consts::MINT_ADDRESS, 999999999999999999, 4);
    ixs.push(ix3);
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let res = context.banks_client.process_transaction(tx).await;
    assert!(res.is_ok());

    context
}
