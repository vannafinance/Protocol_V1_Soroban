use core::panic;

use crate::errors::{InterestRateError, LendingError};
use crate::events::{
    LendingDepositEvent, LendingTokenBurnEvent, LendingTokenMintEvent, LendingWithdrawEvent,
};
use crate::types::{ContractDetails, PoolDataKey, TokenDataKey};
use soroban_sdk::{
    Address, Env, String, Symbol, U256, Vec, contract, contractimpl, log, panic_with_error,
    symbol_short, token,
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

pub mod vxlm_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/vxlm_token_contract.wasm"
    );
}

#[contract]
pub struct LiquidityPoolXLM;

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const VXLM_SYMBOL: Symbol = symbol_short!("VXLM");

pub const WAD_U128: u128 = 10000_0000_00000_00000; // 1e18

#[contractimpl]
impl LiquidityPoolXLM {
    pub fn __constructor(
        env: Env,
        admin: Address,
        native_token_address: Address,
        registry_contract: Address,
        account_manager: Address,
        rate_model: Address,
        token_issuer: Address,
        treasury: Address,
        origination_fee: U256,
    ) {
        let key = PoolDataKey::Admin;

        env.storage().persistent().set(&PoolDataKey::Admin, &admin);
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

        env.storage()
            .persistent()
            .set(&ContractDetails::Treasury, &treasury);
        Self::extend_ttl_contractdatakey(&env, ContractDetails::Treasury);
        env.events()
            .publish(("constructor", "treasury_set"), &treasury);

        env.storage()
            .persistent()
            .set(&ContractDetails::OriginationFee, &origination_fee);
        Self::extend_ttl_contractdatakey(&env, ContractDetails::OriginationFee);
        env.events()
            .publish(("constructor", "origination_fee_set"), &origination_fee);

        env.storage()
            .persistent()
            .set(&TokenDataKey::NativeXLMAddress, &native_token_address);
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::NativeXLMAddress);
        env.events()
            .publish(("constructor", "native_xlm_set"), &native_token_address);

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

        let key = PoolDataKey::Admin;

