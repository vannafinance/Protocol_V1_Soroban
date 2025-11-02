use core::panic;

use crate::errors::{InterestRateError, LendingError};
use crate::events::{
    LendingDepositEvent, LendingTokenBurnEvent, LendingTokenMintEvent, LendingWithdrawEvent,
};
use crate::types::{ContractDetails, DataKey, PoolDataKey, TokenDataKey};
use soroban_sdk::{
    Address, Env, String, Symbol, U256, Vec, contract, contractimpl, log, panic_with_error, token,
};

pub mod rate_model_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/rate_model_contract.wasm"
    );
}

pub mod registry_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/registry_contract.wasm"
    );
}

pub mod smart_account_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/smart_account_contract.wasm"
    );
}

pub mod veurc_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/veurc_token_contract.wasm"
    );
}

#[contract]
pub struct LiquidityPoolEURC;

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;

pub const WAD_U128: u128 = 10000_0000_00000_00000; // 1e18

#[contractimpl]
impl LiquidityPoolEURC {
    pub fn __constructor(
        env: Env,
        admin: Address,
        native_token_address: Address,
        registry_contract: Address,
        account_manager: Address,
        rate_model: Address,
        token_issuer: Address,
    ) {
        let key = DataKey::Admin;

        env.storage().persistent().set(&DataKey::Admin, &admin);
        Self::extend_ttl_datakey(&env, key);

        env.events().publish(("constructor", "admin_set"), &admin);

        env.storage()
            .persistent()
            .set(&ContractDetails::RegistryContract, &registry_contract);
        Self::extend_ttl_contractdatakey(&env, ContractDetails::RegistryContract);
        env.events()
            .publish(("constructor", "registry_set"), &registry_contract);

        env.storage()
            .persistent()
            .set(&ContractDetails::AccountManager, &account_manager);
        Self::extend_ttl_contractdatakey(&env, ContractDetails::AccountManager);
        env.events()
            .publish(("constructor", "account_manager_set"), &account_manager);

        env.storage()
            .persistent()
            .set(&ContractDetails::RateModel, &rate_model);
        Self::extend_ttl_contractdatakey(&env, ContractDetails::RateModel);
        env.events()
            .publish(("constructor", "rate_model_set"), &rate_model);

        env.storage().persistent().set(
            &TokenDataKey::NativeEURCClientAddress,
            &native_token_address,
        );
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::NativeEURCClientAddress);
        env.events()
            .publish(("constructor", "native_eurc_set"), &native_token_address);

