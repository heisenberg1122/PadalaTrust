#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup_env<'a>() -> (Env, PadalaTrustContractClient<'a>, Address, Address, token::Client<'a>) {
    let env = Env::default();
    env.mock_all_auths();

    let ofw_sender = Address::generate(&env);
    let university = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let token_client = token::Client::new(&env, &token_contract);
    let token_admin_client = token::AdminClient::new(&env, &token_contract);

    // Provide the OFW with 2000 mock USDC
    token_admin_client.mint(&ofw_sender, &2000);

    let contract_id = env.register_contract(None, PadalaTrustContract);
    let client = PadalaTrustContractClient::new(&env, &contract_id);

    (env, client, ofw_sender, university, token_client)
}

#[test]
fn test_1_happy_path_lock_and_claim() {
    let (_, client, ofw_sender, university, token_client) = setup_env();
    let student_id = 2024001;
    let tuition_amount = 500;

    // OFW locks funds for the student
    client.lock_tuition(&ofw_sender, &university, &student_id, &token_client.address, &tuition_amount);
    
    // Ensure vault holds the funds
    assert_eq!(token_client.balance(&client.address), tuition_amount);
    assert_eq!(token_client.balance(&ofw_sender), 1500);

    // University claims funds
    client.claim_tuition(&university, &student_id);

    // Verify university received funds and vault is empty
    assert_eq!(token_client.balance(&university), tuition_amount);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
#[should_panic(expected = "Only the designated university can claim these funds")]
fn test_2_edge_case_unauthorized_claim() {
    let (env, client, ofw_sender, university, token_client) = setup_env();
    let hacker = Address::generate(&env);
    
    client.lock_tuition(&ofw_sender, &university, &12345, &token_client.address, &500);
    
    // Random address attempts to claim the locked funds
    client.claim_tuition(&hacker, &12345);
}

#[test]
#[should_panic(expected = "No tuition locked for this student ID")]
fn test_3_edge_case_invalid_student_id() {
    let (_, client, ofw_sender, university, token_client) = setup_env();
    
    client.lock_tuition(&ofw_sender, &university, &1111, &token_client.address, &500);
    
    // University attempts to claim with a typo in the student ID
    client.claim_tuition(&university, &9999);
}

#[test]
fn test_4_state_verification_and_recall() {
    let (env, client, ofw_sender, university, token_client) = setup_env();
    let student_id = 5555;
    
    client.lock_tuition(&ofw_sender, &university, &student_id, &token_client.address, &1000);

    // Verify correct mapping in smart contract storage
    let stored_lock: TuitionLock = env
        .as_contract(&client.address, || env.storage().instance().get(&DataKey::Tuition(student_id)))
        .unwrap();

    assert_eq!(stored_lock.sender, ofw_sender);
    assert_eq!(stored_lock.university, university);
    assert_eq!(stored_lock.amount, 1000);

    // Sender decides to recall funds (e.g., student deferred enrollment)
    client.recall_funds(&ofw_sender, &student_id);
    
    // Verify funds returned to OFW
    assert_eq!(token_client.balance(&ofw_sender), 2000);
}

#[test]
#[should_panic(expected = "A tuition lock already exists for this student ID")]
fn test_5_edge_case_duplicate_lock() {
    let (_, client, ofw_sender, university, token_client) = setup_env();
    let student_id = 777;
    
    // Create first lock
    client.lock_tuition(&ofw_sender, &university, &student_id, &token_client.address, &500);
    
    // Attempt to override the active lock for the same student before it is claimed
    client.lock_tuition(&ofw_sender, &university, &student_id, &token_client.address, &500);
}