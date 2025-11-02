use soroban_sdk::{Address, Env, Symbol, U256, Vec, contract};
use soroban_sdk::{contractimpl, log};

use crate::types::RiskEngineError;
use crate::types::RiskEngineKey;

// 1.1 * e18
pub const BALANCE_TO_BORROW_THRESHOLD: u128 = 11_0000000_00000_00000;
pub const WAD_U128: u128 = 10000_0000_00000_00000; //1e18
const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;

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
        Self::extend_ttl_risk(&env, RiskEngineKey::Admin);
        Self::extend_ttl_risk(&env, RiskEngineKey::RegistryContract);
    }

    pub fn is_borrow_allowed(
        env: &Env,
        symbol: Symbol,
        borrow_amount_wad: U256,
        margin_account: Address,
    ) -> Result<bool, RiskEngineError> {
        let registry_addr = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_addr);
        let oracle_contract_addr = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_contract_addr);

        let (price, decimals) = oracle_client.get_price_latest(&symbol);
        let wad_scale = WAD_U128 / (10_u32.pow(decimals) as u128);
        let price_wad = price * wad_scale;

        let oracle_price_wad = U256::from_u128(&env, price_wad);
        let borrow_value_wad = Self::mul_wad_down(&env, borrow_amount_wad, oracle_price_wad);
        // let borrow_value = borrow_amount.mul(&oracle_price);

        let current_balance_wad = Self::get_current_total_balance(&env, margin_account.clone())?;
        let current_debt_wad = Self::get_current_total_borrows(&env, margin_account.clone())?;

        log!(
            &env,
            "Current balance and debt before {}",
            current_balance_wad,
            current_debt_wad
        );
        let res = Self::is_account_healthy(
            env,
            current_balance_wad.add(&borrow_value_wad),
            current_debt_wad.add(&borrow_value_wad),
        )?;
        Ok(res)
    }

    pub fn is_withdraw_allowed(
        env: &Env,
        symbol: Symbol,
        withdraw_amount_wad: U256,
        margin_account: Address,
    ) -> Result<bool, RiskEngineError> {
        let registry_contract: Address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_contract);

        // check has debt
        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &margin_account.clone());
        if !smart_account_contract_client.has_debt() {
            log!(&env, "Returning, since account has no debt");
            return Ok(true);
        }

        let oracle_contract_address: Address = registry_client.get_oracle_contract_address();

        let oracle_client = oracle_contract::Client::new(&env, &oracle_contract_address);
        let (price, decimals) = oracle_client.get_price_latest(&symbol);
        let wad_scale = WAD_U128 / (10_u32.pow(decimals) as u128);
        let price_wad = price * wad_scale;
        let oracle_price_wad = U256::from_u128(&env, price_wad);
        // let withdraw_value = withdraw_amount_wad.mul(&oracle_price_wad);

        let withdraw_value_wad = Self::mul_wad_down(&env, withdraw_amount_wad, oracle_price_wad);

        let current_account_balance_wad =
            Self::get_current_total_balance(&env, margin_account.clone()).unwrap();
        let current_account_debt_wad =
            Self::get_current_total_borrows(&env, margin_account.clone()).unwrap();

        let res = Self::is_account_healthy(
            env,
            current_account_balance_wad.sub(&withdraw_value_wad),
            current_account_debt_wad,
        )
        .unwrap();

        Ok(res)
    }

    pub fn is_account_healthy(
        env: &Env,
        total_account_balance_wad: U256,
        total_account_debt_wad: U256,
    ) -> Result<bool, RiskEngineError> {
        log!(
            &env,
            "Total account balance, debt",
            total_account_balance_wad,
            total_account_debt_wad
        );
        if total_account_debt_wad == U256::from_u128(&env, 0) {
            log!(&env, "Yes account is HEALTHY!");
            return Ok(true);
        } else {
            let res = (total_account_balance_wad.mul(&U256::from_u128(&env, WAD_U128)))
                .div(&total_account_debt_wad)
                > U256::from_u128(&env, BALANCE_TO_BORROW_THRESHOLD);
            log!(&env, "Is Account is healthy : ", res);
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
        log!(
            &env,
            "collateral_token_symbols are ",
            collateral_token_symbols
        );

        let oracle_address = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_address);

        let mut total_account_balance_usd_wad: U256 = U256::from_u128(&env, 0);
        for token in collateral_token_symbols.iter() {
            let token_balance_wad =
                smart_account_contract_client.get_collateral_token_balance(&token.clone());

            let (oracle_price_usd, decimals) = oracle_client.get_price_latest(&token);
            let wad_scale = WAD_U128 / (10_u32.pow(decimals) as u128);
            let oracle_price_wad_usd = oracle_price_usd * wad_scale;

            // Mutliply balance with oracle price
            let balance_wad = Self::mul_wad_down(
                &env,
                token_balance_wad,
                U256::from_u128(&env, oracle_price_wad_usd),
            );

            total_account_balance_usd_wad = total_account_balance_usd_wad.add(&balance_wad);
        }
        Ok(total_account_balance_usd_wad)
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

        let mut total_account_debt_usd_wad: U256 = U256::from_u128(&env, 0);

        for tokenx in borrowed_token_symbols.iter() {
            let token_balance_wad =
                smart_account_contract_client.get_borrowed_token_debt(&tokenx.clone());

            let oracle_contract_address: Address = registry_client.get_oracle_contract_address();
            let oracle_client = oracle_contract::Client::new(env, &oracle_contract_address);
            let (oracle_price_usd, decimals) = oracle_client.get_price_latest(&tokenx);

            let wad_scale = WAD_U128 / (10_u32.pow(decimals) as u128);
            let oracle_price_wad_usd = oracle_price_usd * wad_scale;

            log!(
                &env,
                "oracle_price_wad_usd is ",
                oracle_price_wad_usd,
                tokenx
            );

            // Mutliply balance with oracle price
            let balance_wad = Self::mul_wad_down(
                &env,
                token_balance_wad,
                U256::from_u128(&env, oracle_price_wad_usd),
            );
            log!(&env, "balance_wad is ", balance_wad, tokenx);

            total_account_debt_usd_wad = total_account_debt_usd_wad.add(&balance_wad);
            log!(
                &env,
                "total_account_debt_usd_wad is ",
                total_account_debt_usd_wad,
                tokenx
            );
        }
        Ok(total_account_debt_usd_wad)
    }

    fn extend_ttl_risk(env: &Env, key: RiskEngineKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&RiskEngineKey::RegistryContract)
            .expect("Failed to fetch registry contract address")
    }

    pub fn mul_wad_down(env: &Env, a: U256, b: U256) -> U256 {
        a.mul(&b).div(&U256::from_u128(&env, WAD_U128))
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
