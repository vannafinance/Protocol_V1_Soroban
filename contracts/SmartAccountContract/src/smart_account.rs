use core::panic;

use soroban_sdk::{
    Address, Env, Symbol, U256, Vec, contract, contractimpl, log, panic_with_error, symbol_short,
    token,
};

use crate::types::{
    SmartAccExternalAction, SmartAccountActivationEvent, SmartAccountDataKey,
    SmartAccountDeactivationEvent, SmartAccountError,
};

use blend_contract_sdk::pool::{self, Positions, Reserve};
use blend_contract_sdk::pool::{PoolConfig, ReserveConfig, ReserveData};

use blend_contract_sdk::pool::{Client as BlendPoolClient, Request};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const WAD_U128: u128 = 10000_0000_00000_00000; // 10^18 for decimals
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");

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

        let key = SmartAccountDataKey::IsAccountActive;
        // When deployed the smart account is inactive, which should be activated explicitly
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_smart_account(&env, key);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::AccountManager);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::RegistryContract);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::OwnerAddress);
    }

    pub fn deactivate_account(env: &Env) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_smart_account(&env, key);
        env.events().publish(
            (Symbol::new(&env, "Smart_Account_Deactivated"),),
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
        env.events().publish(
            (Symbol::new(&env, "Smart_Account_Activated"),),
            SmartAccountActivationEvent {
                margin_account: env.current_contract_address(),
                activated_time: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn remove_borrowed_token_balance(
        env: Env,
        token_symbol: Symbol,
        amount_wad: u128,
    ) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let this_account = env.current_contract_address();

        if token_symbol == XLM_SYMBOL {
            let pool_xlm_address = registry_client.get_lendingpool_xlm();
            let native_xlm_address = registry_client.get_xlm_contract_adddress();
            let xlm_token = token::Client::new(&env, &native_xlm_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, xlm_token.decimals());
            xlm_token.transfer(&this_account, &pool_xlm_address, &amount_scaled);
        } else if token_symbol == USDC_SYMBOL {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            let native_usdc_address = registry_client.get_usdc_contract_address();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &pool_usdc_address, &amount_scaled);
        } else if token_symbol == EURC_SYMBOL {
            let pool_eurc_address = registry_client.get_lendingpool_eurc();
            let native_eurc_address = registry_client.get_eurc_contract_address();
            let eurc_token = token::Client::new(&env, &native_eurc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, eurc_token.decimals());
            eurc_token.transfer(&this_account, &pool_eurc_address, &amount_scaled);
        }
        Ok(())
    }

    pub fn remove_collateral_token_balance(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
        amount_wad: u128,
    ) -> Result<(), SmartAccountError> {
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();
        Self::remove_collateral_token_bal_internal(env, user_address, token_symbol, amount_wad)
    }

    fn remove_collateral_token_bal_internal(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
        amount_wad: u128,
    ) -> Result<(), SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let this_account = env.current_contract_address();

        if token_symbol == XLM_SYMBOL {
            let native_xlm_address = registry_client.get_xlm_contract_adddress();
            let xlm_token = token::Client::new(&env, &native_xlm_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, xlm_token.decimals());
            let bal_before = xlm_token.balance(&this_account);
            xlm_token.transfer(&this_account, &user_address, &amount_scaled);
            let bal_after = xlm_token.balance(&this_account);
            log!(
                &env,
                "Transfering xlm ",
                amount_scaled,
                bal_before,
                bal_after
            );
        } else if token_symbol == USDC_SYMBOL {
            let native_usdc_address = registry_client.get_usdc_contract_address();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &user_address, &amount_scaled);
        } else if token_symbol == EURC_SYMBOL {
            let native_eurc_address = registry_client.get_eurc_contract_address();
            let eurc_token = token::Client::new(&env, &native_eurc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, eurc_token.decimals());
            eurc_token.transfer(&this_account, &user_address, &amount_scaled);
        }

        let collateral_balance_wad = Self::get_collateral_token_balance(&env, token_symbol.clone());
        let balance_after_deduction_wad =
            collateral_balance_wad.sub(&U256::from_u128(&env, amount_wad));

        Self::set_collateral_token_bal_internal(
            env,
            token_symbol.clone(),
            balance_after_deduction_wad.clone(),
        );

        if balance_after_deduction_wad == U256::from_u128(&env, 0) {
            Self::remove_collateral_token(&env, token_symbol.clone()).unwrap();
        }

        Ok(())
    }

    pub fn sweep_to(env: &Env, to_address: Address) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let all_collateral_tokens = Self::get_all_collateral_tokens(env);
        for coltoken in all_collateral_tokens.iter() {
            let coltokenbalance = Self::get_collateral_token_balance(env, coltoken.clone());

            let col_token_amount = coltokenbalance.to_u128().unwrap_or_else(|| {
                panic_with_error!(&env, SmartAccountError::IntegerConversionError)
            });

            Self::remove_collateral_token_bal_internal(
                env,
                to_address.clone(),
                coltoken,
                col_token_amount,
            )
            .expect("Failed to remove collateral token balance");
        }
        Ok(())
    }

    pub fn has_debt(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&SmartAccountDataKey::HasDebt)
            .unwrap_or_else(|| false)
    }

    pub fn set_has_debt(env: &Env, has_debt: bool) {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        Self::set_has_debt_internal(env, has_debt);
    }

    fn set_has_debt_internal(env: &Env, has_debt: bool) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::HasDebt, &has_debt);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::HasDebt);
    }

    pub fn get_all_borrowed_tokens(env: &Env) -> Vec<Symbol> {
        let borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::BorrowedTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        borrowed_tokens_list
    }

    pub fn add_borrowed_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();
        let mut borrowed_tokens_list: Vec<Symbol> = Self::get_all_borrowed_tokens(env);
        if !borrowed_tokens_list.contains(&token_symbol.clone()) {
            borrowed_tokens_list.push_back(token_symbol);
        }
        Self::set_borrowed_token_list(env, borrowed_tokens_list);
        Ok(())
    }

    pub fn remove_borrowed_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let mut borrowed_tokens_list: Vec<Symbol> = Self::get_all_borrowed_tokens(env);
        if borrowed_tokens_list.contains(&token_symbol.clone()) {
            let index = borrowed_tokens_list
                .first_index_of(token_symbol.clone())
                .unwrap();
            borrowed_tokens_list.remove(index);
        }

        if borrowed_tokens_list.is_empty() {
            Self::set_has_debt_internal(&env, false);
        }
        Self::set_borrowed_token_list(env, borrowed_tokens_list);
        Ok(())
    }

    pub fn execute(
        env: &Env,
        target_protocol: Address,
        action: SmartAccExternalAction,
        trader_address: Address,
        tokens: Vec<Symbol>,
        tokens_amount_wad: Vec<u128>,
    ) -> Result<(bool, i128), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let registry_address = Self::get_registry_address(&env);

        let registry_client = registry_contract::Client::new(env, &registry_address);
        let blend_pool_address = registry_client.get_blend_pool_address();
        let smart_account = env.current_contract_address();

        let mut request_type: u32 = 0;

        match action {
            SmartAccExternalAction::Deposit => request_type = 0,
            SmartAccExternalAction::Withdraw => request_type = 1,
            SmartAccExternalAction::Swap => request_type = 10,
        }

        if target_protocol == blend_pool_address {
            let pool_address = registry_client.get_blend_pool_address();
            let blend_pool_client = BlendPoolClient::new(env, &pool_address);

            for (token, amt_wad) in tokens.iter().zip(tokens_amount_wad) {
                log!(&env, "Token symbol passed: {}", token);
                if token == XLM_SYMBOL {
                    let native_xlm_address = registry_client.get_xlm_contract_adddress();
                    let xlm_token = token::Client::new(&env, &native_xlm_address);
                    let amt = Self::scale_from_wad(amt_wad, xlm_token.decimals());
                    let resv: Reserve = blend_pool_client.get_reserve(&native_xlm_address);
                    let pool_config: pool::PoolConfig = blend_pool_client.get_config();
                    blend_pool_client.get_config().oracle;
                    let positions_before = blend_pool_client.get_positions(&smart_account);
                    let b_tokens_before =
                        positions_before.supply.get(resv.config.index).unwrap_or(0);

                    let b_rate = resv.data.b_rate;
                    let request = Request {
                        address: native_xlm_address,
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &resv.asset,
                        &requests,
                    );
                    if request_type == 0 {
                        log!(&env, "Blend Pool Deposit b_rate {}, amount {}", b_rate, amt);
                        let b_tokens_minted =
                            positions.supply.get_unchecked(resv.config.index) - b_tokens_before;
                        return Ok((true, b_tokens_minted));
                    } else if request_type == 1 {
                        let b_tokens_burned =
                            b_tokens_before - positions.supply.get_unchecked(resv.config.index);
                        return Ok((true, -b_tokens_burned));
                    } else {
                        panic!("Unsupported request type for XLM in Blend Pool");
                    }
                } else if token == USDC_SYMBOL {
                    let usdc_contract_address = registry_client.get_usdc_contract_address();
                    let usdc_token = token::Client::new(&env, &usdc_contract_address);
                    let amt = Self::scale_from_wad(amt_wad, usdc_token.decimals());
                    let resv = blend_pool_client.get_reserve(&usdc_contract_address);
                    let b_rate = resv.data.b_rate;
                    let positions_before = blend_pool_client.get_positions(&smart_account);
                    let b_tokens_before =
                        positions_before.supply.get(resv.config.index).unwrap_or(0);

                    let request = Request {
                        address: usdc_contract_address,
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &resv.asset,
                        &requests,
                    );
                    if request_type == 0 {
                        log!(&env, "Blend Pool Deposit b_rate {}, amount {}", b_rate, amt);
                        let b_tokens_minted =
                            positions.supply.get_unchecked(resv.config.index) - b_tokens_before;
                        return Ok((true, b_tokens_minted));
                    } else if request_type == 1 {
                        let b_tokens_burned =
                            b_tokens_before - positions.supply.get_unchecked(resv.config.index);
                        return Ok((true, -b_tokens_burned));
                    } else {
                        panic!("Unsupported request type for XLM in Blend Pool");
                    }
                } else if token == EURC_SYMBOL {
                    let eurc_contract_address = registry_client.get_eurc_contract_address();
                    let eurc_token = token::Client::new(&env, &eurc_contract_address);
                    let amt = Self::scale_from_wad(amt_wad, eurc_token.decimals());
                    let resv = blend_pool_client.get_reserve(&eurc_contract_address);
                    let b_rate = resv.data.b_rate;
                    let positions_before = blend_pool_client.get_positions(&smart_account);
                    let b_tokens_before =
                        positions_before.supply.get(resv.config.index).unwrap_or(0);

                    let request = Request {
                        address: eurc_contract_address,
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &resv.asset,
                        &requests,
                    );
                    if request_type == 0 {
                        log!(&env, "Blend Pool Deposit b_rate {}, amount {}", b_rate, amt);
                        let b_tokens_minted =
                            positions.supply.get_unchecked(resv.config.index) - b_tokens_before;
                        return Ok((true, b_tokens_minted));
                    } else if request_type == 1 {
                        let b_tokens_burned =
                            b_tokens_before - positions.supply.get_unchecked(resv.config.index);
                        return Ok((true, -b_tokens_burned));
                    } else {
                        panic!("Unsupported request type for XLM in Blend Pool");
                    }
                } else {
                    panic!("No external protocol mapped for the given token symbol");
                }
            }
        } else {
            panic!("No external protocol mapped for the given id");
        }

        Ok((false, 0))
    }

    fn set_borrowed_token_list(env: &Env, list: Vec<Symbol>) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::BorrowedTokensList, &list);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::BorrowedTokensList);
    }

    pub fn get_all_collateral_tokens(env: &Env) -> Vec<Symbol> {
        let collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::CollateralTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        collateral_tokens_list
    }

    pub fn add_collateral_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();

        let mut existing_tokens = Self::get_all_collateral_tokens(&env);
        if !existing_tokens.contains(&token_symbol) {
            existing_tokens.push_back(token_symbol);
        }
        Self::set_collateral_tokens_list(env, existing_tokens);
        Ok(())
    }

    fn remove_collateral_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let mut existing_tokens: Vec<Symbol> = Self::get_all_collateral_tokens(&env);
        if existing_tokens.contains(&token_symbol) {
            let index = existing_tokens.first_index_of(&token_symbol).unwrap();
            existing_tokens.remove(index);
        }
        Self::set_collateral_tokens_list(env, existing_tokens);
        Ok(())
    }

    fn set_collateral_tokens_list(env: &Env, list: Vec<Symbol>) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::CollateralTokensList, &list);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::CollateralTokensList);
    }

    pub fn get_collateral_token_balance(env: &Env, token_symbol: Symbol) -> U256 {
        let key_a = SmartAccountDataKey::CollateralBalanceWAD(token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        token_balance
    }

    pub fn set_collateral_token_balance(
        env: &Env,
        token_symbol: Symbol,
        balance_wad: U256,
    ) -> Result<(), SmartAccountError> {
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();
        Self::set_collateral_token_bal_internal(env, token_symbol, balance_wad);
        Ok(())
    }

    fn set_collateral_token_bal_internal(env: &Env, token_symbol: Symbol, balance_wad: U256) {
        let key_a = SmartAccountDataKey::CollateralBalanceWAD(token_symbol.clone());
        env.storage().persistent().set(&key_a, &balance_wad);
        Self::extend_ttl_smart_account(&env, key_a);
    }

    pub fn get_borrowed_token_debt(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<U256, SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let this_account = env.current_contract_address();

        if token_symbol == XLM_SYMBOL {
            let pool_xlm_address = registry_client.get_lendingpool_xlm();
            let xlm_pool_client = lending_protocol_xlm::Client::new(&env, &pool_xlm_address);

            let xlm_debt = xlm_pool_client.get_borrow_balance(&this_account);
            return Ok(xlm_debt);
        } else if token_symbol == USDC_SYMBOL {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            let usdc_pool_client = lending_protocol_usdc::Client::new(&env, &pool_usdc_address);

            let usdc_debt = usdc_pool_client.get_borrow_balance(&this_account);
            return Ok(usdc_debt);
        } else if token_symbol == EURC_SYMBOL {
            let pool_eurc_address = registry_client.get_lendingpool_eurc();
            let eurc_pool_client = lending_protocol_eurc::Client::new(&env, &pool_eurc_address);

            let eurc_debt = eurc_pool_client.get_borrow_balance(&this_account);
            return Ok(eurc_debt);
        } else {
            panic!("User doen't have a borrows in the given token");
        }
    }

    pub fn is_account_active(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&SmartAccountDataKey::IsAccountActive)
            .unwrap_or(false)
    }

    fn scale_for_operation(amount_wad: u128, xlm_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(xlm_decimals)) / WAD_U128) as i128
    }

    fn scale_from_wad(amount_wad: u128, token_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(token_decimals)) / WAD_U128) as i128
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

pub mod tracking_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/tracking_token_contract.wasm"
    );
}
