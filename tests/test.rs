use drillx::equix;
use ore_api::consts::{BUS_ADDRESSES, NOOP_PROGRAM_ID};
use ore_utils::AccountDeserialize as _;
use ore_miner_delegation::utils::AccountDeserialize as _;
use solana_program::{clock::Clock, pubkey::Pubkey, rent::Rent, system_instruction};
use solana_program_test::{processor, read_file, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account, compute_budget::ComputeBudgetInstruction, program_pack::Pack, signature::Keypair, signer::Signer, transaction::Transaction
};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};

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
    let ix = ore_miner_delegation::instruction::open_managed_proof(payer.pubkey());

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

    let ix = ore_miner_delegation::instruction::open_managed_proof(context.payer.pubkey());

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
    let ix = ore_miner_delegation::instruction::open_managed_proof(context.payer.pubkey());

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


    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(550000);
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

    // Verify miner's delegate stake account mount
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
}

#[tokio::test]
pub async fn test_claim() {
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

    let ix = ore_miner_delegation::instruction::open_managed_proof(context.payer.pubkey());

    let ix_delegate_stake =
        ore_miner_delegation::instruction::init_delegate_stake(context.payer.pubkey(), context.payer.pubkey());
    let mut tx = Transaction::new_with_payer(&[ix, ix_delegate_stake], Some(&context.payer.pubkey()));

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


    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(550000);
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

    // Verify miner's delegate stake account amount
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
    assert_eq!(ore_proof.balance, delegated_stake.amount);

    // create miners ata
    let ix_0 = create_associated_token_account(&context.payer.pubkey(), &context.payer.pubkey(), &ore_api::consts::MINT_ADDRESS, &spl_token::id());

    // Claim from the delegated balance
    let ix =
        ore_miner_delegation::instruction::claim(context.payer.pubkey(), ore_proof.balance);

    let mut tx =
        Transaction::new_with_payer(&[ix_0, ix], Some(&context.payer.pubkey()));

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

    let ore_proof = context
        .banks_client
        .get_account(ore_proof_account.0)
        .await
        .unwrap()
        .unwrap();
    let ore_proof = ore_api::state::Proof::try_from_bytes(&ore_proof.data).unwrap();
    assert_eq!(ore_proof.balance, 0);

    // Verify miner's delegate stake account amount
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert_eq!(ore_proof.balance, delegated_stake.amount);
}

