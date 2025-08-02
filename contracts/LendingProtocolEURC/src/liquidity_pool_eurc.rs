use core::panic;

use crate::errors::{InterestRateError, LendingError, LendingTokenError};
use crate::events::{
    LendingDepositEvent, LendingTokenBurnEvent, LendingTokenMintEvent, LendingWithdrawEvent,
};
use crate::types::{ContractDetails, DataKey, PoolDataKey, TokenDataKey};
use soroban_sdk::{
    Address, Env, String, Symbol, U256, Vec, contract, contractimpl, panic_with_error, token,
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

pub mod risk_engine_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/risk_engine_contract.wasm"
    );
}

#[contract]
pub struct LiquidityPoolEURC;

// pub const EURC_CONTRACT_ID: [u8; 32] = [0; 32];
const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;

#[contractimpl]
impl LiquidityPoolEURC {
    pub fn __constructor(
        env: Env,
        admin: Address,
        native_token_address: Address,
        veurc_token_address: Address,
        registry_contract: Address,
        account_manager: Address,
        rate_model: Address,
        token_issuer: Address,
    ) {
        let key = DataKey::Admin;

        env.storage().persistent().set(&DataKey::Admin, &admin);
        Self::extend_ttl_datakey(&env, key);

        env.events().publish(("constructor", "admin_set"), &admin);
        let veurc_symbol = Symbol::new(&env, "VEURC");

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
            &TokenDataKey::VTokenClientAddress(veurc_symbol.clone()),
            &veurc_token_address,
        );
        Self::extend_ttl_tokendatakey(
            &env,
            TokenDataKey::VTokenClientAddress(veurc_symbol.clone()),
        );

