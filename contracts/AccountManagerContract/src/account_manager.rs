use core::panic;

use soroban_sdk::{
    Address, Bytes, BytesN, Env, Symbol, U256, Vec, contract, contractimpl, log, panic_with_error,
    symbol_short, token,
    xdr::{FromXdr, ToXdr},
};
// use blend_contract_sdk::pool::Client as BlendPoolClient;

use crate::types::{
    AccountCreationEvent, AccountDeletionEvent, AccountManagerError, AccountManagerKey,
    ExternalProtocolCall, TraderBorrowEvent, TraderLiquidateEvent, TraderRepayEvent,
    TraderSettleAccountEvent,
};

use smart_account_contract::SmartAccExternalAction;

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
pub const WAD_U128: u128 = 10000_0000_00000_00000; // 1e18
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const BLUSDC_SYMBOL: Symbol = symbol_short!("BLUSDC");
const AQUSDC_SYMBOL: Symbol = symbol_short!("AQUSDC");
const SOUSDC_SYMBOL: Symbol = symbol_short!("SOUSDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");
const BLEND_XLM: &str = "BLEND_XLM";
const BLEND_USDC: &str = "BLEND_USDC";
const BLEND_EURC: &str = "BLEND_EURC";
const AQUARIUS_XLM_USDC: &str = "AQ_XLM_USDC"; // Aquarius XLM-USDC LP token tracking
const SOROSWAP_XLM_USDC: &str = "SS_XLM_USDC"; // Soroswap XLM-USDC LP token tracking

pub mod smart_account_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/smart_account_contract.wasm"
    );
}

pub mod risk_engine_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/risk_engine_contract.wasm"
    );
}

pub mod registry_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/registry_contract.wasm"
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

pub mod tracking_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/tracking_token_contract.wasm"
    );
}

#[contract]
pub struct AccountManagerContract;

#[contractimpl]
impl AccountManagerContract {
    pub fn __constructor(env: &Env, admin: Address, registry_contract: Address) {
        env.storage()
            .persistent()
            .set(&AccountManagerKey::Admin, &admin);

        env.storage()
            .persistent()
            .set(&AccountManagerKey::RegistryContract, &registry_contract);

        Self::extend_ttl_account_manager(&env, AccountManagerKey::Admin);
        Self::extend_ttl_account_manager(&env, AccountManagerKey::RegistryContract);
    }

    pub fn create_account(
        env: &Env,
        trader_address: Address,
    ) -> Result<Address, AccountManagerError> {
        trader_address.require_auth();

        /* if Self::has_smart_account(&env, &trader_address) {
            panic!("Trader already has a smart account!");
        } */

        let users_key = AccountManagerKey::UsersList;
        let mut users = env
            .storage()
            .persistent()
            .get(&users_key)
            .unwrap_or_else(|| Vec::new(&env));

        if !users.contains(&trader_address) {
            users.push_back(trader_address.clone());
            env.storage().persistent().set(&users_key, &users);
            Self::extend_ttl_account_manager(env, users_key);
        }

        let registry_contract_address: Address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(env, &registry_contract_address);
        let smart_account: Address;

        let mut inactive_accounts = Self::get_inactive_accounts(env, trader_address.clone());
        if inactive_accounts.len() == 0 {
            let smart_account_hash = registry_client.get_smart_account_hash();
            smart_account = Self::create_smart_account(env, &trader_address, smart_account_hash);
            registry_client.add_account(&trader_address, &smart_account);
        } else {
            smart_account = inactive_accounts.pop_back().unwrap();
            Self::set_inactive_accounts(env, trader_address.clone(), inactive_accounts);
            registry_client.update_account(&trader_address, &smart_account);
        }

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);
        smart_account_client.activate_account();

