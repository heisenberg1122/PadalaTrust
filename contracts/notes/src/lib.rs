#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Tuition(u64), // Mapped by Student ID
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuitionLock {
    pub sender: Address,
    pub university: Address,
    pub token: Address,
    pub amount: i128,
}

#[contract]
pub struct PadalaTrustContract;

#[contractimpl]
impl PadalaTrustContract {
    /// Locks remittance funds intended strictly for a specific student's tuition.
    /// The OFW (sender) authorizes this transaction.
    pub fn lock_tuition(
        env: Env,
        sender: Address,
        university: Address,
        student_id: u64,
        token: Address,
        amount: i128,
    ) {
        sender.require_auth();

        if amount <= 0 {
            panic!("Tuition amount must be greater than zero");
        }

        if env.storage().instance().has(&DataKey::Tuition(student_id)) {
            panic!("A tuition lock already exists for this student ID");
        }

        let lock = TuitionLock {
            sender: sender.clone(),
            university,
            token: token.clone(),
            amount,
        };

        // Transfer funds from the OFW to the smart contract vault
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        env.storage().instance().set(&DataKey::Tuition(student_id), &lock);
    }

    /// The designated University claims the tuition funds by providing the matching Student ID.
    pub fn claim_tuition(env: Env, caller: Address, student_id: u64) {
        caller.require_auth();

        let lock: TuitionLock = env
            .storage()
            .instance()
            .get(&DataKey::Tuition(student_id))
            .expect("No tuition locked for this student ID");

        if caller != lock.university {
            panic!("Only the designated university can claim these funds");
        }

        let token_client = token::Client::new(&env, &lock.token);
        
        // Disburse funds directly to the university
        token_client.transfer(&env.current_contract_address(), &lock.university, &lock.amount);

        // Remove the lock state after successful claim
        env.storage().instance().remove(&DataKey::Tuition(student_id));
    }

    /// Allows the OFW to recall the funds if the student drops out or university rejects.
    pub fn recall_funds(env: Env, caller: Address, student_id: u64) {
        caller.require_auth();

        let lock: TuitionLock = env
            .storage()
            .instance()
            .get(&DataKey::Tuition(student_id))
            .expect("No tuition locked for this student ID");

        if caller != lock.sender {
            panic!("Only the original sender can recall funds");
        }

        let token_client = token::Client::new(&env, &lock.token);
        
        // Return funds back to the OFW
        token_client.transfer(&env.current_contract_address(), &lock.sender, &lock.amount);

        // Clean up state
        env.storage().instance().remove(&DataKey::Tuition(student_id));
    }
}