        env.storage().persistent().set(&PoolDataKey::Admin, &admin);
        Self::extend_ttl_datakey(&env, key);
        Ok(String::from_str(&env, "Adminkey set successfully reset"))
    }

    pub fn get_admin(env: &Env) -> Result<Address, LendingError> {
        let key = PoolDataKey::Admin;
        let admin_address: Address = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("Admin key has not been set"));
        Ok(admin_address)
    }

    pub fn initialize_pool_xlm(
        env: Env,
        vxlm_token_contract_address: Address,
    ) -> Result<String, LendingError> {
        let admin: Address = Self::get_admin(&env).unwrap();
        admin.require_auth();

        env.storage().persistent().set(
            &TokenDataKey::VTokenContractAddress(VXLM_SYMBOL),
            &vxlm_token_contract_address,
        );
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::VTokenContractAddress(VXLM_SYMBOL));

        env.storage()
            .persistent()
            .set(&PoolDataKey::Initialised, &true);
        Self::extend_ttl_pooldatakey(&env, PoolDataKey::Initialised);

        env.events()
            .publish(("initialize_pool_xlm", "xlm_pool_initialized"), true);
        Ok(String::from_str(&env, "XLM pool initialised"))
    }

    pub fn deposit_xlm(env: Env, lender: Address, amount_wad: U256) {
        lender.require_auth();
        if amount_wad <= U256::from_u128(&env, 0) {
            panic!("Deposit amount must be positive");
        }
        // Check if pool is initialised
        Self::is_xlm_pool_initialised(&env);
        Self::before_deposit(&env);
        let amount_wad_u128 = Self::convert_u256_to_u128(&env, &amount_wad);

        // Getting the amount of tokens to be minted for Asset deposited
        let vtokens_to_be_minted_wad = Self::convert_xlm_to_vtoken(&env, amount_wad.clone());

        let native_token_address: Address = Self::get_native_xlm_client_address(&env);
        let xlm_token = token::Client::new(&env, &native_token_address);

        let user_balance = xlm_token.balance(&lender);
        let user_balance_wad = Self::scale_for_balance(user_balance, xlm_token.decimals()) as u128;

        if user_balance_wad < amount_wad_u128 {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }

        log!(&env, "reached zzzssss");
        let amount_scaled = Self::scale_for_operation(amount_wad_u128, xlm_token.decimals());
        // Transfer XLM from user to this contract
        xlm_token.transfer(
            &lender,                         // from
            &env.current_contract_address(), // to
            &amount_scaled,
        );

        // Update lender list
        Self::add_lender_to_list_xlm(&env, &lender);

        // Now Mint the VXLM tokens that were created for the lender
        Self::mint_vxlm_tokens(&env, lender.clone(), vtokens_to_be_minted_wad);

        env.events().publish(
            (Symbol::new(&env, "deposit_event"), lender.clone()),
            LendingDepositEvent {
                lender: lender.clone(),
                amount: amount_wad,
                timestamp: env.ledger().timestamp(),
                asset_symbol: XLM_SYMBOL,
            },
        );
    }

    pub fn redeem_vxlm(env: &Env, lender: Address, tokens_to_redeem_wad: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_xlm_pool_initialised(&env);
        Self::before_withdraw(env);

        let vxlm_token_contract_address: Address = Self::get_vtoken_contract_address(env);

        let vxlm_token_client =
            vxlm_token_contract::Client::new(&env, &vxlm_token_contract_address);
        let vxlm_balance_wad = Self::scale_for_balance(
            vxlm_token_client.balance(&lender),
            vxlm_token_client.decimals(),
        );
        let vxlm_balance_wad_u256 = U256::from_u128(&env, vxlm_balance_wad as u128);

        // Check if lender has enough token balance to redeem
        if tokens_to_redeem_wad > vxlm_balance_wad_u256 {
            panic!("Insufficient Token Balance to redeem");
        }

        let xlm_value_to_transfer_wad =
            Self::convert_vtoken_to_xlm(env, tokens_to_redeem_wad.clone());
        let native_token_address: Address = Self::get_native_xlm_client_address(&env);
        let xlm_token = token::Client::new(&env, &native_token_address);
        let current_pool_balance_wad = Self::get_total_liquidity_in_pool(&env);

        log!(
            &env,
            "pool bal, xlm_transfer {:?},{:?}",
            current_pool_balance_wad,
            xlm_value_to_transfer_wad
        );
        // Check if there is enough balance in the pool to redeem
        if current_pool_balance_wad < xlm_value_to_transfer_wad {
            panic_with_error!(&env, LendingError::InsufficientPoolBalance);
        }

        let amount_wad_u128: u128 = Self::convert_u256_to_u128(&env, &xlm_value_to_transfer_wad);

        let amount_scaled = Self::scale_for_operation(amount_wad_u128, xlm_token.decimals());

        xlm_token.transfer(
            &env.current_contract_address(), // from
            &lender,                         // to
            &amount_scaled,
        );

        Self::burn_vxlm_tokens(&env, lender.clone(), tokens_to_redeem_wad.clone());

        // emit event after withdraw
        env.events().publish(
            (Symbol::new(&env, "withdraw_event"), lender.clone()),
            LendingWithdrawEvent {
                lender: lender,
                vtoken_amount: tokens_to_redeem_wad,
                timestamp: env.ledger().timestamp(),
                asset_symbol: XLM_SYMBOL,
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
        let native_token_address: Address = Self::get_native_xlm_client_address(&env);
        let xlm_token = token::Client::new(&env, &native_token_address);
        let amount_wad_u128 = Self::convert_u256_to_u128(env, &amount_wad);
        let amount_scaled = Self::scale_for_operation(amount_wad_u128, xlm_token.decimals());

        let origination_fee_wad = Self::get_origination_fee(env);
        let origination_fee_mul_wad = Self::mul_wad_down(&env, &amount_wad, &origination_fee_wad);
        let ori_fee_mul_wad_u128 = Self::convert_u256_to_u128(env, &origination_fee_mul_wad);
        let ori_fee_scaled = Self::scale_for_operation(ori_fee_mul_wad_u128, xlm_token.decimals());

        let treasury = Self::get_treasury(env);

        log!(&env, "Sending to treasury", ori_fee_scaled, amount_scaled);

        // Transfering origination fee to treasury
        xlm_token.transfer(&env.current_contract_address(), &treasury, &ori_fee_scaled);

        log!(&env, "Lending to user account");

        xlm_token.transfer(
            &env.current_contract_address(), // from
            &smart_account,                  // to
            &amount_scaled,
        );

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

        log!(&env, "reached3344", user_borrow_shares_wad, borrows_wad);
        let res1 = user_borrow_shares_wad.sub(&borrow_shares_wad);
        log!(&env, "reached5566");

        let res2 = total_borrow_shares_wad.sub(&borrow_shares_wad);
        log!(&env, "reached6677");

        let res3 = borrows_wad.sub(&amount_wad);
        log!(&env, "reached7788");

        // if res1 == U256::from_u32(&env, 0) {
        //     smart_account_client.remove_borrowed_token(&XLM_SYMBOL);
        // }

        Self::set_user_borrow_shares(env, trader_smart_account.clone(), res1.clone());
        Self::set_total_borrow_shares(env, res2);
        env.storage().persistent().set(&key_c, &res3);
        Self::extend_ttl_pooldatakey(env, key_c);

        return Ok(res1 == U256::from_u32(&env, 0));
    }

    fn mint_vxlm_tokens(env: &Env, lender: Address, tokens_to_mint_wad: U256) {
        let tokens_to_mint_wad_u128: u128 = Self::convert_u256_to_u128(&env, &tokens_to_mint_wad);

        let vxlm_token_contract_address: Address = Self::get_vtoken_contract_address(env);

        let vxlm_token_client =
            vxlm_token_contract::Client::new(&env, &vxlm_token_contract_address);

        let tokens_to_mint_scaled =
            Self::scale_for_operation(tokens_to_mint_wad_u128, vxlm_token_client.decimals());

        vxlm_token_client.mint(&lender, &tokens_to_mint_scaled); // Mint tokens to recipient

        let total_minted_wad = Self::get_total_vxlm_minted(env);
        let new_total_minted_wad = total_minted_wad.add(&tokens_to_mint_wad);
        let key_y = TokenDataKey::TotalTokensMintedWAD(VXLM_SYMBOL);
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
                token_symbol: VXLM_SYMBOL,
            },
        );
    }

    fn burn_vxlm_tokens(env: &Env, lender: Address, tokens_to_burn_wad: U256) {
        let tokens_to_burn_wad_u128: u128 = Self::convert_u256_to_u128(&env, &tokens_to_burn_wad);

        let vxlm_token_contract_address: Address = Self::get_vtoken_contract_address(env);

        let vxlm_token_client =
            vxlm_token_contract::Client::new(&env, &vxlm_token_contract_address);

        let tokens_to_burn_scaled =
            Self::scale_for_operation(tokens_to_burn_wad_u128, vxlm_token_client.decimals());
        // burn tokens from his address.
        vxlm_token_client.burn(&lender, &tokens_to_burn_scaled);

        let total_burnt_wad = Self::get_total_vxlm_burnt(env);
        let new_total_burnt_wad = total_burnt_wad.add(&tokens_to_burn_wad);
        let key_a = TokenDataKey::TotalTokensBurntWAD(VXLM_SYMBOL);
        env.storage().persistent().set(&key_a, &new_total_burnt_wad);
        Self::extend_ttl_tokendatakey(&env, key_a);

        env.events().publish(
            (Symbol::new(&env, "burn_event"), lender.clone()),
            LendingTokenBurnEvent {
                lender: lender.clone(),
                token_amount: tokens_to_burn_wad,
                timestamp: env.ledger().timestamp(),
                token_symbol: VXLM_SYMBOL,
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
        let total_borrow_shares_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
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
        let key = PoolDataKey::LastUpdatedTime;
        if lastupdatetime == env.ledger().timestamp() {
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
        let interest_accrued_wad = Self::mul_wad_down(env, &borrows_wad, &rate_factor_wad);
        log!(&env, "interest_accrued iss", interest_accrued_wad);
        let res_wad = borrows_wad.add(&interest_accrued_wad);
        log!(&env, "interest_accrued borrows", res_wad);
        env.storage().persistent().set(&key_c, &res_wad);
        log!(&env, "Just updated state at!", env.ledger().timestamp());
        Self::extend_ttl_pooldatakey(env, key_c);

        env.storage()
            .persistent()
            .set(&key, &env.ledger().timestamp());
        Self::extend_ttl_pooldatakey(env, key);
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
        let user_borrow_shares_wad: U256 = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let res_wad = Self::convert_borrow_shares_asset(env, user_borrow_shares_wad);
        res_wad
    }

    pub fn get_total_borrow_shares(env: &Env) -> U256 {
        let key_b = PoolDataKey::TotalBorrowSharesWAD;
        env.storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    fn set_total_borrow_shares(env: &Env, res: U256) {
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
        log!(&env, "Total liquidity in the pool wad", (liquidity_wad));
        log!(&env, "Total borrows in the pool wad", (borrows_wad));

        log!(&env, "Time difference", (blocktimestamp - lastupdatetime));

        let res_wad = U256::from_u128(&env, (blocktimestamp - lastupdatetime) as u128)
            .mul(&(rate_model_client.get_borrow_rate_per_sec(&liquidity_wad, &borrows_wad)));
        log!(&env, "returning rate_factor wad!", res_wad);

        Ok(res_wad)
    }

    pub fn get_total_liquidity_in_pool(env: &Env) -> U256 {
        let native_token_address: Address = Self::get_native_xlm_client_address(&env);
        let xlm_token = token::Client::new(&env, &native_token_address);
        let current_pool_balance = xlm_token.balance(&env.current_contract_address());
        let current_pool_balance_wad =
            Self::scale_for_balance(current_pool_balance, xlm_token.decimals());
        U256::from_u128(&env, current_pool_balance_wad as u128)
    }

    pub fn get_last_updated_time(env: &Env) -> u64 {
        let key = PoolDataKey::LastUpdatedTime;
        env.storage().persistent().get(&key).unwrap_or_else(|| {
            let time: u64 = env.ledger().timestamp();
            env.storage().persistent().set(&key, &time);
            Self::extend_ttl_pooldatakey(env, key);
            time
        })
    }

    pub fn get_current_total_vxlm_balance(env: &Env) -> U256 {
        let vxlm_token_contract_address: Address = Self::get_vtoken_contract_address(env);
        let xlm_token_client = vxlm_token_contract::Client::new(&env, &vxlm_token_contract_address);
        let total_supply = xlm_token_client.total_supply();
        let total_supply_wad = Self::scale_for_balance(total_supply, xlm_token_client.decimals());
        log!(&env, "total supply vxlm wad", total_supply_wad);
        U256::from_u128(&env, total_supply_wad as u128)
    }

    pub fn get_total_vxlm_minted(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensMintedWAD(VXLM_SYMBOL))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_vxlm_burnt(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensBurntWAD(VXLM_SYMBOL))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    // Helper function to add lender to list
    fn add_lender_to_list_xlm(env: &Env, lender: &Address) {
        let key_b = PoolDataKey::Lenders(XLM_SYMBOL);
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
    pub fn get_lenders_xlm(env: Env) -> Vec<Address> {
        let list_address: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(XLM_SYMBOL))
            .unwrap_or_else(|| Vec::new(&env));
        list_address
    }

    pub fn is_xlm_pool_initialised(env: &Env) -> bool {
        if env.storage().persistent().has(&PoolDataKey::Initialised) {
            env.storage()
                .persistent()
                .get(&PoolDataKey::Initialised)
                .unwrap()
        } else {
            panic!("Lending pool not initialised")
        }
    }

    pub fn get_native_xlm_client_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&TokenDataKey::NativeXLMAddress)
            .unwrap_or_else(|| panic!("Native XLM client address not set"))
    }

    fn get_vtoken_contract_address(env: &Env) -> Address {
        let vxlm_symbol = VXLM_SYMBOL;
        env.storage()
            .persistent()
            .get(&TokenDataKey::VTokenContractAddress(vxlm_symbol))
            .unwrap_or_else(|| panic!("Failed to fetch VXLM Token contract address"))
    }

    pub fn up_wad(env: &Env, x: U256) -> U256 {
        x.mul(&U256::from_u128(&env, WAD_U128))
    }

    pub fn down_wad(env: &Env, x: U256) -> U256 {
        x.div(&U256::from_u128(&env, WAD_U128))
    }

    fn scale_for_operation(amount_wad: u128, xlm_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(xlm_decimals)) / WAD_U128) as i128
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
    // Converts XLM to VXLM
    pub fn convert_xlm_to_vtoken(env: &Env, amount_wad: U256) -> U256 {
        let pool_balance_wad = Self::get_total_liquidity_in_pool(env);
        let minted_wad = Self::get_total_vxlm_minted(env);

        if pool_balance_wad == U256::from_u128(&env, 0) || minted_wad == U256::from_u128(&env, 0) {
            amount_wad
        } else {
            let supply_wad = Self::get_current_total_vxlm_balance(env);
            let res = amount_wad.mul(&supply_wad);
            let resx_wad = res.div(&pool_balance_wad);

            resx_wad
        }
    }

    //  Converting VXLM to XLM
    pub fn convert_vtoken_to_xlm(env: &Env, vtokens_to_be_burnt_wad: U256) -> U256 {
        let pool_balance_wad = Self::get_total_liquidity_in_pool(env);
        log!(&env, "Pool balance wad", pool_balance_wad);
        let v_token_supply_wad = Self::get_current_total_vxlm_balance(env);
        log!(&env, "v_token_supply_wad", v_token_supply_wad);

        let res = vtokens_to_be_burnt_wad.mul(&pool_balance_wad);
        let resx_wad = res.div(&v_token_supply_wad);
        log!(&env, "resx_wad", resx_wad);

        resx_wad
    }

    pub fn update_origination_fee(env: &Env, origination_fee: U256) {
        let admin: Address = Self::get_admin(&env).unwrap();
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&ContractDetails::OriginationFee, &origination_fee);
        Self::extend_ttl_contractdatakey(&env, ContractDetails::OriginationFee);
    }

    pub fn get_origination_fee(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&ContractDetails::OriginationFee)
            .unwrap_or_else(|| panic!("Origination fee not initialised"))
    }

    pub fn get_treasury(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&ContractDetails::Treasury)
            .unwrap_or_else(|| panic!("Treasury address not set"))
    }

    fn convert_u256_to_u128(env: &Env, x: &U256) -> u128 {
        x.to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError))
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&ContractDetails::RegistryContract)
            .expect("Failed to fetch registry contract")
    }

    fn extend_ttl_datakey(env: &Env, key: PoolDataKey) {
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