        Ok(smart_account)
    }

    pub fn close_account(
        env: &Env,
        smart_account_address: Address,
    ) -> Result<bool, AccountManagerError> {
        let trader_address = Self::get_trader_address(env, &smart_account_address);
        trader_address.require_auth();

        let smart_account_client =
            smart_account_contract::Client::new(&env, &smart_account_address);

        if smart_account_client.has_debt() {
            panic!("Cannot delete account with debt, please repay debt first");
        }

        smart_account_client.sweep_to(&trader_address);

        let registry_contract_address: Address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(env, &registry_contract_address);

        smart_account_client.deactivate_account();
        registry_client.close_account(&trader_address, &smart_account_address);

        let mut inactive_accounts = Self::get_inactive_accounts(env, trader_address.clone());
        inactive_accounts.push_back(smart_account_address.clone());
        Self::set_inactive_accounts(env, trader_address.clone(), inactive_accounts);

        let kex_yy = AccountManagerKey::AccountClosedTime(smart_account_address.clone());
        // Set account deletion time
        env.storage()
            .persistent()
            .set(&kex_yy, &env.ledger().timestamp());
        Self::extend_ttl_account_manager(env, kex_yy);

        env.events().publish(
            (Symbol::new(&env, "Smart_Account_Closed"), &trader_address),
            AccountDeletionEvent {
                smart_account: smart_account_address,
                deletion_time: env.ledger().timestamp(),
            },
        );

        Ok(true)
    }

    pub fn deposit_collateral_tokens(
        env: Env,
        smart_account: Address,
        token_symbol: Symbol,
        token_amount_wad: U256,
    ) -> Result<(), AccountManagerError> {
        let trader_address = Self::get_trader_address(&env, &smart_account);
        trader_address.require_auth();

        if token_amount_wad.eq(&U256::from_u128(&env, 0)) {
            panic!("Cannot deposit a zero amount");
        }

        if !Self::get_iscollateral_allowed(&env, token_symbol.clone()) {
            panic!("This token is not allowed as collateral");
        }

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);
        let collateral_tokens = smart_account_client.get_all_collateral_tokens();

        if U256::from_u32(&env, collateral_tokens.len()) >= Self::get_max_asset_cap(&env) {
            panic!("Max asset cap crossed!");
        }

        if !collateral_tokens.contains(token_symbol.clone()) {
            smart_account_client.add_collateral_token(&token_symbol);
        }

        let amount_wad_u128 = Self::convert_u256_to_u128(&env, &token_amount_wad);
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        // Transfer tokens based on type
        if token_symbol == XLM_SYMBOL {
            let token_client = token::Client::new(&env, &registry_client.get_xlm_contract_adddress());
            let amount_scaled = Self::scale_for_operation(amount_wad_u128, token_client.decimals());
            token_client.transfer(&trader_address, &smart_account, &amount_scaled);
        } else if token_symbol == USDC_SYMBOL || token_symbol == BLUSDC_SYMBOL {
            let token_client = token::Client::new(&env, &registry_client.get_usdc_contract_address());
            let amount_scaled = Self::scale_for_operation(amount_wad_u128, token_client.decimals());
            token_client.transfer(&trader_address, &smart_account, &amount_scaled);
        } else if token_symbol == AQUSDC_SYMBOL {
            let token_client = token::Client::new(&env, &registry_client.get_aquarius_usdc_addr());
            let amount_scaled = Self::scale_for_operation(amount_wad_u128, token_client.decimals());
            token_client.transfer(&trader_address, &smart_account, &amount_scaled);
        } else if token_symbol == SOUSDC_SYMBOL {
            let token_client = token::Client::new(&env, &registry_client.get_soroswap_usdc_addr());
            let amount_scaled = Self::scale_for_operation(amount_wad_u128, token_client.decimals());
            token_client.transfer(&trader_address, &smart_account, &amount_scaled);
        } else if token_symbol == EURC_SYMBOL {
            let token_client = token::Client::new(&env, &registry_client.get_eurc_contract_address());
            let amount_scaled = Self::scale_for_operation(amount_wad_u128, token_client.decimals());
            token_client.transfer(&trader_address, &smart_account, &amount_scaled);
        } else {
            panic!("Collateral not allowed for this token symbol");
        }

        // Update balance
        let existing_bal = smart_account_client.get_collateral_token_balance(&token_symbol);
        smart_account_client.set_collateral_token_balance(
            &token_symbol, 
            &existing_bal.add(&token_amount_wad)
        );

        Ok(())
    }

    pub fn withdraw_collateral_balance(
        env: Env,
        smart_account: Address,
        token_symbol: Symbol,
        token_amount_wad: U256,
    ) -> Result<(), AccountManagerError> {
        let trader_address = Self::get_trader_address(&env, &smart_account);
        trader_address.require_auth();

        if token_amount_wad.eq(&U256::from_u128(&env, 0)) {
            panic!("Cannot withdraw a zero amount");
        }

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);

        let collateral_tokens_list = smart_account_client.get_all_collateral_tokens();
        if !collateral_tokens_list.contains(token_symbol.clone()) {
            panic!("User doesn't have collateral in this token");
        }

        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let risk_engine_address = registry_client.get_risk_engine_address();
        let risk_engine_client = risk_engine_contract::Client::new(&env, &risk_engine_address);

        if !risk_engine_client.is_withdraw_allowed(&token_symbol, &token_amount_wad, &smart_account)
        {
            panic!("Account is unhealthy! withdraw is not allowed");
        }

        let amount_u128: u128 = Self::convert_u256_to_u128(&env, &token_amount_wad);

        smart_account_client.remove_collateral_token_balance(
            &trader_address,
            &token_symbol,
            &amount_u128,
        );

        Ok(())
    }

    pub fn borrow(
        env: &Env,
        smart_account: Address,
        borrow_amount_wad: U256,
        token_symbol: Symbol,
    ) -> Result<(), AccountManagerError> {
        let trader_address = Self::get_trader_address(&env, &smart_account);
        trader_address.require_auth();

        if borrow_amount_wad.eq(&U256::from_u128(&env, 0)) {
            panic!("Cannot borrow a zero amount");
        }

        let registry_address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let risk_engine_address = registry_client.get_risk_engine_address();
        let risk_engine_client = risk_engine_contract::Client::new(&env, &risk_engine_address);

        // Check borrow allowance
        if !risk_engine_client.is_borrow_allowed(
            &token_symbol,
            &borrow_amount_wad,
            &smart_account,
        ) {
            panic!("Borrowing is not allowed for this user");
        }

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);

        // Execute lending based on token type
        if token_symbol == XLM_SYMBOL {
            let pool_xlm = registry_client.get_lendingpool_xlm();
            lending_protocol_xlm::Client::new(&env, &pool_xlm)
                .lend_to(&smart_account, &borrow_amount_wad);
            smart_account_client.add_borrowed_token(&XLM_SYMBOL);
            smart_account_client.set_has_debt(&true);
        } else if token_symbol == USDC_SYMBOL || token_symbol == BLUSDC_SYMBOL {
            let pool_usdc = registry_client.get_lendingpool_usdc();
            lending_protocol_usdc::Client::new(&env, &pool_usdc)
                .lend_to(&smart_account, &borrow_amount_wad);
            smart_account_client.add_borrowed_token(&BLUSDC_SYMBOL);
            smart_account_client.set_has_debt(&true);
        } else if token_symbol == AQUSDC_SYMBOL {
            let pool_aquarius_usdc = registry_client.get_lendingpool_aquarius_usdc();
            lending_protocol_usdc::Client::new(&env, &pool_aquarius_usdc)
                .lend_to(&smart_account, &borrow_amount_wad);
            smart_account_client.add_borrowed_token(&AQUSDC_SYMBOL);
            smart_account_client.set_has_debt(&true);
        } else if token_symbol == SOUSDC_SYMBOL {
            let pool_soroswap_usdc = registry_client.get_lendingpool_soroswap_usdc();
            lending_protocol_usdc::Client::new(&env, &pool_soroswap_usdc)
                .lend_to(&smart_account, &borrow_amount_wad);
            smart_account_client.add_borrowed_token(&SOUSDC_SYMBOL);
            smart_account_client.set_has_debt(&true);
        } else if token_symbol == EURC_SYMBOL {
            let pool_eurc = registry_client.get_lendingpool_eurc();
            lending_protocol_eurc::Client::new(&env, &pool_eurc)
                .lend_to(&smart_account, &borrow_amount_wad);
            smart_account_client.add_borrowed_token(&EURC_SYMBOL);
            smart_account_client.set_has_debt(&true);
        } else {
            panic!("No lending pool available for given token_symbol");
        }

        // Publish simplified event
        env.events().publish(
            (Symbol::new(&env, "Trader_Borrow"), smart_account.clone()),
            token_symbol,
        );

        Ok(())
    }

    pub fn repay(
        env: Env,
        repay_amount_wad: U256,
        token_symbol: Symbol,
        smart_account: Address,
    ) -> Result<(), AccountManagerError> {
        let trader_address = Self::get_trader_address(&env, &smart_account);
        trader_address.require_auth();

        if repay_amount_wad.eq(&U256::from_u128(&env, 0)) {
            panic!("Cannot repay a zero amount");
        }

        let registry_address: Address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);

        let borrowed_tokens = smart_account_client.get_all_borrowed_tokens();

        if !borrowed_tokens.contains(token_symbol.clone()) {
            panic!("User doen't have debt in the token symbol passed");
        }

        let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &repay_amount_wad);

        let _debt = smart_account_client.get_borrowed_token_debt(&token_symbol.clone());
        // !! Should we check if the repay amount is greater than the debt amount?

        if token_symbol == XLM_SYMBOL {
            let pool_xlm_contract = registry_client.get_lendingpool_xlm();
            let xlm_client = lending_protocol_xlm::Client::new(&env, &pool_xlm_contract);
            let bool = xlm_client.collect_from(&repay_amount_wad, &smart_account);
            smart_account_client.remove_borrowed_token_balance(&XLM_SYMBOL, &amount_wad_u128);
            if bool {
                smart_account_client.remove_borrowed_token(&XLM_SYMBOL);
            }
        } else if token_symbol == USDC_SYMBOL || token_symbol == BLUSDC_SYMBOL {
            let pool_usdc_contract = registry_client.get_lendingpool_usdc();
            let usdc_client = lending_protocol_usdc::Client::new(&env, &pool_usdc_contract);
            let bool = usdc_client.collect_from(&repay_amount_wad, &smart_account);
            smart_account_client.remove_borrowed_token_balance(&BLUSDC_SYMBOL, &amount_wad_u128);
            if bool {
                smart_account_client.remove_borrowed_token(&BLUSDC_SYMBOL);
            }
        } else if token_symbol == AQUSDC_SYMBOL {
            let pool_aquarius_usdc_contract = registry_client.get_lendingpool_aquarius_usdc();
            let usdc_client = lending_protocol_usdc::Client::new(&env, &pool_aquarius_usdc_contract);
            let bool = usdc_client.collect_from(&repay_amount_wad, &smart_account);
            smart_account_client.remove_borrowed_token_balance(&AQUSDC_SYMBOL, &amount_wad_u128);
            if bool {
                smart_account_client.remove_borrowed_token(&AQUSDC_SYMBOL);
            }
        } else if token_symbol == SOUSDC_SYMBOL {
            let pool_soroswap_usdc_contract = registry_client.get_lendingpool_soroswap_usdc();
            let usdc_client = lending_protocol_usdc::Client::new(&env, &pool_soroswap_usdc_contract);
            let bool = usdc_client.collect_from(&repay_amount_wad, &smart_account);
            smart_account_client.remove_borrowed_token_balance(&SOUSDC_SYMBOL, &amount_wad_u128);
            if bool {
                smart_account_client.remove_borrowed_token(&SOUSDC_SYMBOL);
            }
        } else if token_symbol == EURC_SYMBOL {
            let pool_eurc_contract = registry_client.get_lendingpool_eurc();
            let eurc_client = lending_protocol_eurc::Client::new(&env, &pool_eurc_contract);
            let bool = eurc_client.collect_from(&repay_amount_wad, &smart_account);
            smart_account_client.remove_borrowed_token_balance(&EURC_SYMBOL, &amount_wad_u128);
            if bool {
                smart_account_client.remove_borrowed_token(&EURC_SYMBOL);
            }
        } else {
            panic!("No lending pool available for given token_symbol");
        }

        env.events().publish(
            (
                Symbol::new(&env, "Trader_Repay_Event"),
                smart_account.clone(),
            ),
            TraderRepayEvent {
                smart_account: smart_account,
                token_amount: repay_amount_wad,
                timestamp: env.ledger().timestamp(),
                token_symbol,
            },
        );
        Ok(())
    }

    pub fn liquidate(env: Env, smart_account: Address) -> Result<(), AccountManagerError> {
        let trader_address = Self::get_trader_address(&env, &smart_account);
        trader_address.require_auth();

        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);

        let risk_engine_address = registry_client.get_risk_engine_address();
        let risk_engine_client = risk_engine_contract::Client::new(&env, &risk_engine_address);
        if risk_engine_client.is_account_healthy(
            &risk_engine_client.get_current_total_balance(&smart_account),
            &risk_engine_client.get_current_total_borrows(&smart_account),
        ) {
            panic!("Cannot liquidate when account is healthy!!");
        }

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);
        let all_borrowed_tokens = smart_account_client.get_all_borrowed_tokens();

        for tokenx in all_borrowed_tokens.iter() {
            if tokenx == XLM_SYMBOL {
                let pool_xlm_contract = registry_client.get_lendingpool_xlm();
                let xlm_client: lending_protocol_xlm::Client<'_> =
                    lending_protocol_xlm::Client::new(&env, &pool_xlm_contract);
                let liquidate_amount = xlm_client.get_borrow_balance(&smart_account);
                let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &liquidate_amount);
                let bool = xlm_client.collect_from(&liquidate_amount, &smart_account);
                smart_account_client.remove_borrowed_token_balance(&XLM_SYMBOL, &amount_wad_u128);
                if bool {
                    smart_account_client.remove_borrowed_token(&XLM_SYMBOL);
                }
            } else if tokenx == USDC_SYMBOL || tokenx == BLUSDC_SYMBOL {
                let pool_usdc_contract = registry_client.get_lendingpool_usdc();
                let usdc_client: lending_protocol_usdc::Client<'_> =
                    lending_protocol_usdc::Client::new(&env, &pool_usdc_contract);
                let liquidate_amount = usdc_client.get_borrow_balance(&smart_account);

                let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &liquidate_amount);

                let bool = usdc_client.collect_from(&liquidate_amount, &smart_account);
                smart_account_client.remove_borrowed_token_balance(&BLUSDC_SYMBOL, &amount_wad_u128);
                if bool {
                    smart_account_client.remove_borrowed_token(&BLUSDC_SYMBOL);
                }
            } else if tokenx == AQUSDC_SYMBOL {
                let pool_aquarius_usdc_contract = registry_client.get_lendingpool_aquarius_usdc();
                let usdc_client: lending_protocol_usdc::Client<'_> =
                    lending_protocol_usdc::Client::new(&env, &pool_aquarius_usdc_contract);
                let liquidate_amount = usdc_client.get_borrow_balance(&smart_account);

                let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &liquidate_amount);

                let bool = usdc_client.collect_from(&liquidate_amount, &smart_account);
                smart_account_client.remove_borrowed_token_balance(&AQUSDC_SYMBOL, &amount_wad_u128);
                if bool {
                    smart_account_client.remove_borrowed_token(&AQUSDC_SYMBOL);
                }
            } else if tokenx == SOUSDC_SYMBOL {
                let pool_soroswap_usdc_contract = registry_client.get_lendingpool_soroswap_usdc();
                let usdc_client: lending_protocol_usdc::Client<'_> =
                    lending_protocol_usdc::Client::new(&env, &pool_soroswap_usdc_contract);
                let liquidate_amount = usdc_client.get_borrow_balance(&smart_account);

                let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &liquidate_amount);

                let bool = usdc_client.collect_from(&liquidate_amount, &smart_account);
                smart_account_client.remove_borrowed_token_balance(&SOUSDC_SYMBOL, &amount_wad_u128);
                if bool {
                    smart_account_client.remove_borrowed_token(&SOUSDC_SYMBOL);
                }
            } else if tokenx == EURC_SYMBOL {
                let pool_eurc_contract = registry_client.get_lendingpool_eurc();
                let eurc_client: lending_protocol_eurc::Client<'_> =
                    lending_protocol_eurc::Client::new(&env, &pool_eurc_contract);
                let liquidate_amount = eurc_client.get_borrow_balance(&smart_account);

                let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &liquidate_amount);

                let bool = eurc_client.collect_from(&liquidate_amount, &smart_account);
                smart_account_client.remove_borrowed_token_balance(&EURC_SYMBOL, &amount_wad_u128);
                if bool {
                    smart_account_client.remove_borrowed_token(&EURC_SYMBOL);
                }
            } else {
                panic!("This token pool doesn't exist")
            }
        }

        smart_account_client.sweep_to(&trader_address);

        env.events().publish(
            (
                Symbol::new(&env, "Trader_Liquidate_Event"),
                smart_account.clone(),
            ),
            TraderLiquidateEvent {
                smart_account: smart_account,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn settle_account(env: Env, smart_account: Address) -> Result<bool, AccountManagerError> {
        let trader_address = Self::get_trader_address(&env, &smart_account);
        trader_address.require_auth();

        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &smart_account);
        let borrowed_tokens = smart_account_contract_client.get_all_borrowed_tokens();
        for tokenx in borrowed_tokens.iter() {
            let token_debt = smart_account_contract_client.get_borrowed_token_debt(&tokenx.clone());
            Self::repay(env.clone(), token_debt, tokenx, smart_account.clone())
                .expect("Failed to repay while settling the account");
        }
        env.events().publish(
            (
                Symbol::new(&env, "Trader_SettleAccount_Event"),
                smart_account.clone(),
            ),
            TraderSettleAccountEvent {
                smart_account,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(true)
    }

    fn extend_ttl_account_manager(env: &Env, key: AccountManagerKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    pub fn set_max_asset_cap(env: &Env, cap: U256) {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::Admin)
            .unwrap_or_else(|| panic!("Failed to fetch admin address n1"));
        admin.require_auth();

        let key = AccountManagerKey::AssetCap;
        env.storage().persistent().set(&key, &cap);
        Self::extend_ttl_account_manager(env, key);
    }

    pub fn get_max_asset_cap(env: &Env) -> U256 {
        let key = AccountManagerKey::AssetCap;
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("Asset cap not set"))
    }

    pub fn get_iscollateral_allowed(env: &Env, token_symbol: Symbol) -> bool {
        let key = AccountManagerKey::IsCollateralAllowed(token_symbol);
        env.storage().persistent().get(&key).unwrap_or(false)
    }

    pub fn set_iscollateral_allowed(env: &Env, token_symbol: Symbol) {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::Admin)
            .unwrap_or_else(|| panic!("Admin key not set!"));
        admin.require_auth();

        let key = AccountManagerKey::IsCollateralAllowed(token_symbol);
        env.storage().persistent().set(&key, &true);
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&AccountManagerKey::RegistryContract)
            .unwrap_or_else(|| panic!("Failed to fetch registry contract address n1"))
    }

    fn get_trader_address(env: &Env, smart_account: &Address) -> Address {
        env.storage()
            .persistent()
            .get(&AccountManagerKey::TraderAddress(smart_account.clone()))
            .expect("Failed to fetch Traders address")
    }

    pub fn generate_salt(
        env: &Env,
        trader_address: Address,
        account_manager: Address,
        smart_account_num: u32,
    ) -> BytesN<32> {
        // Convert addresses to XDR for consistent serialization
        // Make sure empty addresses are not sent
        assert!(trader_address.to_string().len() != 0);
        assert!(account_manager.to_string().len() != 0);

        let trader_xdr = trader_address.to_xdr(env);
        let manager_xdr = account_manager.to_xdr(env);
        let num_xdr = smart_account_num.to_le_bytes();

        // Create a combined buffer to hash both addresses together
        let mut combined = Bytes::new(env);

        // Append trader address bytes
        for i in 0..trader_xdr.len() {
            combined.push_back(trader_xdr.get(i).unwrap());
        }

        // Append account manager bytes
        for i in 0..manager_xdr.len() {
            combined.push_back(manager_xdr.get(i).unwrap());
        }

        for i in 0..num_xdr.len() {
            combined.push_back(*num_xdr.get(i).unwrap());
        }

        // Use Soroban's built-in SHA256 hash function
        // This ensures a unique 32-byte output for any unique input combination
        env.crypto().sha256(&combined).into()
    }

    pub fn get_inactive_accounts(env: &Env, trader_address: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&AccountManagerKey::InactiveAccountOf(trader_address))
            .unwrap_or(Vec::new(env))
    }

    fn set_inactive_accounts(env: &Env, trader_address: Address, inactive_accounts: Vec<Address>) {
        let keyx = AccountManagerKey::InactiveAccountOf(trader_address);
        env.storage().persistent().set(&keyx, &inactive_accounts);
        Self::extend_ttl_account_manager(env, keyx);
    }

    fn convert_u256_to_u128(env: &Env, x: &U256) -> u128 {
        x.to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, AccountManagerError::IntegerConversionError))
    }

    fn create_smart_account(
        env: &Env,
        trader_address: &Address,
        smart_account_hash: BytesN<32>,
    ) -> Address {
        let mut trader_smart_accounts = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::SmartAccounts(trader_address.clone()))
            .unwrap_or(Vec::new(env));

        let salt = Self::generate_salt(
            &env,
            trader_address.clone(),
            env.current_contract_address(),
            trader_smart_accounts.len(),
        );

        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(env.current_contract_address().to_val());
        constructor_args.push_back(Self::get_registry_address(env).to_val());
        constructor_args.push_back(trader_address.to_val());

        let smart_account = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(smart_account_hash, constructor_args);

        trader_smart_accounts.push_back(smart_account.clone());

        env.storage().persistent().set(
            &AccountManagerKey::SmartAccounts(trader_address.clone()),
            &trader_smart_accounts,
        );

        env.storage().persistent().set(
            &AccountManagerKey::TraderAddress(smart_account.clone()),
            &trader_address,
        );

        env.events().publish(
            (Symbol::new(&env, "Smart_account_creation"), trader_address),
            AccountCreationEvent {
                smart_account: smart_account.clone(),
                creation_time: env.ledger().timestamp(),
            },
        );
        smart_account
    }

    fn scale_for_operation(amount_wad: u128, xlm_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(xlm_decimals)) / WAD_U128) as i128
    }
    /// To be implemented
    pub fn approve() {}

    pub fn execute(env_x: &Env, smart_account: Address, extern_proto_call_bytes: Bytes) {
        let trader_address = Self::get_trader_address(&env_x, &smart_account);
        trader_address.require_auth();

        let call: ExternalProtocolCall =
            ExternalProtocolCall::from_xdr(env_x, &extern_proto_call_bytes)
                .expect("deserialize failed");

        let registry_address: Address = Self::get_registry_address(&env_x);
        let registry_client = registry_contract::Client::new(&env_x, &registry_address);

        let mut tokens_amount_wad = Vec::new(env_x);

        call.amount_out
            .iter()
            .for_each(|x| tokens_amount_wad.push_back(x.to_u128().unwrap()));

        let smart_acc_client = smart_account_contract::Client::new(env_x, &smart_account);

        let (_ok, token_delta) = smart_acc_client.execute(
            &call.protocol_address,
            &call.type_action,
            &trader_address,
            &call.tokens_out,
            &tokens_amount_wad,
        );

        // Check if this is a Blend protocol operation
        if registry_client.has_blend_pool_address() {
            let blend_pool_address = registry_client.get_blend_pool_address();
            if call.protocol_address == blend_pool_address {
                if call.tokens_out.len() != 1 {
                    panic!("Blend operations support exactly one token per call");
                }

                if token_delta != 0 && registry_client.has_tracking_token_contract_addr() {
                    let token_symbol = call.tokens_out.get(0).unwrap();
                    let tracking_symbol = Self::tracking_symbol_for_blend(env_x, &token_symbol);

                    let tracking_token_address = registry_client.get_tracking_token_contract_addr();
                    let tracking_client =
                        tracking_token_contract::Client::new(env_x, &tracking_token_address);

                    if token_delta > 0 {
                        tracking_client.mint(&tracking_symbol, &smart_account, &token_delta);
                    } else {
                        tracking_client.burn(&tracking_symbol, &smart_account, &(-token_delta));
                    }

                    let tracking_balance = tracking_client.balance(&smart_account, &tracking_symbol);
                    if tracking_balance > 0 {
                        smart_acc_client.add_collateral_token(&tracking_symbol);
                    }
                }
                return;
            }
        }

        // Handle Aquarius protocol operations
        if registry_client.has_aquarius_router_address() {
            let aquarius_router_address = registry_client.get_aquarius_router_address();
            if call.protocol_address == aquarius_router_address && registry_client.has_tracking_token_contract_addr() {
                let tracking_token_address = registry_client.get_tracking_token_contract_addr();
                let tracking_client =
                    tracking_token_contract::Client::new(env_x, &tracking_token_address);

                match call.type_action {
                    SmartAccExternalAction::AddLiquidity => {
                        // Mint LP tracking tokens for liquidity provision
                        if token_delta > 0 {
                            let tracking_symbol = Self::tracking_symbol_for_aquarius_lp(
                                env_x,
                                &call.tokens_out.get(0).unwrap(),
                                &call.tokens_out.get(1).unwrap(),
                            );

                            tracking_client.mint(&tracking_symbol, &smart_account, &token_delta);

                            let tracking_balance =
                                tracking_client.balance(&smart_account, &tracking_symbol);
                            if tracking_balance > 0 {
                                smart_acc_client.add_collateral_token(&tracking_symbol);
                            }
                        }
                    }
                    SmartAccExternalAction::RemoveLiquidity => {
                        // Burn LP tracking tokens for liquidity removal
                        if token_delta < 0 {
                            let tracking_symbol = Self::tracking_symbol_for_aquarius_lp(
                                env_x,
                                &call.tokens_out.get(0).unwrap(),
                                &call.tokens_out.get(1).unwrap(),
                            );

                            tracking_client.burn(&tracking_symbol, &smart_account, &(-token_delta));
                        }
                    }
                    SmartAccExternalAction::Swap => {
                        // Swaps don't affect LP token tracking
                    }
                    _ => {}
                }
                return;
            }
        }

        // Handle Soroswap protocol operations
        if registry_client.has_soroswap_router_address() {
            let soroswap_router_address = registry_client.get_soroswap_router_address();
            if call.protocol_address == soroswap_router_address
                && registry_client.has_tracking_token_contract_addr()
            {
                let tracking_token_address = registry_client.get_tracking_token_contract_addr();
                let tracking_client =
                    tracking_token_contract::Client::new(env_x, &tracking_token_address);

                match call.type_action {
                    SmartAccExternalAction::AddLiquidity => {
                        if token_delta > 0 {
                            let tracking_symbol = Self::tracking_symbol_for_soroswap_lp(
                                env_x,
                                &call.tokens_out.get(0).unwrap(),
                                &call.tokens_out.get(1).unwrap(),
                            );
                            tracking_client.mint(&tracking_symbol, &smart_account, &token_delta);
                            let tracking_balance =
                                tracking_client.balance(&smart_account, &tracking_symbol);
                            if tracking_balance > 0 {
                                smart_acc_client.add_collateral_token(&tracking_symbol);
                            }
                        }
                    }
                    SmartAccExternalAction::RemoveLiquidity => {
                        if token_delta < 0 {
                            let tracking_symbol = Self::tracking_symbol_for_soroswap_lp(
                                env_x,
                                &call.tokens_out.get(0).unwrap(),
                                &call.tokens_out.get(1).unwrap(),
                            );
                            tracking_client.burn(
                                &tracking_symbol,
                                &smart_account,
                                &(-token_delta),
                            );
                        }
                    }
                    SmartAccExternalAction::Swap => {
                        // Swaps don't affect LP token tracking
                    }
                    _ => {}
                }
                return;
            }
        }
    }

    pub fn can_call(
        env: &Env,
        protocol_addr: Address,
        smart_account: Address,
        call_data: Bytes,
    ) -> (bool, Address, Address) {
        (
            true,
            Address::from_str(env, "strkey"),
            Address::from_str(env, "strkey"),
        )
    }

    pub fn sweepto() {}

    fn tracking_symbol_for_blend(env: &Env, token_symbol: &Symbol) -> Symbol {
        if *token_symbol == XLM_SYMBOL {
            Symbol::new(env, BLEND_XLM)
        } else if *token_symbol == USDC_SYMBOL || *token_symbol == BLUSDC_SYMBOL {
            Symbol::new(env, BLEND_USDC)
        } else if *token_symbol == EURC_SYMBOL {
            Symbol::new(env, BLEND_EURC)
        } else {
            panic!("Tracking token not configured for given token symbol");
        }
    }

    fn tracking_symbol_for_aquarius_lp(
        env: &Env,
        token0: &Symbol,
        token1: &Symbol,
    ) -> Symbol {
        if (*token0 == XLM_SYMBOL && *token1 == USDC_SYMBOL)
            || (*token0 == USDC_SYMBOL && *token1 == XLM_SYMBOL)
        {
            Symbol::new(env, AQUARIUS_XLM_USDC)
        } else {
            panic!("Aquarius LP tracking not configured for this token pair");
        }
    }

    fn tracking_symbol_for_soroswap_lp(
        env: &Env,
        token0: &Symbol,
        token1: &Symbol,
    ) -> Symbol {
        if (*token0 == XLM_SYMBOL && *token1 == USDC_SYMBOL)
            || (*token0 == USDC_SYMBOL && *token1 == XLM_SYMBOL)
        {
            Symbol::new(env, SOROSWAP_XLM_USDC)
        } else {
            panic!("Soroswap LP tracking not configured for this token pair");
        }
    }
}
