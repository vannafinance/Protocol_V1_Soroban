use core::panic;

use soroban_sdk::{Address, Env, Symbol, U256, Vec, contract, contractimpl, token};

use crate::types::{SmartAccountDataKey, SmartAccountDeactivationEvent, SmartAccountError};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

#[contract]
pub struct SmartAccountContract;

#[contractimpl]
impl SmartAccountContract {
    pub fn __constructor(
        env: Env,
        account_manager: Address,
        registry_contract: Address,
        user_address: Address,
    ) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::AccountManager, &account_manager);

        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::RegistryContract, &registry_contract);

        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::OwnerAddress, &user_address);

        Self::activate_account(&env).expect("Failed to activate account");
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::AccountManager);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::RegistryContract);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::OwnerAddress);
    }

    pub fn deactivate_account(env: Env, user_address: Address) -> Result<(), SmartAccountError> {
        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_smart_account(&env, key);
        env.events().publish(
            (
                Symbol::new(&env, "Account_Deactivated"),
                user_address.clone(),
            ),
            SmartAccountDeactivationEvent {
                margin_account: env.current_contract_address(),
                deactivate_time: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn activate_account(env: &Env) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();
        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &true);
        Self::extend_ttl_smart_account(&env, key);
        Ok(())
    }

    pub fn remove_borrowed_token_balance(
        env: Env,
        token_symbol: Symbol,
        amount: u128,
    ) -> Result<(), SmartAccountError> {
        Self::check_auth(&env, token_symbol.clone()).unwrap();

        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        if token_symbol == Symbol::new(&env, "XLM") {
            let pool_xlm_address = registry_client.get_lendingpool_xlm();
            let xlm_client = lending_protocol_xlm::Client::new(&env, &pool_xlm_address);
            let native_xlm_address = xlm_client.get_native_xlm_client_address();

            let xlm_token = token::Client::new(&env, &native_xlm_address);
            xlm_token.transfer(
                &env.current_contract_address(),
                &pool_xlm_address,
                &(amount as i128),
            );
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            let usdc_client = lending_protocol_usdc::Client::new(&env, &pool_usdc_address);
            let native_usdc_address = usdc_client.get_native_usdc_client_address();

            let usdc_token = token::Client::new(&env, &native_usdc_address);
            usdc_token.transfer(
                &env.current_contract_address(),
                &pool_usdc_address,
                &(amount as i128),
            );
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let pool_eurc_address = registry_client.get_lendingpool_eurc();
            let eurc_client = lending_protocol_eurc::Client::new(&env, &pool_eurc_address);
            let native_eurc_address = eurc_client.get_native_eurc_client_address();

            let eurc_token = token::Client::new(&env, &native_eurc_address);
            eurc_token.transfer(
                &env.current_contract_address(),
                &pool_eurc_address,
                &(amount as i128),
            );
        }
        Ok(())
    }

    pub fn remove_collateral_token_balance(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
        amount: u128,
    ) -> Result<(), SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();

        if token_symbol == Symbol::new(&env, "XLM") {
            let native_xlm_address = registry_client.get_xlm_token_contract_adddress();

            let xlm_token = token::Client::new(&env, &native_xlm_address);
            xlm_token.transfer(
                &env.current_contract_address(),
                &user_address,
                &(amount as i128),
            );
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let native_usdc_address = registry_client.get_usdc_contract_address();

            let usdc_token = token::Client::new(&env, &native_usdc_address);
            usdc_token.transfer(
                &env.current_contract_address(),
                &user_address,
                &(amount as i128),
            );
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let native_eurc_address = registry_client.get_eurc_contract_address();

            let eurc_token = token::Client::new(&env, &native_eurc_address);
            eurc_token.transfer(
                &env.current_contract_address(),
                &user_address,
                &(amount as i128),
            );
        }

        let collateral_balance =
            Self::get_collateral_token_balance(&env, token_symbol.clone()).unwrap();
        let balance_after_deduction = collateral_balance.sub(&U256::from_u128(&env, amount));
        Self::set_collateral_token_balance(
            &env,
            token_symbol.clone(),
            balance_after_deduction.clone(),
        )
        .unwrap();

        if balance_after_deduction == U256::from_u128(&env, 0) {
            Self::remove_collateral_token(&env, token_symbol.clone()).unwrap();
        }

        Ok(())
    }

    pub fn has_debt(env: &Env) -> bool {
        let has_debt = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::HasDebt)
            .unwrap_or_else(|| false);
        has_debt
    }

    pub fn set_has_debt(
        env: &Env,
        has_debt: bool,
        token_symbol: Symbol,
    ) -> Result<(), SmartAccountError> {
        Self::check_auth(&env, token_symbol).unwrap();
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::HasDebt, &has_debt);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::HasDebt);
        Ok(())
    }

    // flaw !! where is add borrowed tokens?
    pub fn get_all_borrowed_tokens(env: &Env) -> Result<Vec<Symbol>, SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        let borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::BorrowedTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        Ok(borrowed_tokens_list)
    }

    pub fn add_borrowed_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        Self::check_auth(&env, token_symbol.clone()).unwrap();
        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::BorrowedTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        if !borrowed_tokens_list.contains(&token_symbol.clone()) {
            borrowed_tokens_list.push_back(token_symbol);
        }
        env.storage().persistent().set(
            &SmartAccountDataKey::BorrowedTokensList,
            &borrowed_tokens_list,
        );
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::BorrowedTokensList);
        Ok(())
    }

    pub fn remove_borrowed_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        Self::check_auth(&env, token_symbol.clone()).unwrap();
        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::BorrowedTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        if borrowed_tokens_list.contains(&token_symbol.clone()) {
            let index = borrowed_tokens_list
                .first_index_of(token_symbol.clone())
                .unwrap();
            borrowed_tokens_list.remove(index);
        }

        if borrowed_tokens_list.is_empty() {
            Self::set_has_debt(&env, false, token_symbol).unwrap();
        }

        env.storage().persistent().set(
            &SmartAccountDataKey::BorrowedTokensList,
            &borrowed_tokens_list,
        );
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::BorrowedTokensList);
        Ok(())
    }

    pub fn get_all_collateral_tokens(env: &Env) -> Result<Vec<Symbol>, SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        let collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::CollateralTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        Ok(collateral_tokens_list)
    }

    pub fn add_collateral_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let mut exisiting_tokens = Self::get_all_collateral_tokens(&env).unwrap();
        if !exisiting_tokens.contains(&token_symbol) {
            exisiting_tokens.push_back(token_symbol);
        }

        env.storage().persistent().set(
            &SmartAccountDataKey::CollateralTokensList,
            &exisiting_tokens,
        );
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::CollateralTokensList);
        Ok(())
    }

    pub fn remove_collateral_token(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<(), SmartAccountError> {
        let mut existing_tokens: Vec<Symbol> = Self::get_all_collateral_tokens(&env).unwrap();
        if existing_tokens.contains(&token_symbol) {
            let index = existing_tokens.first_index_of(&token_symbol).unwrap();
            existing_tokens.remove(index);
        }
        Ok(())
    }

    pub fn get_collateral_token_balance(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<U256, SmartAccountError> {
        let key_a = SmartAccountDataKey::CollateralBalance(token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        Ok(token_balance)
    }

    pub fn set_collateral_token_balance(
        env: &Env,
        token_symbol: Symbol,
        balance: U256,
    ) -> Result<(), SmartAccountError> {
        let key_a = SmartAccountDataKey::CollateralBalance(token_symbol.clone());
        env.storage().persistent().set(&key_a, &balance);
        Self::extend_ttl_smart_account(&env, key_a);
        Ok(())
    }

    pub fn get_borrowed_token_debt(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<U256, SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        if token_symbol == Symbol::new(&env, "XLM") {
            let pool_xlm_address = registry_client.get_lendingpool_xlm();
            let xlm_client = lending_protocol_xlm::Client::new(&env, &pool_xlm_address);

            let xlm_debt = xlm_client.get_borrow_balance(&env.current_contract_address());
            return Ok(xlm_debt);
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            let usdc_client = lending_protocol_usdc::Client::new(&env, &pool_usdc_address);

            let usdc_debt = usdc_client.get_borrow_balance(&env.current_contract_address());
            return Ok(usdc_debt);
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let pool_eurc_address = registry_client.get_lendingpool_eurc();
            let eurc_client = lending_protocol_eurc::Client::new(&env, &pool_eurc_address);

            let eurc_debt = eurc_client.get_borrow_balance(&env.current_contract_address());
            return Ok(eurc_debt);
        } else {
            panic!("User doen't have a borrows in the given token");
        }

        // let key_b = SmartAccountDataKey::BorrowedDebt(token_symbol.clone());
        // let token_debt = env
        //     .storage()
        //     .persistent()
        //     .get(&key_b)
        //     .unwrap_or_else(|| U256::from_u128(&env, 0));
        // Ok(token_debt)
    }

    fn check_auth(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        if token_symbol == Symbol::new(&env, "XLM") {
            let pool_xlm_address = registry_client.get_lendingpool_xlm();
            // make sure only the lending pool has auth to call this function by adding authorization
            pool_xlm_address.require_auth();
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            // make sure only the lending pool has auth to call this function by adding authorization
            pool_usdc_address.require_auth();
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let pool_eurc_address = registry_client.get_lendingpool_eurc();
            // make sure only the lending pool has auth to call this function by adding authorization
            pool_eurc_address.require_auth();
        }
        Ok(())
    }

    fn extend_ttl_smart_account(env: &Env, key: SmartAccountDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn get_account_manager(env: &Env) -> Address {
        let account_manager: Address = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::AccountManager)
            .unwrap_or_else(|| panic!("Failed to get account manager address"));
        account_manager
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&SmartAccountDataKey::RegistryContract)
            .expect("Failed to get registry contract address")
    }
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
