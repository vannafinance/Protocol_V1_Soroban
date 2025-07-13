use crate::errors::LendingError;
use crate::types::{DataKey, PoolDataKey};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, Address, Env, String, Symbol, Vec, U256,
};

#[contract]
pub struct LiquidityPoolXLM;

#[contractimpl]
impl LiquidityPoolXLM {
    pub fn initialize_pool_xlm(env: Env) {
        // Verify contract is deployed
        if !env.storage().persistent().has(&PoolDataKey::Deployed) {
            panic!("Contract not deployed");
        }

        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("Admin not set");

        admin.require_auth();

        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(Symbol::new(&env, "XLM")), &0); // Store the XLM this contract handles
    }

    pub fn deposit(env: Env, lender: Address, amount: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_pool_initialised(&env, Symbol::new(&env, "XLM"));

        // Update lender list
        Self::add_lender(&env, &lender);

        // Adding amount to Lenders balance, first check current balance
        let current_balance: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::LenderBalance(
                lender.clone(),
                Symbol::new(&env, "XLM"),
            ))
            .unwrap_or(U256::from_u128(&env, 0)); // Use U256::from_u128 or U256::zero to initialize U256
        let new_balance = current_balance.add(&amount);

        env.storage().persistent().set(
            &PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "XLM")),
            &new_balance,
        );

        // Adding same amount to Total Pool balance
        let current_pool: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
            .unwrap_or(U256::from_u128(&env, 0));
        let new_pool = current_pool.add(&amount);

        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(Symbol::new(&env, "XLM")), &(new_pool));

        // Now Mint the vXLM tokens that were created for the lender

        // WORK IN PROGRESS

        // Update user balance with interest-bearing token
        let account_contract = Address::from_string(&String::from_str(&env, "ACCOUNT_CONTRACT_ID")); // Replace with actual ID
        let v_asset = String::from_str(&env, "vXLM");
        // AccountContractClient::new(&env, &account_contract)
        //     .update_balance(&lender, &v_asset, &amount, &true);
    }

    pub fn withdraw(env: Env, lender: Address, amount: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_pool_initialised(&env, Symbol::new(&env, "XLM"));

        // Check if lender has registered
        if !env.storage().persistent().has(&PoolDataKey::LenderBalance(
            lender.clone(),
            Symbol::new(&env, "XLM"),
        )) {
            panic_with_error!(&env, LendingError::LenderNotRegistered);
        }

        // Check if lender has enough balance to deduct
        let current_balance: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::LenderBalance(
                lender.clone(),
                Symbol::new(&env, "XLM"),
            ))
            .unwrap();

        if current_balance < amount {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }

        // First deduct amount from Lenders balance
        let new_balance = current_balance.sub(&amount);

        env.storage().persistent().set(
            &PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "XLM")),
            &new_balance,
        );

        // Deduct same amount from pool balance
        let current_pool: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::InsufficientBalance));
        if current_pool < amount {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }
        env.storage().persistent().set(
            &PoolDataKey::Pool(Symbol::new(&env, "XLM")),
            &(current_pool.sub(&amount)),
        );

        // Now burn the vXLM tokens that were created for the lender
        // WORK IN PROGRESS

        let account_contract = Address::from_string(&String::from_str(&env, "ACCOUNT_CONTRACT_ID")); // Replace with actual ID
        let v_asset = String::from_str(&env, "vXLM");
        // AccountContractClient::new(&env, &account_contract)
        //     .update_balance(&lender, &v_asset, &amount, &false);
    }

    pub fn get_pool_balance(env: Env) -> U256 {
        env.storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    // Helper function to add lender to list
    fn add_lender(env: &Env, lender: &Address) {
        let mut lenders: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")))
            .unwrap_or_else(|| Vec::new(&env));

        if !lenders.contains(lender) {
            lenders.push_back(lender.clone());
            env.storage()
                .persistent()
                .set(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")), &lenders);
        }
    }

    // Function to get all lenders
    pub fn get_lenders(env: Env) -> Vec<Address> {
        let list_address: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")))
            .unwrap_or_else(|| Vec::new(&env));
        list_address
    }

    pub fn is_pool_initialised(env: &Env, asset: Symbol) -> bool {
        if !env.storage().persistent().has(&PoolDataKey::Pool(asset)) {
            panic_with_error!(&env, LendingError::PoolNotInitialized);
        }
        true
    }

    pub fn mint_vXLMtokens(env: Env) {
        // WORK IN PROGRESS
    }
}