        env.storage()
            .persistent()
            .set(&TokenDataKey::TokenIssuerAddress, &token_issuer);
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::TokenIssuerAddress);
        env.events()
            .publish(("constructor", "token_issuer_set"), &token_issuer);
    }

    pub fn reset_admin(env: Env, admin: Address) -> Result<String, LendingError> {
        // Resetting the admin, can be done only by exisiting admin
        let admin_existing = Self::get_admin(&env).unwrap();
        admin_existing.require_auth();

        let key = DataKey::Admin;

        env.storage().persistent().set(&DataKey::Admin, &admin);
        Self::extend_ttl_datakey(&env, key);
        Ok(String::from_str(&env, "Adminkey set successfully reset"))
    }

    pub fn get_admin(env: &Env) -> Result<Address, LendingError> {
        let key = DataKey::Admin;
        let admin_address: Address = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("Admin key has not been set"));
        Ok(admin_address)
    }

    pub fn initialize_pool_eurc(
        env: Env,
        veurc_token_contract_address: Address,
    ) -> Result<String, LendingError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("Admin not set");

        admin.require_auth();

        let veurc_symbol = Symbol::new(&env, "VEURC");
        env.storage().persistent().set(
            &TokenDataKey::VTokenContractAddress(veurc_symbol.clone()),
            &veurc_token_contract_address,
        );
        Self::extend_ttl_tokendatakey(
            &env,
            TokenDataKey::VTokenContractAddress(veurc_symbol.clone()),
        );

        env.storage()
            .persistent()
            .set(&PoolDataKey::Initialised, &true); // Store the EURC this contract handles

        env.events()
            .publish(("initialize_pool_eurc", "eurc_pool_initialized"), true);
        Ok(String::from_str(&env, "EURC pool initialised"))
    }

    pub fn deposit_eurc(env: Env, lender: Address, amount_wad: U256) {
        lender.require_auth();
        if amount_wad <= U256::from_u128(&env, 0) {
            panic!("Deposit amount must be positive");
        }
        // Check if pool is initialised
        Self::is_eurc_pool_initialised(&env);
        Self::before_deposit(&env);

        let amount_wad_u128: u128 = amount_wad
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        // Getting the amount of tokens to be minted for Asset deposited
        let vtokens_to_be_minted_wad = Self::convert_eurc_to_vtoken(&env, amount_wad.clone());

        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);

        let user_balance = eurc_token.balance(&lender);
        let user_balance_wad = Self::scale_for_balance(user_balance, eurc_token.decimals()) as u128;

        if user_balance_wad < amount_wad_u128 {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }

        log!(&env, "reached zzzssss");
        let amount_scaled = Self::scale_for_operation(amount_wad_u128, eurc_token.decimals());
        // Transfer EURC from user to this contract
        eurc_token.transfer(
            &lender,                         // from
            &env.current_contract_address(), // to
            &amount_scaled,
        );

        // Update lender list
        Self::add_lender_to_list_eurc(&env, &lender);

        // Now Mint the VEURC tokens that were created for the lender
        Self::mint_veurc_tokens(&env, lender.clone(), vtokens_to_be_minted_wad);

        env.events().publish(
            (Symbol::new(&env, "deposit_event"), lender.clone()),
            LendingDepositEvent {
                lender: lender.clone(),
                amount: amount_wad,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "EURC"),
            },
        );
    }

    pub fn redeem_veurc(env: &Env, lender: Address, tokens_to_redeem_wad: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_eurc_pool_initialised(&env);
        Self::before_withdraw(env);

        let veurc_token_contract_address: Address = Self::get_vtoken_contract_address(env);

        let eurc_token_client =
            veurc_token_contract::Client::new(&env, &veurc_token_contract_address);
        let veurc_balance_wad = Self::scale_for_balance(
            eurc_token_client.balance(&lender),
            eurc_token_client.decimals(),
        );
        let veurc_balance_wad_u256 = U256::from_u128(&env, veurc_balance_wad as u128);

        // Check if lender has enough token balance to redeem
        if tokens_to_redeem_wad > veurc_balance_wad_u256 {
            panic!("Insufficient Token Balance to redeem");
        }

        let eurc_value_to_transfer_wad =
            Self::convert_vtoken_to_eurc(env, tokens_to_redeem_wad.clone());
        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);
        let current_pool_balance_wad = Self::get_total_liquidity_in_pool(&env);

        log!(
            &env,
            "pool bal, eurc_transfer {:?},{:?}",
            current_pool_balance_wad,
            eurc_value_to_transfer_wad
        );
        // Check if there is enough balance in the pool to redeem
        if current_pool_balance_wad < eurc_value_to_transfer_wad {
            panic_with_error!(&env, LendingError::InsufficientPoolBalance);
        }

        let amount_wad_u128: u128 = eurc_value_to_transfer_wad
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let amount_scaled = Self::scale_for_operation(amount_wad_u128, eurc_token.decimals());

        eurc_token.transfer(
            &env.current_contract_address(), // from
            &lender,                         // to
            &amount_scaled,
        );

        Self::burn_veurc_tokens(&env, lender.clone(), tokens_to_redeem_wad.clone());

        // emit event after withdraw
        env.events().publish(
            (Symbol::new(&env, "withdraw_event"), lender.clone()),
            LendingWithdrawEvent {
                lender: lender,
                vtoken_amount: tokens_to_redeem_wad,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "EURC"),
            },
        );
    }

    pub fn lend_to(
        env: &Env,
        smart_account: Address,
        amount_wad: U256,
    ) -> Result<bool, LendingError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        log!(&env, "reached before update state!");

        Self::update_state(env);
        let borrow_shares_wad = Self::convert_asset_borrow_shares(env, amount_wad.clone());
        let mut is_first_borrow: bool = false;

        let key_c = PoolDataKey::BorrowsWAD;
        let user_borrow_shares_wad: U256 =
            Self::get_user_borrow_shares(&env, smart_account.clone());
        let total_borrow_shares_wad: U256 = Self::get_total_borrow_shares(&env);
        let borrows_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| U256::from_u128(&env, 0));

        if user_borrow_shares_wad == U256::from_u32(&env, 0) {
            is_first_borrow = true;
        }

        let res1 = user_borrow_shares_wad.add(&borrow_shares_wad.clone());
        let res2 = total_borrow_shares_wad.add(&borrow_shares_wad.clone());
        let res3 = borrows_wad.add(&amount_wad.clone());

        Self::set_user_borrow_shares(&env, smart_account.clone(), res1);
        Self::set_total_borrow_shares(&env, res2);
        env.storage().persistent().set(&key_c, &res3);
        Self::extend_ttl_pooldatakey(env, key_c);

        // Now transfer amount to trader's smart account address
        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);
        let amount_wad_u128: u128 = amount_wad
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let amount_scaled = Self::scale_for_operation(amount_wad_u128, eurc_token.decimals());

        eurc_token.transfer(
            &env.current_contract_address(), // from
            &smart_account,                  // to
            &amount_scaled,
        );

        // let bal = eurc_token.balance(&smart_account);
        // log!(
        //     &env,
        //     "Balance of margin account after lending {:?}",
        //     bal,
        //     smart_account.clone()
        // );

        let smart_account_client = smart_account_contract::Client::new(&env, &smart_account);
        smart_account_client.add_borrowed_token(&Symbol::new(&env, "EURC"));
        smart_account_client.set_has_debt(&true, &Symbol::new(&env, "EURC"));

        Ok(is_first_borrow)
    }

    pub fn collect_from(
        env: &Env,
        amount_wad: U256,
        trader_smart_account: Address,
    ) -> Result<bool, LendingError> {
        let account_manager: Address = Self::get_account_manager(env);
        account_manager.require_auth();
        Self::update_state(env);

        let borrow_shares_wad = Self::convert_asset_borrow_shares(env, amount_wad.clone());
        if borrow_shares_wad == U256::from_u32(&env, 0) {
            panic!("Zero borrow shares");
        }

        let user_borrow_shares_wad: U256 =
            Self::get_user_borrow_shares(env, trader_smart_account.clone());
        let total_borrow_shares_wad: U256 = Self::get_total_borrow_shares(env);
        let key_c = PoolDataKey::BorrowsWAD;
        let borrows_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| U256::from_u128(&env, 0));

        let amount_wad_u128: u128 = amount_wad
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let smart_account_client = smart_account_contract::Client::new(&env, &trader_smart_account);
        smart_account_client
            .remove_borrowed_token_balance(&Symbol::new(&env, "EURC"), &amount_wad_u128);

        log!(&env, "reached3344", user_borrow_shares_wad, borrows_wad);
        let res1 = user_borrow_shares_wad.sub(&borrow_shares_wad);
        log!(&env, "reached5566");

        let res2 = total_borrow_shares_wad.sub(&borrow_shares_wad);
        log!(&env, "reached6677");

        let res3 = borrows_wad.sub(&amount_wad);
        log!(&env, "reached7788");

        if res1 == U256::from_u32(&env, 0) {
            smart_account_client.remove_borrowed_token(&Symbol::new(&env, "EURC"));
        }

        Self::set_user_borrow_shares(env, trader_smart_account.clone(), res1.clone());
        Self::set_total_borrow_shares(env, res2);
        env.storage().persistent().set(&key_c, &res3);
        return Ok(res1 == U256::from_u32(&env, 0));
    }

    fn mint_veurc_tokens(env: &Env, lender: Address, tokens_to_mint_wad: U256) {
        let tokens_to_mint_wad_u128: u128 = tokens_to_mint_wad
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let veurc_token_contract_address: Address = Self::get_vtoken_contract_address(env);

        let eurc_token_client =
            veurc_token_contract::Client::new(&env, &veurc_token_contract_address);

        let tokens_to_mint_scaled =
            Self::scale_for_operation(tokens_to_mint_wad_u128, eurc_token_client.decimals());

        // mint tokens to his address.
        eurc_token_client.mint(&lender, &tokens_to_mint_scaled); // Mint tokens to recipient

        // // Update total token balance available right now
        // let current_total_token_balance_wad = Self::get_current_total_veurc_balance(env);
        // let new_total_token_balance_wad = current_total_token_balance_wad.add(&tokens_to_mint_wad);
        // Self::set_total_vtoken_balance(env, &new_total_token_balance_wad);

        let total_minted_wad = Self::get_total_veurc_minted(env);
        let new_total_minted_wad = total_minted_wad.add(&tokens_to_mint_wad);
        let key_y = TokenDataKey::TotalTokensMintedWAD(Symbol::new(&env, "VEURC"));
        env.storage()
            .persistent()
            .set(&key_y, &new_total_minted_wad);
        Self::extend_ttl_tokendatakey(&env, key_y);

        env.events().publish(
            (Symbol::new(&env, "mint_event"), lender.clone()),
            LendingTokenMintEvent {
                lender: lender.clone(),
                token_amount: tokens_to_mint_wad,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "VEURC"),
            },
        );
    }

    fn burn_veurc_tokens(env: &Env, lender: Address, tokens_to_burn_wad: U256) {
        let tokens_to_burn_wad_u128: u128 = tokens_to_burn_wad
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let veurc_token_contract_address: Address = Self::get_vtoken_contract_address(env);

        let eurc_token_client =
            veurc_token_contract::Client::new(&env, &veurc_token_contract_address);

        let tokens_to_burn_scaled =
            Self::scale_for_operation(tokens_to_burn_wad_u128, eurc_token_client.decimals());
        // burn tokens from his address.
        eurc_token_client.burn(&lender, &tokens_to_burn_scaled);

        // let current_total_token_balance_wad = Self::get_current_total_veurc_balance(env);
        // let new_total_token_balance_wad = current_total_token_balance_wad.sub(&tokens_to_burn_wad);
        // Self::set_total_vtoken_balance(env, &new_total_token_balance_wad);

        let total_burnt_wad = Self::get_total_veurc_burnt(env);
        let new_total_burnt_wad = total_burnt_wad.add(&tokens_to_burn_wad);
        let key_a = TokenDataKey::TotalTokensBurntWAD(Symbol::new(&env, "VEURC"));
        env.storage().persistent().set(&key_a, &new_total_burnt_wad);
        Self::extend_ttl_tokendatakey(&env, key_a);

        env.events().publish(
            (Symbol::new(&env, "burn_event"), lender.clone()),
            LendingTokenBurnEvent {
                lender: lender.clone(),
                token_amount: tokens_to_burn_wad,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "VEURC"),
                // token_value,
            },
        );
    }

    pub fn convert_asset_borrow_shares(env: &Env, amount_wad: U256) -> U256 {
        let key_b = PoolDataKey::TotalBorrowSharesWAD;
        let total_borrow_shares_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));

        if total_borrow_shares_wad == U256::from_u32(&env, 0) {
            return amount_wad;
        } else {
            let res = amount_wad.mul(&total_borrow_shares_wad);
            let result_wad = res.div(&Self::get_borrows(&env));
            return result_wad;
        }
    }

    pub fn convert_borrow_shares_asset(env: &Env, debt_wad: U256) -> U256 {
        let key_b = PoolDataKey::TotalBorrowSharesWAD;
        let total_borrow_shares_wad: U256 = env.storage().persistent().get(&key_b).unwrap();
        if total_borrow_shares_wad == U256::from_u32(&env, 0) {
            return debt_wad;
        } else {
            let res_wad = debt_wad.mul(&Self::get_borrows(&env));
            let result_wad = res_wad.div(&total_borrow_shares_wad);
            return result_wad;
        }
    }

    fn mul_wad_down(env: &Env, a: &U256, b: &U256) -> U256 {
        a.mul(b).div(&U256::from_u128(&env, WAD_U128))
    }

    pub fn update_state(env: &Env) {
        let lastupdatetime = Self::get_last_updated_time(&env);
        if lastupdatetime == env.ledger().timestamp() {
            env.storage()
                .persistent()
                .set(&PoolDataKey::LastUpdatedTime, &env.ledger().timestamp());
            log!(&env, "Time not elapsed, no update state");
            return;
        }
        log!(&env, "reached inside update state!", lastupdatetime);

        let key_c = PoolDataKey::BorrowsWAD;
        let borrows_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        log!(&env, "Existing borrows", borrows_wad);
        let rate_factor_wad = Self::get_rate_factor(&env).unwrap();
        // let interest_accrued = Self::mul_wad_down(env, &borrows, &rate_factor);
        let interest_accrued_wad = Self::mul_wad_down(env, &borrows_wad, &rate_factor_wad);

        // let interest_accrued_wad = borrows_wad.mul(&rate_factor_wad);
        log!(&env, "interest_accrued iss", interest_accrued_wad);

        let res_wad = borrows_wad.add(&interest_accrued_wad);

        log!(&env, "interest_accrued borrows", res_wad);

        env.storage().persistent().set(&key_c, &res_wad);
        log!(&env, "Just updated state at!", env.ledger().timestamp());

        env.storage()
            .persistent()
            .set(&PoolDataKey::LastUpdatedTime, &env.ledger().timestamp());
    }

    pub fn before_deposit(env: &Env) {
        Self::update_state(env);
    }
    pub fn before_withdraw(env: &Env) {
        Self::update_state(env);
    }

    pub fn get_user_borrow_shares(env: &Env, trader: Address) -> U256 {
        let key_a = PoolDataKey::UserBorrowSharesWAD(trader.clone());
        let user_borrow_shares_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        return user_borrow_shares_wad;
    }

    fn set_user_borrow_shares(env: &Env, trader: Address, res: U256) {
        let key_a = PoolDataKey::UserBorrowSharesWAD(trader);
        env.storage().persistent().set(&key_a, &res);
        Self::extend_ttl_pooldatakey(env, key_a);
    }
    pub fn get_borrow_balance(env: &Env, trader: Address) -> U256 {
        let key_a = PoolDataKey::UserBorrowSharesWAD(trader.clone());
        let user_borrow_shares_wad: U256 = env.storage().persistent().get(&key_a).unwrap();
        let res = Self::convert_borrow_shares_asset(env, user_borrow_shares_wad);
        res
    }

    pub fn get_total_borrow_shares(env: &Env) -> U256 {
        let key_b = PoolDataKey::TotalBorrowSharesWAD;
        env.storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn set_total_borrow_shares(env: &Env, res: U256) {
        let key_b = PoolDataKey::TotalBorrowSharesWAD;
        env.storage().persistent().set(&key_b, &res);
        Self::extend_ttl_pooldatakey(env, key_b);
    }

    pub fn total_assets(env: &Env) -> U256 {
        let assets_wad = Self::get_total_liquidity_in_pool(&env);
        let borrows = Self::get_borrows(env);
        let total_assets = assets_wad.add(&borrows);
        total_assets
    }

    pub fn get_borrows(env: &Env) -> U256 {
        let key_c = PoolDataKey::BorrowsWAD;

        let borrows_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let rate_factor_wad = Self::get_rate_factor(&env).unwrap();
        let res = Self::mul_wad_down(&env, &borrows_wad, &rate_factor_wad);
        // let res = borrows.mul(&);
        let result_wad = borrows_wad.add(&res);
        result_wad
    }

    pub fn get_rate_factor(env: &Env) -> Result<U256, InterestRateError> {
        let registy_address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registy_address);
        let rate_model_address = registry_client.get_rate_model_address();
        let rate_model_client = rate_model_contract::Client::new(&env, &rate_model_address);

        let lastupdatetime = Self::get_last_updated_time(&env);
        log!(&env, "reached inside get_rate_factor!", lastupdatetime);

        let blocktimestamp = env.ledger().timestamp();
        log!(&env, "reached blocktimestamp!", blocktimestamp);

        if lastupdatetime == blocktimestamp {
            log!(&env, "returning from get rate factor");
            return Ok(U256::from_u32(&env, 0));
        }
        let key_c = PoolDataKey::BorrowsWAD;

        let borrows_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let liquidity_wad = Self::get_total_liquidity_in_pool(&env);
        // let liquidity_wad = Self::up_wad(&env, liquidity);
        log!(&env, "Total liquidity in the pool wad", (liquidity_wad));
        log!(&env, "Total borrows in the pool wad", (borrows_wad));

        log!(&env, "Time difference", (blocktimestamp - lastupdatetime));

        let res_wad = U256::from_u128(&env, (blocktimestamp - lastupdatetime) as u128)
            .mul(&(rate_model_client.get_borrow_rate_per_sec(&liquidity_wad, &borrows_wad)));
        log!(&env, "returning rate_factor wad!", res_wad);

        Ok(res_wad)
    }

    pub fn get_total_liquidity_in_pool(env: &Env) -> U256 {
        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);
        let current_pool_balance = eurc_token.balance(&env.current_contract_address());
        let current_pool_balance_wad =
            Self::scale_for_balance(current_pool_balance, eurc_token.decimals());
        U256::from_u128(&env, current_pool_balance_wad as u128)
    }

    pub fn get_last_updated_time(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&PoolDataKey::LastUpdatedTime)
            .unwrap_or_else(|| env.ledger().timestamp())
    }

    pub fn get_current_total_veurc_balance(env: &Env) -> U256 {
        let veurc_token_contract_address: Address = Self::get_vtoken_contract_address(env);
        let eurc_token_client =
            veurc_token_contract::Client::new(&env, &veurc_token_contract_address);
        let total_supply = eurc_token_client.total_supply();
        let total_supply_wad = Self::scale_for_balance(total_supply, eurc_token_client.decimals());
        log!(&env, "total supply veurc wad", total_supply_wad);
        U256::from_u128(&env, total_supply_wad as u128)

        // env.storage()
        //     .persistent()
        //     .get(&TokenDataKey::CurrentVTokenBalanceWAD(Symbol::new(
        //         &env, "VEURC",
        //     )))
        //     .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_veurc_minted(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensMintedWAD(Symbol::new(
                &env, "VEURC",
            )))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_veurc_burnt(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensBurntWAD(Symbol::new(
                &env, "VEURC",
            )))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    // Helper function to add lender to list
    fn add_lender_to_list_eurc(env: &Env, lender: &Address) {
        let key_b = PoolDataKey::Lenders(Symbol::new(&env, "EURC"));
        let mut lenders: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| Vec::new(&env));

        if !lenders.contains(lender) {
            lenders.push_back(lender.clone());
            env.storage().persistent().set(&key_b, &lenders);
            Self::extend_ttl_pooldatakey(&env, key_b);
        }
    }

    // Function to get all lenders
    pub fn get_lenders_eurc(env: Env) -> Vec<Address> {
        let list_address: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "EURC")))
            .unwrap_or_else(|| Vec::new(&env));
        list_address
    }

    pub fn is_eurc_pool_initialised(env: &Env) -> bool {
        if env.storage().persistent().has(&PoolDataKey::Initialised) {
            env.storage()
                .persistent()
                .get(&PoolDataKey::Initialised)
                .unwrap()
        } else {
            panic!("Lending pool not initialised")
        }
    }

    pub fn get_native_eurc_client_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&TokenDataKey::NativeEURCClientAddress)
            .unwrap_or_else(|| panic!("Native EURC client address not set"))
    }

    fn get_vtoken_contract_address(env: &Env) -> Address {
        let veurc_symbol = Symbol::new(&env, "VEURC");
        env.storage()
            .persistent()
            .get(&TokenDataKey::VTokenContractAddress(veurc_symbol))
            .unwrap_or_else(|| panic!("Failed to fetch VEURC Token contract address"))
    }

    // fn set_total_vtoken_balance(env: &Env, new_total_token_balance_wad: &U256) {
    //     let key_x = TokenDataKey::CurrentVTokenBalanceWAD(Symbol::new(&env, "VEURC"));
    //     env.storage()
    //         .persistent()
    //         .set(&key_x, new_total_token_balance_wad);
    //     Self::extend_ttl_tokendatakey(&env, key_x);
    // }

    pub fn up_wad(env: &Env, x: U256) -> U256 {
        x.mul(&U256::from_u128(&env, WAD_U128))
    }

    pub fn down_wad(env: &Env, x: U256) -> U256 {
        x.div(&U256::from_u128(&env, WAD_U128))
    }

    fn scale_for_operation(amount_wad: u128, eurc_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(eurc_decimals)) / WAD_U128) as i128
    }

    fn scale_for_balance(amount: i128, decimal: u32) -> i128 {
        amount * (WAD_U128 as i128) / 10i128.pow(decimal)
    }

    fn get_account_manager(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&ContractDetails::AccountManager)
            .expect("Account manager contract address not set !")
    }
    // Converts EURC to VEURC
    pub fn convert_eurc_to_vtoken(env: &Env, amount_wad: U256) -> U256 {
        let pool_balance_wad = Self::get_total_liquidity_in_pool(env);
        let minted_wad = Self::get_total_veurc_minted(env);

        if pool_balance_wad == U256::from_u128(&env, 0) || minted_wad == U256::from_u128(&env, 0) {
            amount_wad
        } else {
            let supply_wad = Self::get_current_total_veurc_balance(env);
            let res = amount_wad.mul(&supply_wad);
            let resx_wad = res.div(&pool_balance_wad);

            resx_wad
        }
    }

    //  Converting VEURC to EURC
    pub fn convert_vtoken_to_eurc(env: &Env, vtokens_to_be_burnt_wad: U256) -> U256 {
        let pool_balance_wad = Self::get_total_liquidity_in_pool(env);
        let v_token_supply_wad = Self::get_current_total_veurc_balance(env);
        let res = vtokens_to_be_burnt_wad.mul(&pool_balance_wad);
        let resx_wad = res.div(&v_token_supply_wad);
        resx_wad
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&ContractDetails::RegistryContract)
            .expect("Failed to fetch registry contract")
    }

    fn extend_ttl_datakey(env: &Env, key: DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn extend_ttl_pooldatakey(env: &Env, key: PoolDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn extend_ttl_tokendatakey(env: &Env, key: TokenDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn extend_ttl_contractdatakey(env: &Env, key: ContractDetails) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
