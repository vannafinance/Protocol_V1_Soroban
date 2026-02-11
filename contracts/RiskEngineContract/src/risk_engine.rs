use soroban_sdk::{Address, Env, Symbol, U256, Vec, contract, symbol_short, token};
use soroban_sdk::{contractimpl, log};

use blend_contract_sdk::pool::Client as BlendPoolClient;

use crate::types::RiskEngineError;
use crate::types::RiskEngineKey;

// 1.1 * e18
pub const BALANCE_TO_BORROW_THRESHOLD: u128 = 11_0000000_00000_00000;
pub const WAD_U128: u128 = 10000_0000_00000_00000; //1e18
const SCALAR_12_U128: u128 = 1_000_000_000_000; // 1e12 for Blend b_rate
const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");
const BLEND_XLM: &str = "BLEND_XLM";
const BLEND_USDC: &str = "BLEND_USDC";
const BLEND_EURC: &str = "BLEND_EURC";

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

        let withdraw_value_wad =
            Self::mul_wad_down(&env, withdraw_amount_wad.clone(), oracle_price_wad);

        let current_account_balance_wad =
            Self::get_current_total_balance(&env, margin_account.clone()).unwrap();
        let current_account_debt_wad =
            Self::get_current_total_borrows(&env, margin_account.clone()).unwrap();

        if withdraw_amount_wad > current_account_balance_wad {
            panic!("Cannot withdraw more value than the current collateral value")
        }

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
        let tracking_token_address = registry_client.get_tracking_token_contract_addr();
        let tracking_token_client =
            tracking_token_contract::Client::new(&env, &tracking_token_address);
        let blend_pool_address = registry_client.get_blend_pool_address();
        let blend_pool_client = BlendPoolClient::new(&env, &blend_pool_address);

        let mut total_account_balance_usd_wad: U256 = U256::from_u128(&env, 0);
        for token in collateral_token_symbols.iter() {
            let (token_balance_wad, price_symbol) =
                if Self::is_blend_tracking_symbol(&env, &token) {
                    let (underlying_symbol, underlying_address, underlying_decimals) =
                        Self::blend_underlying_info(&env, &registry_client, &token);
                    let b_token_balance =
                        tracking_token_client.balance(&margin_account, &token.clone());
                    let reserve = blend_pool_client.get_reserve(&underlying_address);
                    let b_rate = reserve.data.b_rate;
                    let underlying_amount =
                        Self::b_tokens_to_underlying(&env, b_token_balance, b_rate);
                    let underlying_wad =
                        Self::scale_to_wad(&env, underlying_amount, underlying_decimals);
                    (underlying_wad, underlying_symbol)
                } else {
                    (
                        smart_account_contract_client.get_collateral_token_balance(&token.clone()),
                        token.clone(),
                    )
                };

            let oracle_price_wad_usd =
                Self::get_oracle_price_wad(&env, &oracle_client, &price_symbol);
            // Multiply balance with oracle price
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
        let oracle_contract_address: Address = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_contract_address);

        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &margin_account.clone());

        let borrowed_token_symbols = smart_account_contract_client.get_all_borrowed_tokens();

        let mut total_account_debt_usd_wad: U256 = U256::from_u128(&env, 0);

        for tokenx in borrowed_token_symbols.iter() {
            let token_balance_wad =
                smart_account_contract_client.get_borrowed_token_debt(&tokenx.clone());

            let oracle_price_wad_usd = Self::get_oracle_price_wad(&env, &oracle_client, &tokenx);

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

    fn get_oracle_price_wad(
        env: &Env,
        oracle_client: &oracle_contract::Client,
        token: &Symbol,
    ) -> u128 {
        let (oracle_price_usd, decimals) = oracle_client.get_price_latest(&token);
        let wad_scale = WAD_U128 / (10_u32.pow(decimals) as u128);
        let oracle_price_wad_usd = oracle_price_usd * wad_scale;
        oracle_price_wad_usd
    }

    fn is_blend_tracking_symbol(env: &Env, symbol: &Symbol) -> bool {
        symbol == &Symbol::new(env, BLEND_XLM)
            || symbol == &Symbol::new(env, BLEND_USDC)
            || symbol == &Symbol::new(env, BLEND_EURC)
    }

    fn blend_underlying_info(
        env: &Env,
        registry_client: &registry_contract::Client,
        tracking_symbol: &Symbol,
    ) -> (Symbol, Address, u32) {
        if tracking_symbol == &Symbol::new(env, BLEND_XLM) {
            let addr = registry_client.get_xlm_contract_adddress();
            let decimals = token::Client::new(env, &addr).decimals();
            (XLM_SYMBOL, addr, decimals)
        } else if tracking_symbol == &Symbol::new(env, BLEND_USDC) {
            let addr = registry_client.get_usdc_contract_address();
            let decimals = token::Client::new(env, &addr).decimals();
            (USDC_SYMBOL, addr, decimals)
        } else if tracking_symbol == &Symbol::new(env, BLEND_EURC) {
            let addr = registry_client.get_eurc_contract_address();
            let decimals = token::Client::new(env, &addr).decimals();
            (EURC_SYMBOL, addr, decimals)
        } else {
            panic!("Unsupported blend tracking symbol");
        }
    }

    fn b_tokens_to_underlying(env: &Env, b_tokens: i128, b_rate: i128) -> U256 {
        if b_tokens <= 0 || b_rate <= 0 {
            return U256::from_u128(env, 0);
        }
        let b_tokens_u128 = b_tokens as u128;
        let b_rate_u128 = b_rate as u128;
        let numerator =
            U256::from_u128(env, b_tokens_u128).mul(&U256::from_u128(env, b_rate_u128));
        numerator.div(&U256::from_u128(env, SCALAR_12_U128))
    }

    fn scale_to_wad(env: &Env, amount: U256, token_decimals: u32) -> U256 {
        let scale = U256::from_u128(env, 10u128.pow(token_decimals));
        amount.mul(&U256::from_u128(env, WAD_U128)).div(&scale)
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
        let x = a.mul(&b);
        x.div(&U256::from_u128(&env, WAD_U128))
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

pub mod registry_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/registry_contract.wasm"
    );
}

pub mod tracking_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/tracking_token_contract.wasm"
    );
}
