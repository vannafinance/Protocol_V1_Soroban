use soroban_sdk::{Address, Env, Map, Symbol, U256, Vec, contract, symbol_short, token};
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
const BLUSDC_SYMBOL: Symbol = symbol_short!("BLUSDC");
const AQUSDC_SYMBOL: Symbol = symbol_short!("AQUSDC");
const SOUSDC_SYMBOL: Symbol = symbol_short!("SOUSDC");
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
        let oracle_addr = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_addr);
        let smart_account_client = smart_account_contract::Client::new(&env, &margin_account);

        let collateral_tokens = smart_account_client.get_all_collateral_tokens();
        let borrowed_tokens = smart_account_client.get_all_borrowed_tokens();

        let borrow_price_symbol = Self::canonical_price_symbol(env, &symbol);

        // Build price cache — fetch each unique symbol from oracle exactly once
        let mut price_cache: Map<Symbol, u128> = Map::new(env);
        Self::cache_price(env, &oracle_client, &borrow_price_symbol, &mut price_cache);
        for token in collateral_tokens.iter() {
            if !Self::is_blend_tracking_symbol(env, &token) {
                let price_symbol = Self::canonical_price_symbol(env, &token);
                Self::cache_price(env, &oracle_client, &price_symbol, &mut price_cache);
            }
        }
        for token in borrowed_tokens.iter() {
            let price_symbol = Self::canonical_price_symbol(env, &token);
            Self::cache_price(env, &oracle_client, &price_symbol, &mut price_cache);
        }

        // Borrow value
        let borrow_price_wad = price_cache.get(borrow_price_symbol).unwrap_or(0);
        let borrow_value_wad = Self::mul_wad_down(
            env,
            borrow_amount_wad,
            U256::from_u128(env, borrow_price_wad),
        );

        // Total collateral value (using cached prices)
        let mut total_balance_wad = U256::from_u128(env, 0);
        for token in collateral_tokens.iter() {
            let (token_balance_wad, price_symbol) = if Self::is_blend_tracking_symbol(env, &token) {
                let tracking_addr = registry_client.get_tracking_token_contract_addr();
                let tracking_client = tracking_token_contract::Client::new(env, &tracking_addr);
                let blend_pool_addr = registry_client.get_blend_pool_address();
                let blend_client = BlendPoolClient::new(env, &blend_pool_addr);
                let (underlying_sym, underlying_addr, underlying_dec) =
                    Self::blend_underlying_info(env, &registry_client, &token);
                let b_balance = tracking_client.balance(&margin_account, &token.clone());
                let reserve = blend_client.get_reserve(&underlying_addr);
                let underlying_amt =
                    Self::b_tokens_to_underlying(env, b_balance, reserve.data.b_rate);
                let underlying_wad = Self::scale_to_wad(env, underlying_amt, underlying_dec);
                // Ensure underlying price is cached
                Self::cache_price(env, &oracle_client, &underlying_sym, &mut price_cache);
                (underlying_wad, underlying_sym)
            } else {
                (
                    smart_account_client.get_collateral_token_balance(&token.clone()),
                    Self::canonical_price_symbol(env, &token),
                )
            };
            let price_wad = price_cache.get(price_symbol).unwrap_or(0);
            total_balance_wad = total_balance_wad.add(&Self::mul_wad_down(
                env,
                token_balance_wad,
                U256::from_u128(env, price_wad),
            ));
        }

        // Fallback safety: if collateral list is stale/missing but balance exists for the
        // queried symbol, include it so borrow checks don't false-reject.
        if !Self::is_blend_tracking_symbol(env, &symbol) && !collateral_tokens.contains(symbol.clone()) {
            let direct_bal_wad = smart_account_client.get_collateral_token_balance(&symbol);
            if direct_bal_wad > U256::from_u128(env, 0) {
                let price_symbol = Self::canonical_price_symbol(env, &symbol);
                let price_wad = price_cache.get(price_symbol).unwrap_or(0);
                total_balance_wad = total_balance_wad.add(&Self::mul_wad_down(
                    env,
                    direct_bal_wad,
                    U256::from_u128(env, price_wad),
                ));
            }
        }

        // Total debt value — call LendingPools directly (avoids SmartAccount→Registry→Pool chain)
        let mut total_debt_wad = U256::from_u128(env, 0);
        for token in borrowed_tokens.iter() {
            let token_debt_wad = Self::get_debt_direct(env, &registry_client, &token, &margin_account);
            let price_symbol = Self::canonical_price_symbol(env, &token);
            let price_wad = price_cache.get(price_symbol).unwrap_or(0);
            total_debt_wad = total_debt_wad.add(&Self::mul_wad_down(
                env,
                token_debt_wad,
                U256::from_u128(env, price_wad),
            ));
        }

        Self::is_account_healthy(
            env,
            total_balance_wad.add(&borrow_value_wad),
            total_debt_wad.add(&borrow_value_wad),
        )
    }

    pub fn is_withdraw_allowed(
        env: &Env,
        symbol: Symbol,
        withdraw_amount_wad: U256,
        margin_account: Address,
    ) -> Result<bool, RiskEngineError> {
        let smart_account_client = smart_account_contract::Client::new(&env, &margin_account);
        if !smart_account_client.has_debt() {
            return Ok(true);
        }

        let registry_addr = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_addr);
        let oracle_addr = registry_client.get_oracle_contract_address();
        let oracle_client = oracle_contract::Client::new(env, &oracle_addr);

        let collateral_tokens = smart_account_client.get_all_collateral_tokens();
        let borrowed_tokens = smart_account_client.get_all_borrowed_tokens();

        let withdraw_price_symbol = Self::canonical_price_symbol(env, &symbol);

        // Build price cache
        let mut price_cache: Map<Symbol, u128> = Map::new(env);
        Self::cache_price(env, &oracle_client, &withdraw_price_symbol, &mut price_cache);
        for token in collateral_tokens.iter() {
            if !Self::is_blend_tracking_symbol(env, &token) {
                let price_symbol = Self::canonical_price_symbol(env, &token);
                Self::cache_price(env, &oracle_client, &price_symbol, &mut price_cache);
            }
        }
        for token in borrowed_tokens.iter() {
            let price_symbol = Self::canonical_price_symbol(env, &token);
            Self::cache_price(env, &oracle_client, &price_symbol, &mut price_cache);
        }

        let withdraw_price_wad = price_cache.get(withdraw_price_symbol).unwrap_or(0);
        let withdraw_value_wad = Self::mul_wad_down(
            env,
            withdraw_amount_wad.clone(),
            U256::from_u128(env, withdraw_price_wad),
        );

        // Total collateral value (using cached prices)
        let mut total_balance_wad = U256::from_u128(env, 0);
        for token in collateral_tokens.iter() {
            let (token_balance_wad, price_symbol) = if Self::is_blend_tracking_symbol(env, &token) {
                let tracking_addr = registry_client.get_tracking_token_contract_addr();
                let tracking_client = tracking_token_contract::Client::new(env, &tracking_addr);
                let blend_pool_addr = registry_client.get_blend_pool_address();
                let blend_client = BlendPoolClient::new(env, &blend_pool_addr);
                let (underlying_sym, underlying_addr, underlying_dec) =
                    Self::blend_underlying_info(env, &registry_client, &token);
                let b_balance = tracking_client.balance(&margin_account, &token.clone());
                let reserve = blend_client.get_reserve(&underlying_addr);
                let underlying_amt =
                    Self::b_tokens_to_underlying(env, b_balance, reserve.data.b_rate);
                let underlying_wad = Self::scale_to_wad(env, underlying_amt, underlying_dec);
                Self::cache_price(env, &oracle_client, &underlying_sym, &mut price_cache);
                (underlying_wad, underlying_sym)
            } else {
                (
                    smart_account_client.get_collateral_token_balance(&token.clone()),
                    Self::canonical_price_symbol(env, &token),
                )
            };
            let price_wad = price_cache.get(price_symbol).unwrap_or(0);
            total_balance_wad = total_balance_wad.add(&Self::mul_wad_down(
                env,
                token_balance_wad,
                U256::from_u128(env, price_wad),
            ));
        }

        // Fallback safety for stale collateral list state.
        if !Self::is_blend_tracking_symbol(env, &symbol) && !collateral_tokens.contains(symbol.clone()) {
            let direct_bal_wad = smart_account_client.get_collateral_token_balance(&symbol);
            if direct_bal_wad > U256::from_u128(env, 0) {
                let price_symbol = Self::canonical_price_symbol(env, &symbol);
                let price_wad = price_cache.get(price_symbol).unwrap_or(0);
                total_balance_wad = total_balance_wad.add(&Self::mul_wad_down(
                    env,
                    direct_bal_wad,
                    U256::from_u128(env, price_wad),
                ));
            }
        }

        // Total debt value — call LendingPools directly
        let mut total_debt_wad = U256::from_u128(env, 0);
        for token in borrowed_tokens.iter() {
            let token_debt_wad = Self::get_debt_direct(env, &registry_client, &token, &margin_account);
            let price_symbol = Self::canonical_price_symbol(env, &token);
            let price_wad = price_cache.get(price_symbol).unwrap_or(0);
            total_debt_wad = total_debt_wad.add(&Self::mul_wad_down(
                env,
                token_debt_wad,
                U256::from_u128(env, price_wad),
            ));
        }

        if withdraw_amount_wad > total_balance_wad {
            panic!("Cannot withdraw more value than the current collateral value")
        }

        Self::is_account_healthy(env, total_balance_wad.sub(&withdraw_value_wad), total_debt_wad)
    }

    pub fn is_account_healthy(
        env: &Env,
        total_account_balance_wad: U256,
        total_account_debt_wad: U256,
    ) -> Result<bool, RiskEngineError> {
        if total_account_debt_wad == U256::from_u128(&env, 0) {
            return Ok(true);
        }
        let res = (total_account_balance_wad.mul(&U256::from_u128(&env, WAD_U128)))
            .div(&total_account_debt_wad)
            > U256::from_u128(&env, BALANCE_TO_BORROW_THRESHOLD);
        Ok(res)
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

        let mut total_balance_usd_wad: U256 = U256::from_u128(&env, 0);
        for token in collateral_token_symbols.iter() {
            let (token_balance_wad, price_symbol) = if Self::is_blend_tracking_symbol(&env, &token)
            {
                let tracking_token_address = registry_client.get_tracking_token_contract_addr();
                let tracking_token_client =
                    tracking_token_contract::Client::new(&env, &tracking_token_address);
                let blend_pool_address = registry_client.get_blend_pool_address();
                let blend_pool_client = BlendPoolClient::new(&env, &blend_pool_address);
                let (underlying_symbol, underlying_address, underlying_decimals) =
                    Self::blend_underlying_info(&env, &registry_client, &token);
                let b_token_balance =
                    tracking_token_client.balance(&margin_account, &token.clone());
                let reserve = blend_pool_client.get_reserve(&underlying_address);
                let b_rate = reserve.data.b_rate;
                let underlying_amount = Self::b_tokens_to_underlying(&env, b_token_balance, b_rate);
                let underlying_wad =
                    Self::scale_to_wad(&env, underlying_amount, underlying_decimals);
                (underlying_wad, underlying_symbol)
            } else {
                (
                    smart_account_contract_client.get_collateral_token_balance(&token.clone()),
                    token.clone(),
                )
            };

            let canonical_price_symbol = Self::canonical_price_symbol(&env, &price_symbol);
            let oracle_price_wad =
                Self::get_oracle_price_wad(&env, &oracle_client, &canonical_price_symbol);
            // Multiply balance with oracle price
            let balance_wad = Self::mul_wad_down(
                &env,
                token_balance_wad,
                U256::from_u128(&env, oracle_price_wad),
            );
            total_balance_usd_wad = total_balance_usd_wad.add(&balance_wad);
        }
        Ok(total_balance_usd_wad)
    }

    pub fn get_current_total_borrows(
        env: &Env,
        margin_account: Address,
    ) -> Result<U256, RiskEngineError> {
        let registry_addr = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registry_addr);
        let oracle_client = oracle_contract::Client::new(
            env, 
            &registry_client.get_oracle_contract_address()
        );

        let smart_account_client = smart_account_contract::Client::new(&env, &margin_account);
        let borrowed_tokens = smart_account_client.get_all_borrowed_tokens();

        let mut total_debt_usd_wad = U256::from_u128(&env, 0);

        for token in borrowed_tokens.iter() {
            let token_debt_wad = Self::get_debt_direct(env, &registry_client, &token, &margin_account);
            let canonical_price_symbol = Self::canonical_price_symbol(&env, &token);
            let oracle_price_wad = Self::get_oracle_price_wad(&env, &oracle_client, &canonical_price_symbol);
            let debt_value_wad = Self::mul_wad_down(
                &env,
                token_debt_wad,
                U256::from_u128(&env, oracle_price_wad),
            );
            total_debt_usd_wad = total_debt_usd_wad.add(&debt_value_wad);
        }
        Ok(total_debt_usd_wad)
    }

    /// Get a borrower's live debt directly from the LendingPool contract,
    /// bypassing the SmartAccount→Registry→LendingPool indirection.
    ///
    /// NOTE:
    /// We must use `get_borrow_balance` here (not borrow shares). Shares are an
    /// internal accounting unit and can diverge from user-visible debt, which can
    /// incorrectly make healthy accounts look under-collateralized.
    fn get_debt_direct(
        env: &Env,
        registry_client: &registry_contract::Client,
        token: &Symbol,
        margin_account: &Address,
    ) -> U256 {
        if token == &XLM_SYMBOL {
            lending_protocol_xlm::Client::new(env, &registry_client.get_lendingpool_xlm())
                .get_borrow_balance(margin_account)
        } else if token == &USDC_SYMBOL || token == &BLUSDC_SYMBOL {
            lending_protocol_usdc::Client::new(env, &registry_client.get_lendingpool_usdc())
                .get_borrow_balance(margin_account)
        } else if token == &AQUSDC_SYMBOL {
            lending_protocol_usdc::Client::new(env, &registry_client.get_lendingpool_aquarius_usdc())
                .get_borrow_balance(margin_account)
        } else if token == &SOUSDC_SYMBOL {
            lending_protocol_usdc::Client::new(env, &registry_client.get_lendingpool_soroswap_usdc())
                .get_borrow_balance(margin_account)
        } else if token == &EURC_SYMBOL {
            lending_protocol_eurc::Client::new(env, &registry_client.get_lendingpool_eurc())
                .get_borrow_balance(margin_account)
        } else {
            U256::from_u128(env, 0)
        }
    }

    fn get_oracle_price_wad(
        _env: &Env,
        oracle_client: &oracle_contract::Client,
        token: &Symbol,
    ) -> u128 {
        let (oracle_price_usd, decimals) = oracle_client.get_price_latest(&token);
        let wad_scale = WAD_U128 / 10_u128.pow(decimals);
        oracle_price_usd * wad_scale
    }

    /// Fetch oracle price into the cache only if not already present.
    /// This ensures the oracle contract is called at most once per unique symbol.
    fn cache_price(
        _env: &Env,
        oracle_client: &oracle_contract::Client,
        symbol: &Symbol,
        cache: &mut Map<Symbol, u128>,
    ) {
        if cache.get(symbol.clone()).is_none() {
            let (price, decimals) = oracle_client.get_price_latest(symbol);
            let wad_scale = WAD_U128 / 10_u128.pow(decimals);
            cache.set(symbol.clone(), price * wad_scale);
        }
    }

    fn canonical_price_symbol(env: &Env, symbol: &Symbol) -> Symbol {
        if symbol == &BLUSDC_SYMBOL || symbol == &AQUSDC_SYMBOL || symbol == &SOUSDC_SYMBOL {
            USDC_SYMBOL
        } else if symbol == &Symbol::new(env, "AQUARIUS_USDC")
            || symbol == &Symbol::new(env, "SOROSWAP_USDC")
        {
            USDC_SYMBOL
        } else {
            symbol.clone()
        }
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
        let numerator = U256::from_u128(env, b_tokens_u128).mul(&U256::from_u128(env, b_rate_u128));
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
