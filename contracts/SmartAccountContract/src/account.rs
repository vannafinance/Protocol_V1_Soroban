use core::panic;

use soroban_sdk::{Address, Env, Symbol, U256, Vec, contract, contractimpl, token};

use crate::types::{SmartAccountDataKey, SmartAccountDeactivationEvent, SmartAccountError};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

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
    }

    pub fn deactivate_account(env: Env, user_address: Address) -> Result<(), SmartAccountError> {
        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_margin_account(&env, key);
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

    pub fn activate_account(env: Env) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();
        // user_address.require_auth();
        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &true);
        Self::extend_ttl_margin_account(&env, key);

        Ok(())
    }

    pub fn withdraw_balance(
        env: Env,
        token_symbol: Symbol,
        amount: u64,
    ) -> Result<(), SmartAccountError> {
        let registry_address = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::RegistryContract)
            .expect("Failed to get registry contract address");

        let registry_client = registry_contract::Client::new(&env, &registry_address);
        if token_symbol == Symbol::new(&env, "XLM") {
            let xlm_address = registry_client.get_lendingpool_xlm();
            let xlm_client = lending_protocol_xlm::Client::new(&env, &xlm_address);
            let native_xlm_address = xlm_client.get_native_xlm_client_address();

            let xlm_token = token::Client::new(&env, &native_xlm_address);
            xlm_token.transfer(
                &env.current_contract_address(),
                &xlm_address,
                &(amount as i128),
            );
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let usdc_address = registry_client.get_lendingpool_usdc();
            let usdc_client = lending_protocol_usdc::Client::new(&env, &usdc_address);
            let native_usdc_address = usdc_client.get_native_usdc_client_address();

            let usdc_token = token::Client::new(&env, &native_usdc_address);
            usdc_token.transfer(
                &env.current_contract_address(),
                &usdc_address,
                &(amount as i128),
            );
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let eurc_address = registry_client.get_lendingpool_eurc();
            let eurc_client = lending_protocol_eurc::Client::new(&env, &eurc_address);
            let native_eurc_address = eurc_client.get_native_eurc_client_address();

            let eurc_token = token::Client::new(&env, &native_eurc_address);
            eurc_token.transfer(
                &env.current_contract_address(),
                &eurc_address,
                &(amount as i128),
            );
        }
        Ok(())
    }

    pub fn has_debt(env: &Env) -> bool {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();

        let has_debt = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::HasDebt)
            .unwrap_or_else(|| false);
        has_debt
    }

    pub fn set_has_debt(env: &Env, has_debt: bool) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::HasDebt, &has_debt);
        Self::extend_ttl_margin_account(&env, SmartAccountDataKey::HasDebt);
        Ok(())
    }

    pub fn get_all_borrowed_tokens(env: &Env) -> Result<Vec<Symbol>, SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        let borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::UserBorrowedTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        Ok(borrowed_tokens_list)
    }

    pub fn get_all_collateral_tokens(env: &Env) -> Result<Vec<Symbol>, SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        let collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::UserCollateralTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        Ok(collateral_tokens_list)
    }

    pub fn get_collateral_token_balance(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<U256, SmartAccountError> {
        let key_a = SmartAccountDataKey::UserCollateralBalance(token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        Ok(token_balance)
    }

    pub fn get_borrowed_token_debt(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<U256, SmartAccountError> {
        let key_b = SmartAccountDataKey::UserBorrowedDebt(token_symbol.clone());
        let token_debt = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        Ok(token_debt)
    }

    fn extend_ttl_margin_account(env: &Env, key: SmartAccountDataKey) {
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
}