#[tokio::test]
pub async fn test_stake() {
    let mut context = init_program().await;

    let miner = Keypair::new();
    let staker = Keypair::new();
    
    let managed_proof_authority = Pubkey::find_program_address(
        &[b"managed-proof-authority", miner.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );

    // Send miner and staker sol 
    let ix0 = system_instruction::transfer(&context.payer.pubkey(), &miner.pubkey(), 1000000000);
    let ix1 = system_instruction::transfer(&context.payer.pubkey(), &staker.pubkey(), 1000000000);
    let ix2 = system_instruction::transfer(&context.payer.pubkey(), &managed_proof_authority.0, 1000000000);

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    let mut tx = Transaction::new_with_payer(&[ix0, ix1, ix2], Some(&context.payer.pubkey()));
    tx.sign(&[&context.payer], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");


    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.pubkey().as_ref()], &ore_miner_delegation::id());
    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", miner.pubkey().as_ref(), managed_proof_account.0.as_ref()], &ore_miner_delegation::id());
    let ore_proof_account = Pubkey::find_program_address(
        &[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()],
        &ore_api::id(),
    );

    let ix = ore_miner_delegation::instruction::open_managed_proof(miner.pubkey());

    let ix_delegate_stake =
        ore_miner_delegation::instruction::init_delegate_stake(miner.pubkey(), miner.pubkey());
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
    let ix0 = ore_api::instruction::reset(miner.pubkey());
    
    // Set ix1 to be the proof declaration authentication
    let proof_declaration = ore_api::instruction::auth(ore_proof_account.0);

    let ix =
        ore_miner_delegation::instruction::mine(miner.pubkey(), BUS_ADDRESSES[0], solution);

    let mut tx =
        Transaction::new_with_payer(&[cu_limit_ix, proof_declaration, ix0, ix], Some(&miner.pubkey()));

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
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
    assert_eq!(ore_proof.balance, delegated_stake.amount);

    // create miners ata
    let ix_0 = create_associated_token_account(&miner.pubkey(), &miner.pubkey(), &ore_api::consts::MINT_ADDRESS, &spl_token::id());

    // create managed_proof_authority ata
    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", miner.pubkey().as_ref()], &ore_miner_delegation::id());
    let ix_1 = create_associated_token_account(&miner.pubkey(), &managed_proof_authority.0, &ore_api::consts::MINT_ADDRESS, &spl_token::id());

    // Claim from the delegated balance
    let ix =
        ore_miner_delegation::instruction::claim(miner.pubkey(), ore_proof.balance);

    let mut tx =
        Transaction::new_with_payer(&[ix_0, ix_1, ix], Some(&miner.pubkey()));

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
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert_eq!(ore_proof.balance, delegated_stake.amount);

    let miner_token_account_addr = spl_associated_token_account::get_associated_token_address(
        &miner.pubkey(),
        &ore_api::consts::MINT_ADDRESS,
    );
    let staker_token_account_addr = spl_associated_token_account::get_associated_token_address(
        &staker.pubkey(),
        &ore_api::consts::MINT_ADDRESS,
    );

    let managed_proof_authority_token_account = get_associated_token_address(&managed_proof_authority.0, &ore_api::consts::MINT_ADDRESS);
    let managed_proof_authority_token_account = context.banks_client.get_account(managed_proof_authority_token_account).await.unwrap().unwrap();
    let miner_token_account = spl_token::state::Account::unpack(&managed_proof_authority_token_account.data).unwrap();
    let miner_token_balance = miner_token_account.amount;

    // transfer to staker from miner
    let ix0 = create_associated_token_account(&staker.pubkey(), &staker.pubkey(), &ore_api::consts::MINT_ADDRESS, &spl_token::id());
    let ix1 = spl_token::instruction::transfer_checked(&spl_token::id(), &miner_token_account_addr, &ore_api::consts::MINT_ADDRESS, &staker_token_account_addr, &miner.pubkey(), &[&miner.pubkey()], miner_token_balance, ore_api::consts::TOKEN_DECIMALS).unwrap();
    let mut tx =
        Transaction::new_with_payer(&[ix0, ix1], Some(&miner.pubkey()));

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

    // Delegate stake from staker to miner pool
    let ix0 =
        ore_miner_delegation::instruction::init_delegate_stake(staker.pubkey(), miner.pubkey());
    let ix =
        ore_miner_delegation::instruction::delegate_stake(staker.pubkey(), miner.pubkey(), miner_token_balance);

    let mut tx =
        Transaction::new_with_payer(&[ix0, ix], Some(&staker.pubkey()));

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

    let staker_delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", staker.pubkey().as_ref(), managed_proof_account.0.as_ref()], &ore_miner_delegation::id());
    let delegated_stake = context.banks_client.get_account(staker_delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert_eq!(miner_token_balance, delegated_stake.amount);
}

#[tokio::test]
pub async fn test_unstake() {
    let mut context = init_program().await;

    let miner = Keypair::new();
    let staker = Keypair::new();
    
    let managed_proof_authority = Pubkey::find_program_address(
        &[b"managed-proof-authority", miner.pubkey().as_ref()],
        &ore_miner_delegation::id(),
    );

    // Send miner and staker sol 
    let ix0 = system_instruction::transfer(&context.payer.pubkey(), &miner.pubkey(), 1000000000);
    let ix1 = system_instruction::transfer(&context.payer.pubkey(), &staker.pubkey(), 1000000000);
    let ix2 = system_instruction::transfer(&context.payer.pubkey(), &managed_proof_authority.0, 1000000000);

    let blockhash = context
        .banks_client
        .get_latest_blockhash()
        .await
        .expect("should get latest blockhash");

    let mut tx = Transaction::new_with_payer(&[ix0, ix1, ix2], Some(&context.payer.pubkey()));
    tx.sign(&[&context.payer], blockhash);

    context
        .banks_client
        .process_transaction(tx)
        .await
        .expect("process_transaction should be ok");


    let managed_proof_account = Pubkey::find_program_address(&[b"managed-proof-account", miner.pubkey().as_ref()], &ore_miner_delegation::id());
    let delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", miner.pubkey().as_ref(), managed_proof_account.0.as_ref()], &ore_miner_delegation::id());
    let ore_proof_account = Pubkey::find_program_address(
        &[ore_api::consts::PROOF, managed_proof_authority.0.as_ref()],
        &ore_api::id(),
    );

    let ix = ore_miner_delegation::instruction::open_managed_proof(miner.pubkey());

    let ix_delegate_stake =
        ore_miner_delegation::instruction::init_delegate_stake(miner.pubkey(), miner.pubkey());
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
    let ix0 = ore_api::instruction::reset(miner.pubkey());
    
    // Set ix1 to be the proof declaration authentication
    let proof_declaration = ore_api::instruction::auth(ore_proof_account.0);

    let ix =
        ore_miner_delegation::instruction::mine(miner.pubkey(), BUS_ADDRESSES[0], solution);

    let mut tx =
        Transaction::new_with_payer(&[cu_limit_ix, proof_declaration, ix0, ix], Some(&miner.pubkey()));

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
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert!(delegated_stake.amount > 0);
    assert_eq!(ore_proof.balance, delegated_stake.amount);

    // create miners ata
    let ix_0 = create_associated_token_account(&miner.pubkey(), &miner.pubkey(), &ore_api::consts::MINT_ADDRESS, &spl_token::id());

    // create managed_proof_authority ata
    let managed_proof_authority = Pubkey::find_program_address(&[b"managed-proof-authority", miner.pubkey().as_ref()], &ore_miner_delegation::id());
    let ix_1 = create_associated_token_account(&miner.pubkey(), &managed_proof_authority.0, &ore_api::consts::MINT_ADDRESS, &spl_token::id());

    // Claim from the delegated balance
    let ix =
        ore_miner_delegation::instruction::claim(miner.pubkey(), ore_proof.balance);

    let mut tx =
        Transaction::new_with_payer(&[ix_0, ix_1, ix], Some(&miner.pubkey()));

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
    let delegated_stake = context.banks_client.get_account(delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert_eq!(ore_proof.balance, delegated_stake.amount);

    let miner_token_account_addr = spl_associated_token_account::get_associated_token_address(
        &miner.pubkey(),
        &ore_api::consts::MINT_ADDRESS,
    );
    let staker_token_account_addr = spl_associated_token_account::get_associated_token_address(
        &staker.pubkey(),
        &ore_api::consts::MINT_ADDRESS,
    );

    let managed_proof_authority_token_account = get_associated_token_address(&managed_proof_authority.0, &ore_api::consts::MINT_ADDRESS);
    let managed_proof_authority_token_account = context.banks_client.get_account(managed_proof_authority_token_account).await.unwrap().unwrap();
    let miner_token_account = spl_token::state::Account::unpack(&managed_proof_authority_token_account.data).unwrap();
    let miner_token_balance = miner_token_account.amount;

    // transfer to staker from miner
    let ix0 = create_associated_token_account(&staker.pubkey(), &staker.pubkey(), &ore_api::consts::MINT_ADDRESS, &spl_token::id());
    let ix1 = spl_token::instruction::transfer_checked(&spl_token::id(), &miner_token_account_addr, &ore_api::consts::MINT_ADDRESS, &staker_token_account_addr, &miner.pubkey(), &[&miner.pubkey()], miner_token_balance, ore_api::consts::TOKEN_DECIMALS).unwrap();
    let mut tx =
        Transaction::new_with_payer(&[ix0, ix1], Some(&miner.pubkey()));

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

    // Delegate stake from staker to miner pool
    let ix0 =
        ore_miner_delegation::instruction::init_delegate_stake(staker.pubkey(), miner.pubkey());
    let ix =
        ore_miner_delegation::instruction::delegate_stake(staker.pubkey(), miner.pubkey(), miner_token_balance);

    let mut tx =
        Transaction::new_with_payer(&[ix0, ix], Some(&staker.pubkey()));

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

    let staker_delegated_stake_account = Pubkey::find_program_address(&[b"delegated-stake", staker.pubkey().as_ref(), managed_proof_account.0.as_ref()], &ore_miner_delegation::id());
    let delegated_stake = context.banks_client.get_account(staker_delegated_stake_account.0).await.unwrap().unwrap();
    let delegated_stake =
        ore_miner_delegation::state::DelegatedStake::try_from_bytes(&delegated_stake.data).unwrap();

    assert_eq!(miner_token_balance, delegated_stake.amount);


    // Unstake
    let ix =
        ore_miner_delegation::instruction::undelegate_stake(staker.pubkey(), miner.pubkey(), miner_token_balance);

    let mut tx =
        Transaction::new_with_payer(&[ix], Some(&staker.pubkey()));

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

    let staker_token_account_data = context.banks_client.get_account(staker_token_account_addr).await.unwrap().unwrap();
    let staker_token_account = spl_token::state::Account::unpack(&staker_token_account_data.data).unwrap();
    let staker_token_balance = staker_token_account.amount;

    assert_eq!(miner_token_balance, staker_token_balance);
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
