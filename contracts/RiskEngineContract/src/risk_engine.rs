use soroban_sdk::contractimpl;
use soroban_sdk::{Address, Env, Symbol, U256, Vec, contract};

use crate::types::RiskEngineError;
use crate::types::{AccountDataKey, RiskEngineKey};

const BALANCE_TO_BORROW_THRESHOLD: u128 = 1100000000000000000;

#[contract]
pub struct RiskEngineContract;

#[contractimpl]
impl RiskEngineContract {
    pub fn __constructor(env: &Env, admin: Address, registry_contract: Address) {
        env.storage()
            .persistent()
            .set(&RiskEngineKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&RiskEngineKey::RegistryContract, &registry_contract);
    }

    pub fn is_borrow_allowed(
        env: &Env,
        symbol: Symbol,
        borrow_amount: U256,
        margin_account: Address,
    ) -> Result<bool, RiskEngineError> {
        //  Fetch price from oracle

        let registry_contract: Address = env
            .storage()
            .persistent()
            .get(&RiskEngineKey::RegistryContract)
            .expect("Failed to get registry contract address !");
        let registry_client = registry_contract::Client::new(&env, &registry_contract);
        let oracle_contract_address: Address = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_contract_address);

        let price = oracle_client.get_price_of(&(symbol, Symbol::new(&env, "USD")));
        let oracle_price = U256::from_u128(&env, price);
        let borrow_value = borrow_amount.mul(&oracle_price);

        let current_account_balance =
            Self::get_current_total_balance(&env, margin_account.clone()).unwrap();
        let current_account_debt =
            Self::get_current_total_borrows(&env, margin_account.clone()).unwrap();
        let res = Self::is_account_healthy(
            env,
            current_account_balance.add(&borrow_value),
            current_account_debt.add(&borrow_value),
        )
        .unwrap();
        Ok(res)
    }

    pub fn is_withdraw_allowed(
        env: &Env,
        symbol: Symbol,
        withdraw_amount: U256,
        margin_account: Address,
    ) -> Result<bool, RiskEngineError> {
        let registry_contract: Address = env
            .storage()
            .persistent()
            .get(&RiskEngineKey::RegistryContract)
            .expect("Failed to get registry contract address !");
        let registry_client = registry_contract::Client::new(&env, &registry_contract);

        // check has debt
        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &margin_account.clone());
        if !smart_account_contract_client.has_debt() {
            return Ok(true);
        }

        let oracle_contract_address: Address = registry_client.get_oracle_contract_address();

        let oracle_client = oracle_contract::Client::new(&env, &oracle_contract_address);
        let price = oracle_client.get_price_of(&(symbol, Symbol::new(&env, "USD")));
        let oracle_price = U256::from_u128(&env, price);
        let withdraw_value = withdraw_amount.mul(&oracle_price);

        let current_account_balance =
            Self::get_current_total_balance(&env, margin_account.clone()).unwrap();
        let current_account_debt =
            Self::get_current_total_borrows(&env, margin_account.clone()).unwrap();

        let res = Self::is_account_healthy(
            env,
            current_account_balance.sub(&withdraw_value),
            current_account_debt,
        )
        .unwrap();

        Ok(res)
    }

    pub fn is_account_healthy(
        env: &Env,
        total_account_balance: U256,
        total_account_debt: U256,
    ) -> Result<bool, RiskEngineError> {
        if total_account_debt == U256::from_u128(&env, 0) {
            return Ok(true);
        } else {
            let res = total_account_balance.div(&total_account_debt)
                > U256::from_u128(&env, BALANCE_TO_BORROW_THRESHOLD);
            return Ok(res);
        }
    }

    pub fn get_current_total_balance(
        env: &Env,
        margin_account: Address,
    ) -> Result<U256, RiskEngineError> {
        let registry_address: Address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &margin_account.clone());
        let collateral_token_symbols: Vec<Symbol> =
            smart_account_contract_client.get_all_collateral_tokens();

        let oracle_address = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_address);

        let mut total_account_balance: U256 = U256::from_u128(&env, 0);
        for token in collateral_token_symbols.iter() {
            let token_balance =
                smart_account_contract_client.get_collateral_token_balance(&token.clone());

            let oracle_price_usd = oracle_client.get_price_of(&(token, Symbol::new(&env, "USD")));

            total_account_balance = total_account_balance
                .add(&token_balance.mul(&U256::from_u128(&env, oracle_price_usd)));
        }
        Ok(total_account_balance)
    }

    pub fn get_current_total_borrows(
        env: &Env,
        margin_account: Address,
    ) -> Result<U256, RiskEngineError> {
        let registry_address: Address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &margin_account.clone());

        let borrowed_token_symbols = smart_account_contract_client.get_all_borrowed_tokens();

        let mut total_account_debt: U256 = U256::from_u128(&env, 0);

        for tokenx in borrowed_token_symbols.iter() {
            let token_balance =
                smart_account_contract_client.get_borrowed_token_debt(&tokenx.clone());

            let oracle_contract_address: Address = registry_client.get_oracle_contract_address();
            let oracle_client = oracle_contract::Client::new(env, &oracle_contract_address);
            let oracle_price_usd = oracle_client.get_price_of(&(tokenx, Symbol::new(&env, "USD")));

            total_account_debt = total_account_debt
                .add(&token_balance.mul(&U256::from_u128(&env, oracle_price_usd)));
        }
        Ok(total_account_debt)
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&RiskEngineKey::RegistryContract)
            .expect("Failed to fetch registry contract address")
    }
}

pub mod oracle_contract {
    // soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/oracle_contract.wasm");
}

pub mod smart_account_contract {
    // soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/smart_account_contract.wasm"
    );
}

pub mod rate_model_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/rate_model_contract.wasm"
    );
}

pub mod lending_protocol_xlm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_xlm.wasm"
    );
}

pub mod lending_protocol_usdc {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_usdc.wasm"
    );
}

pub mod lending_protocol_eurc {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_eurc.wasm"
    );
}

pub mod registry_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/registry_contract.wasm"
    );
}