        env.storage()
            .persistent()
            .set(&TokenDataKey::EurcClientAddress, &native_token_address);
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::EurcClientAddress);
        env.events()
            .publish(("constructor", "native_eurc_set"), &native_token_address);

        env.storage()
            .persistent()
            .set(&TokenDataKey::TokenIssuerAddress, &token_issuer);
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::TokenIssuerAddress);
        env.events()
            .publish(("constructor", "token_issuer_set"), &token_issuer);
    }

    pub fn set_admin(env: Env, admin: Address) -> Result<String, LendingError> {
        // Resetting the admin, can be done only by exisiting admin
        let admin_existing = Self::get_admin(&env).unwrap();
        admin_existing.require_auth();

        let key = DataKey::Admin;

        env.storage().persistent().set(&DataKey::Admin, &admin);
        Self::extend_ttl_datakey(&env, key);
        Ok(String::from_str(&env, "Adminkey set successfully"))
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

    pub fn initialize_pool_eurc(env: Env) -> Result<String, LendingError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("Admin not set");

        admin.require_auth();
        let eurc_symbol = Symbol::new(&env, "EURC");
        let veurc_symbol = Symbol::new(&env, "VEURC");

        // env.storage()
        //     .persistent()
        //     .get(&ContractDetails::RegistryContract);
        // Self::extend_ttl_contractdatakey(&env, ContractDetails::RegistryContract);

        // env.storage()
        //     .persistent()
        //     .set(&ContractDetails::AccountManager, &account_manager);
        // Self::extend_ttl_contractdatakey(&env, ContractDetails::AccountManager);

        // env.storage()
        //     .persistent()
        //     .set(&ContractDetails::RateModel, &rate_model);
        // Self::extend_ttl_contractdatakey(&env, ContractDetails::RateModel);

        // env.storage()
        //     .persistent()
        //     .set(&ContractDetails::Treasury, &treasury_address);
        // Self::extend_ttl_contractdatakey(&env, ContractDetails::Treasury);

        // env.storage().persistent().set(
        //     &TokenDataKey::VTokenClientAddress(veurc_symbol.clone()),
        //     &veurc_token_address,
        // );
        // Self::extend_ttl_tokendatakey(&env, TokenDataKey::VTokenClientAddress(veurc_symbol.clone()));

        // env.storage()
        //     .persistent()
        //     .set(&TokenDataKey::NativeEURCClientAddress, &native_token_address);
        // Self::extend_ttl_tokendatakey(&env, TokenDataKey::NativeEURCClientAddress);

        // env.storage().persistent().set(
        //     &PoolDataKey::PoolAddress(eurc_symbol.clone()),
        //     &eurc_pool_address,
        // );
        // Self::extend_ttl_pooldatakey(&env, PoolDataKey::PoolAddress(eurc_symbol.clone()));

        env.storage()
            .persistent()
            .set(&TokenDataKey::TokenIssuerAddress, &admin.clone());
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::TokenIssuerAddress);

        env.storage().persistent().set(
            &PoolDataKey::Pool(eurc_symbol.clone()),
            &U256::from_u128(&env, 0),
        ); // Store the EURC this contract handles
        Self::extend_ttl_pooldatakey(&env, PoolDataKey::Pool(eurc_symbol.clone()));
        Ok(String::from_str(&env, "EURC pool initialised"))
    }

    pub fn deposit_eurc(env: Env, lender: Address, amount: U256) {
        lender.require_auth();
        if amount <= U256::from_u128(&env, 0) {
            panic!("Deposit amount must be positive");
        }
        // Check if pool is initialised
        Self::is_eurc_pool_initialised(&env, Symbol::new(&env, "EURC"));

        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        // Getting the amount of tokens to be minted for Asset deposited
        let vtokens_to_be_minted = Self::convert_eurc_to_vtoken(&env, amount.clone());

        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);

        let user_balance = eurc_token.balance(&lender) as u128;

        if user_balance < amount_u128 {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }

        // Transfer EURC from user to this contract
        eurc_token.transfer(
            &lender,                         // from
            &env.current_contract_address(), // to
            &(amount_u128 as i128),
        );

        // Update lender list
        Self::add_lender_to_list_eurc(&env, &lender);

        let key = PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "EURC"));

        // Adding amount to Lenders balance, first check current balance, if no balance start with 0
        let current_balance: U256 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(U256::from_u128(&env, 0)); // Use U256::from_u128 or U256::zero to initialize U256

        let new_balance = current_balance.add(&amount);

        env.storage().persistent().set(&key, &new_balance);
        Self::extend_ttl_pooldatakey(&env, key);

        // Adding same amount to Total Pool balance
        let current_pool: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "EURC")))
            .unwrap_or(U256::from_u128(&env, 0));

        let new_pool = current_pool.add(&amount);

        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(Symbol::new(&env, "EURC")), &(new_pool));
        Self::extend_ttl_pooldatakey(&env, PoolDataKey::Pool(Symbol::new(&env, "EURC")));

        let token_value: U256 = env
            .storage()
            .persistent()
            .get(&TokenDataKey::VTokenValue(Symbol::new(&env, "VEURC")))
            .unwrap();

        // Now Mint the VEURC tokens that were created for the lender
        Self::mint_veurc_tokens(&env, lender.clone(), vtokens_to_be_minted, token_value);

        env.events().publish(
            (Symbol::new(&env, "deposit_event"), lender.clone()),
            LendingDepositEvent {
                lender: lender.clone(),
                amount: amount,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "EURC"),
            },
        );
    }

    pub fn redeem_veurc(env: &Env, lender: Address, tokens_to_redeem: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_eurc_pool_initialised(&env, Symbol::new(&env, "EURC"));
        let key = PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "EURC"));

        // Check if lender has registered
        if !env.storage().persistent().has(&key) {
            panic!("Lender not registered");
        }

        let key_k = TokenDataKey::VTokenBalance(lender.clone(), Symbol::new(&env, "VEURC"));
        let veurc_balance = env
            .storage()
            .persistent()
            .get(&key_k)
            .unwrap_or_else(|| U256::from_u32(&env, 0));

        if tokens_to_redeem > veurc_balance {
            panic!("Insufficient Token Balance to redeem");
        }

        let eurc_value = Self::convert_vtoken_to_asset(env, tokens_to_redeem.clone());

        // Check if lender has enough balance to deduct
        let current_balance: U256 = env.storage().persistent().get(&key).unwrap();

        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);

        let amount_u128: u128 = eurc_value
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        eurc_token.transfer(
            &env.current_contract_address(), // from
            &lender,                         // to
            &(amount_u128 as i128),
        );

        // First deduct amount from Lenders balance
        let new_balance = current_balance.sub(&eurc_value);
        env.storage().persistent().set(&key, &new_balance);
        Self::extend_ttl_pooldatakey(&env, key);

        let pool_key = PoolDataKey::Pool(Symbol::new(&env, "EURC"));
        // Deduct same amount from total pool balance
        let current_pool_balance: U256 = env
            .storage()
            .persistent()
            .get(&pool_key)
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::PoolNotInitialized));
        if current_pool_balance < eurc_value {
            panic_with_error!(&env, LendingError::InsufficientPoolBalance);
        }

        env.storage()
            .persistent()
            .set(&pool_key, &(current_pool_balance.sub(&eurc_value)));
        Self::extend_ttl_pooldatakey(&env, pool_key);

        let token_value: U256 = env
            .storage()
            .persistent()
            .get(&TokenDataKey::VTokenValue(Symbol::new(&env, "VEURC")))
            .unwrap();

        // Making sure token_value is not zero before dividing
        if token_value == U256::from_u128(&env, 0) {
            panic_with_error!(&env, LendingTokenError::InvalidVTokenValue);
        }

        Self::burn_veurc_tokens(&env, lender.clone(), tokens_to_redeem.clone(), token_value);

        // emit event after withdraw
        env.events().publish(
            (Symbol::new(&env, "withdraw_event"), lender.clone()),
            LendingWithdrawEvent {
                lender: lender,
                vtoken_amount: tokens_to_redeem,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "EURC"),
            },
        );
    }

    pub fn lend_to(
        env: &Env,
        trader_smart_account: Address,
        amount: U256,
    ) -> Result<bool, LendingError> {
        let account_manager: Address = env
            .storage()
            .persistent()
            .get(&ContractDetails::AccountManager)
            .expect("Account manager contract address not set !");
        account_manager.require_auth();

        Self::update_state(env);
        let borrow_shares = Self::convert_asset_borrow_shares(env, amount.clone());
        let mut is_first_borrow: bool = false;

        let key_a = PoolDataKey::UserBorrowShares(trader_smart_account.clone());
        let key_b = PoolDataKey::TotalBorrowShares;
        let key_c = PoolDataKey::Borrows;
        let user_borrow_shares: U256 = env.storage().persistent().get(&key_a).unwrap();
        let total_borrow_shares: U256 = env.storage().persistent().get(&key_b).unwrap();
        let borrows: U256 = env.storage().persistent().get(&key_c).unwrap();

        if user_borrow_shares == U256::from_u32(&env, 0) {
            is_first_borrow = true;
        }

        let res1 = user_borrow_shares.add(&borrow_shares.clone());
        let res2 = total_borrow_shares.add(&borrow_shares.clone());
        let res3 = borrows.add(&amount.clone());

        env.storage().persistent().set(&key_a, &res1);
        env.storage().persistent().set(&key_b, &res2);
        env.storage().persistent().set(&key_c, &res3);
        Self::extend_ttl_pooldatakey(env, key_a);
        Self::extend_ttl_pooldatakey(env, key_b);
        Self::extend_ttl_pooldatakey(env, key_c);

        // Now transfer amount to trader's smart account address
        let native_token_address: Address = Self::get_native_eurc_client_address(&env);
        let eurc_token = token::Client::new(&env, &native_token_address);
        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));
        eurc_token.transfer(
            &env.current_contract_address(), // from
            &trader_smart_account,           // to
            &(amount_u128 as i128),
        );

        let smart_account_client = smart_account_contract::Client::new(&env, &trader_smart_account);
        smart_account_client.add_borrowed_token(&Symbol::new(&env, "EURC"));
        smart_account_client.set_has_debt(&true);

        Ok(is_first_borrow)
    }

    pub fn collect_from(
        env: &Env,
        amount: U256,
        trader_smart_account: Address,
    ) -> Result<bool, LendingError> {
        let account_manager: Address = env
            .storage()
            .persistent()
            .get(&ContractDetails::AccountManager)
            .expect("Account manager contract address not set !");
        account_manager.require_auth();
        Self::update_state(env);

        let borrow_shares = Self::convert_asset_borrow_shares(env, amount.clone());
        if borrow_shares == U256::from_u32(&env, 0) {
            panic!("Zero borrow shares");
        }

        let user_borrow_shares: U256 =
            Self::get_user_borrow_shares(env, trader_smart_account.clone());
        let total_borrow_shares: U256 = Self::get_total_borrow_shares(env);
        let key_c = PoolDataKey::Borrows;
        let borrows: U256 = env.storage().persistent().get(&key_c).unwrap();

        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let smart_account_client = smart_account_contract::Client::new(&env, &trader_smart_account);
        smart_account_client
            .remove_borrowed_token_balance(&Symbol::new(&env, "EURC"), &amount_u128);

        let res1 = user_borrow_shares.sub(&borrow_shares);
        let res2 = total_borrow_shares.sub(&borrow_shares);
        let res3 = borrows.sub(&amount);

        if res1 == U256::from_u32(&env, 0) {
            smart_account_client.remove_borrowed_token(&Symbol::new(&env, "EURC"));
        }

        Self::set_user_borrow_shares(env, trader_smart_account.clone(), res1.clone());
        Self::set_total_borrow_shares(env, res2);
        env.storage().persistent().set(&key_c, &res3);
        return Ok(res1 == U256::from_u32(&env, 0));
    }

    fn mint_veurc_tokens(env: &Env, lender: Address, tokens_to_mint: U256, token_value: U256) {
        let key = TokenDataKey::VTokenBalance(lender.clone(), Symbol::new(&env, "VEURC"));

        // Check if user has balance initialised, else initialise key for user
        if !env.storage().persistent().has(&key) {
            env.storage()
                .persistent()
                .set(&key, &U256::from_u128(&env, 0));
            Self::extend_ttl_tokendatakey(&env, key.clone());
        }

        let tokens_to_mint_u128: u128 = tokens_to_mint
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));
        let veurc_symbol = Symbol::new(&env, "VEURC");

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::VTokenClientAddress(veurc_symbol))
            .unwrap();

        let token_sac = token::StellarAssetClient::new(&env, &token_address);

        // mint tokens to his address.
        token_sac.mint(&lender, &(tokens_to_mint_u128 as i128)); // Mint tokens to recipient

        let current_veurc_balance: U256 = env.storage().persistent().get(&key).unwrap();
        let new_veurc_balance = current_veurc_balance.add(&tokens_to_mint);
        env.storage().persistent().set(&key, &new_veurc_balance);
        Self::extend_ttl_tokendatakey(&env, key.clone());

        // Update total token balance available right now
        let current_total_token_balance = Self::get_current_total_veurc_balance(env);
        let new_total_token_balance = current_total_token_balance.add(&tokens_to_mint);
        let key_x = TokenDataKey::CurrentVTokenBalance(Symbol::new(&env, "VEURC"));
        env.storage()
            .persistent()
            .set(&key_x, &new_total_token_balance);
        Self::extend_ttl_tokendatakey(&env, key_x);

        let total_minted = Self::get_total_veurc_minted(env);
        let new_total_minted = total_minted.add(&tokens_to_mint);
        let key_y = TokenDataKey::TotalTokensMinted(Symbol::new(&env, "VEURC"));
        env.storage().persistent().set(&key_y, &new_total_minted);
        Self::extend_ttl_tokendatakey(&env, key_y);

        env.events().publish(
            (Symbol::new(&env, "mint_event"), lender.clone()),
            LendingTokenMintEvent {
                lender: lender.clone(),
                token_amount: tokens_to_mint,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "VEURC"),
                token_value: token_value,
            },
        );
    }

    fn burn_veurc_tokens(env: &Env, lender: Address, tokens_to_burn: U256, token_value: U256) {
        let key = TokenDataKey::VTokenBalance(lender.clone(), Symbol::new(&env, "VEURC"));
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, LendingTokenError::TokenBalanceNotInitialised);
        }

        let current_veurc_balance: U256 = env.storage().persistent().get(&key).unwrap();
        // Check if user has enough tokens to burn
        if current_veurc_balance < tokens_to_burn {
            panic_with_error!(&env, LendingTokenError::InsufficientTokenBalance);
        }

        let new_veurc_balance = current_veurc_balance.sub(&tokens_to_burn);
        env.storage().persistent().set(&key, &new_veurc_balance);
        Self::extend_ttl_tokendatakey(&env, key);

        let tokens_to_burn_u128: u128 = tokens_to_burn
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));
        let veurc_symbol = Symbol::new(&env, "VEURC");

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::VTokenClientAddress(veurc_symbol))
            .unwrap();

        let token_sac = token::TokenClient::new(&env, &token_address);

        // burn tokens from his address.
        token_sac.burn(&lender, &(tokens_to_burn_u128 as i128));

        let current_total_token_balance = Self::get_current_total_veurc_balance(env);
        let new_total_token_balance = current_total_token_balance.sub(&tokens_to_burn);
        env.storage().persistent().set(
            &TokenDataKey::CurrentVTokenBalance(Symbol::new(&env, "VEURC")),
            &new_total_token_balance,
        );
        Self::extend_ttl_tokendatakey(
            &env,
            TokenDataKey::CurrentVTokenBalance(Symbol::new(&env, "VEURC")),
        );

        let total_burnt = Self::get_total_veurc_burnt(env);
        let new_total_burnt = total_burnt.add(&tokens_to_burn);
        let key_a = TokenDataKey::TotalTokensBurnt(Symbol::new(&env, "VEURC"));
        env.storage().persistent().set(&key_a, &new_total_burnt);
        Self::extend_ttl_tokendatakey(&env, key_a);

        env.events().publish(
            (Symbol::new(&env, "burn_event"), lender.clone()),
            LendingTokenBurnEvent {
                lender: lender.clone(),
                token_amount: tokens_to_burn,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "VEURC"),
                token_value,
            },
        );
    }

    pub fn get_eurc_pool_balance(env: &Env) -> U256 {
        Self::is_eurc_pool_initialised(&env, Symbol::new(&env, "EURC"));

        env.storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "EURC")))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    pub fn convert_asset_borrow_shares(env: &Env, amount: U256) -> U256 {
        let key_b = PoolDataKey::TotalBorrowShares;
        let total_borrow_shares: U256 = env.storage().persistent().get(&key_b).unwrap();

        if total_borrow_shares == U256::from_u32(&env, 0) {
            return amount;
        } else {
            let res = amount.mul(&total_borrow_shares);
            let result = res.div(&Self::get_borrows(&env));
            return result;
        }
    }

    pub fn convert_borrow_shares_asset(env: &Env, debt: U256) -> U256 {
        let key_b = PoolDataKey::TotalBorrowShares;
        let total_borrow_shares: U256 = env.storage().persistent().get(&key_b).unwrap();
        if total_borrow_shares == U256::from_u32(&env, 0) {
            return debt;
        } else {
            let res = debt.mul(&Self::get_borrows(&env));
            let result = res.div(&total_borrow_shares);
            return result;
        }
    }

    pub fn update_state(env: &Env) {
        let lastupdatetime = Self::get_last_updated_time(&env);
        if lastupdatetime == env.ledger().timestamp() {
            return;
        }
        let key_c = PoolDataKey::Borrows;
        let borrows: U256 = env.storage().persistent().get(&key_c).unwrap();
        let rate_factor = Self::get_rate_factor(&env).unwrap();
        let interest_accrued = borrows.mul(&rate_factor);
        let res = borrows.add(&interest_accrued);
        env.storage().persistent().set(&key_c, &res);
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
        let key_a = PoolDataKey::UserBorrowShares(trader.clone());
        let user_borrow_shares: U256 = env.storage().persistent().get(&key_a).unwrap();
        return user_borrow_shares;
    }

    pub fn set_user_borrow_shares(env: &Env, trader: Address, res: U256) {
        let key_a = PoolDataKey::UserBorrowShares(trader.clone());
        env.storage().persistent().set(&key_a, &res);
        Self::extend_ttl_pooldatakey(env, key_a);
    }
    pub fn get_borrow_balance(env: &Env, trader: Address) -> U256 {
        let key_a = PoolDataKey::UserBorrowShares(trader.clone());
        let user_borrow_shares: U256 = env.storage().persistent().get(&key_a).unwrap();
        let res = Self::convert_borrow_shares_asset(env, user_borrow_shares);
        res
    }

    pub fn get_total_borrow_shares(env: &Env) -> U256 {
        let key_b = PoolDataKey::TotalBorrowShares;
        let total_borrow_shares: U256 = env.storage().persistent().get(&key_b).unwrap();
        return total_borrow_shares;
    }

    pub fn set_total_borrow_shares(env: &Env, res: U256) {
        let key_b = PoolDataKey::TotalBorrowShares;
        env.storage().persistent().set(&key_b, &res);
        Self::extend_ttl_pooldatakey(env, key_b);
    }

    pub fn total_assets(env: &Env) -> U256 {
        let token = Symbol::new(&env, "EURC");
        let assets = Self::get_total_liquidity_in_pool(&env, token.clone());
        let borrows = Self::get_borrows(env);
        let total_assets = assets.add(&borrows);
        total_assets
    }

    pub fn get_borrows(env: &Env) -> U256 {
        let key_c = PoolDataKey::Borrows;

        let borrows: U256 = env.storage().persistent().get(&key_c).unwrap();
        let res = borrows.mul(&Self::get_rate_factor(&env).unwrap());
        let result = borrows.add(&res);
        result
    }

    pub fn get_rate_factor(env: &Env) -> Result<U256, InterestRateError> {
        let registy_address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registy_address);
        let rate_model_address = registry_client.get_rate_model_address();
        let rate_model_client = rate_model_contract::Client::new(&env, &rate_model_address);

        let lastupdatetime = Self::get_last_updated_time(&env);
        let blocktimestamp = env.ledger().timestamp();
        if lastupdatetime == blocktimestamp {
            return Ok(U256::from_u32(&env, 0));
        }
        let token = Symbol::new(&env, "EURC");

        let key_c = PoolDataKey::Borrows;

        let borrows: U256 = env.storage().persistent().get(&key_c).unwrap();
        let liquidity = Self::get_total_liquidity_in_pool(&env, token.clone());

        let res = U256::from_u128(&env, (blocktimestamp - lastupdatetime) as u128)
            .mul(&(rate_model_client.get_borrow_rate_per_sec(&liquidity, &borrows)));

        Ok(res)
    }

    pub fn get_total_liquidity_in_pool(env: &Env, token_symbol: Symbol) -> U256 {
        env.storage()
            .persistent()
            .get(&PoolDataKey::Pool(token_symbol))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    pub fn get_last_updated_time(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&PoolDataKey::LastUpdatedTime)
            .unwrap_or_else(|| env.ledger().timestamp())
    }

    pub fn get_current_total_veurc_balance(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::CurrentVTokenBalance(Symbol::new(
                &env, "VEURC",
            )))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_veurc_minted(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensMinted(Symbol::new(&env, "VEURC")))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_veurc_burnt(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensBurnt(Symbol::new(&env, "VEURC")))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    // Helper function to add lender to list
    fn add_lender_to_list_eurc(env: &Env, lender: &Address) {
        let mut lenders: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "EURC")))
            .unwrap_or_else(|| Vec::new(&env));

        if !lenders.contains(lender) {
            lenders.push_back(lender.clone());
            let key_b = PoolDataKey::Lenders(Symbol::new(&env, "EURC"));
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

    pub fn is_eurc_pool_initialised(env: &Env, asset: Symbol) -> bool {
        if !env.storage().persistent().has(&PoolDataKey::Pool(asset)) {
            // panic!("Pool not initialised");
            panic_with_error!(&env, LendingError::PoolNotInitialized);
        }
        true
    }

    pub fn get_eurc_pool_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&PoolDataKey::PoolAddress(Symbol::new(&env, "EURC").clone()))
            .unwrap_or_else(|| panic!("EURC pool address not set"))
    }

    pub fn get_native_eurc_client_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&TokenDataKey::EurcClientAddress)
            .unwrap_or_else(|| panic!("Native EURC client address not set"))
    }

    // Converts EURC to VEURC
    pub fn convert_eurc_to_vtoken(env: &Env, amount: U256) -> U256 {
        let pool_balance = Self::get_eurc_pool_balance(env);
        let minted = Self::get_total_veurc_minted(env);

        if pool_balance == U256::from_u128(&env, 0) || minted == U256::from_u128(&env, 0) {
            amount
        } else {
            let supply = Self::get_current_total_veurc_balance(env);
            let total_liquidity_pool =
                Self::get_total_liquidity_in_pool(env, Symbol::new(&env, "EURC"));

            let res = amount.mul(&supply);
            let resx = res.div(&total_liquidity_pool);

            let vtoken_value = amount.div(&resx);
            env.storage().persistent().set(
                &TokenDataKey::VTokenValue(Symbol::new(&env, "VEURC")),
                &vtoken_value,
            );
            Self::extend_ttl_tokendatakey(
                &env,
                TokenDataKey::VTokenValue(Symbol::new(&env, "VEURC")),
            );
            resx
        }
    }

    //  Converting VEURC to EURC
    pub fn convert_vtoken_to_asset(env: &Env, vtokens_to_be_burnt: U256) -> U256 {
        let pool_balance = Self::get_eurc_pool_balance(env);
        let v_token_supply = Self::get_current_total_veurc_balance(env);
        let res = vtokens_to_be_burnt.mul(&pool_balance);
        let resx = res.div(&v_token_supply);

        let vtoken_value = resx.div(&vtokens_to_be_burnt);
        env.storage().persistent().set(
            &TokenDataKey::VTokenValue(Symbol::new(&env, "VEURC")),
            &vtoken_value,
        );
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::VTokenValue(Symbol::new(&env, "VEURC")));

        resx
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
