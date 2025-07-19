use core::panic;

use crate::errors::{LendingError, LendingTokenError};
use crate::events::{
    LendingDepositEvent, LendingTokenBurnEvent, LendingTokenMintEvent, LendingWithdrawEvent,
};
use crate::types::{DataKey, PoolDataKey, TokenDataKey};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Env, Symbol, Vec, U256,
};

#[contract]
pub struct LiquidityPoolUSDC;

// pub const USDC_CONTRACT_ID: [u8; 32] = [0; 32];
const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_2YEAR: u32 = 6307200 * 2;
const _TLL_LEDGERS_MONTH: u32 = 518400;

#[contractimpl]
impl LiquidityPoolUSDC {
    pub fn initialize_pool_usdc(
        env: Env,
        native_token_address: Address,
        vusdc_token_address: Address,
    ) {
        // Verify contract is deployed
        if !env.storage().persistent().has(&PoolDataKey::Deployed) {
            panic!("Contract not deployed");
        }
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("Admin not set");

        admin.require_auth();

        env.storage()
            .persistent()
            .set(&TokenDataKey::TokenClientAddress, &vusdc_token_address);
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::TokenClientAddress);

        env.storage().persistent().set(
            &TokenDataKey::NativeTokenClientAddress,
            &native_token_address,
        );
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::NativeTokenClientAddress);

        env.storage()
            .persistent()
            .set(&TokenDataKey::TokenIssuerAddress, &admin.clone());
        Self::extend_ttl_tokendatakey(&env, TokenDataKey::TokenIssuerAddress);

        env.storage().persistent().set(
            &PoolDataKey::Pool(Symbol::new(&env, "USDC")),
            &U256::from_u128(&env, 0),
        ); // Store the USDC this contract handles
        Self::extend_ttl_pooldatakey(&env, PoolDataKey::Pool(Symbol::new(&env, "USDC")));
    }

    pub fn deposit_usdc(env: Env, lender: Address, amount: U256) {
        lender.require_auth();
        if amount <= U256::from_u128(&env, 0) {
            panic!("Deposit amount must be positive");
        }
        // Check if pool is initialised
        Self::is_usdc_pool_initialised(&env, Symbol::new(&env, "USDC"));

        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();

        let native_token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::NativeTokenClientAddress)
            .unwrap();
        let usdc_token = token::Client::new(&env, &native_token_address);

        // let usdc_token: token::TokenClient<'_> = Self::get_usdc_token_client(&env, admin);

        let user_balance = usdc_token.balance(&lender) as u128;

        if user_balance < amount_u128 {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }

        // Transfer USDC from user to this contract
        usdc_token.transfer(
            &lender,                         // from
            &env.current_contract_address(), // to
            &(amount_u128 as i128),
        );

        // Update lender list
        Self::add_lender_to_list_usdc(&env, &lender);

        let key = PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "USDC"));

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
            .get(&PoolDataKey::Pool(Symbol::new(&env, "USDC")))
            .unwrap_or(U256::from_u128(&env, 0));

        let new_pool = current_pool.add(&amount);

        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(Symbol::new(&env, "USDC")), &(new_pool));
        Self::extend_ttl_pooldatakey(&env, PoolDataKey::Pool(Symbol::new(&env, "USDC")));

        let token_value: U256 = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenValue(Symbol::new(&env, "vUSDC")))
            .unwrap();

        // Making sure token_value is not zero before dividing
        if token_value == U256::from_u128(&env, 0) {
            panic!("InvalidTokenValue");
            // panic_with_error!(&env, LendingTokenError::InvalidTokenValue);
        }

        let tokens_to_be_minted = amount.div(&token_value);

        // Now Mint the vUSDC tokens that were created for the lender
        Self::mint_vusdc_tokens(&env, lender.clone(), tokens_to_be_minted, token_value);

        env.events().publish(
            (Symbol::new(&env, "deposit_event"), lender.clone()),
            LendingDepositEvent {
                lender: lender.clone(),
                amount: amount,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "USDC"),
            },
        );
    }

    pub fn withdraw_usdc(env: &Env, lender: Address, amount: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_usdc_pool_initialised(&env, Symbol::new(&env, "USDC"));
        let key = PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "USDC"));

        // Check if lender has registered
        if !env.storage().persistent().has(&key) {
            panic!("Lender not registered");
            // panic_with_error!(&env, LendingError::LenderNotRegistered);
        }

        // Check if lender has enough balance to deduct
        let current_balance: U256 = env.storage().persistent().get(&key).unwrap();

        if current_balance < amount {
            panic!("InsufficientBalance");
            // panic_with_error!(&env, LendingError::InsufficientBalance);
        }
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();

        let native_token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::NativeTokenClientAddress)
            .unwrap();
        let usdc_token = token::Client::new(&env, &native_token_address);

        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        usdc_token.transfer(
            &env.current_contract_address(), // from
            &lender,                         // to
            &(amount_u128 as i128),
        );

        // First deduct amount from Lenders balance
        let new_balance = current_balance.sub(&amount);
        env.storage().persistent().set(&key, &new_balance);
        Self::extend_ttl_pooldatakey(&env, key);

        let pool_key = PoolDataKey::Pool(Symbol::new(&env, "USDC"));
        // Deduct same amount from total pool balance
        let current_pool_balance: U256 = env
            .storage()
            .persistent()
            .get(&pool_key)
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::PoolNotInitialized));
        if current_pool_balance < amount {
            panic_with_error!(&env, LendingError::InsufficientPoolBalance);
        }

        env.storage()
            .persistent()
            .set(&pool_key, &(current_pool_balance.sub(&amount)));
        Self::extend_ttl_pooldatakey(&env, pool_key);

        // Now burn the vUSDC tokens that were created for the lender
        // Get token value per unit vusdc
        // When lender wants to withdraw amount of usdc, we need to calculate how many vusdc tokens to burn
        // This is done by dividing the amount of usdc by the token value per unit vusdc
        // token_value is latest value of each vUSDC
        let token_value: U256 = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenValue(Symbol::new(&env, "vUSDC")))
            .unwrap();

        // Making sure token_value is not zero before dividing
        if token_value == U256::from_u128(&env, 0) {
            panic_with_error!(&env, LendingTokenError::InvalidTokenValue);
        }

        let tokens_to_be_burnt = amount.div(&token_value);

        Self::burn_vusdc_tokens(&env, lender.clone(), tokens_to_be_burnt, token_value);

        // emit event after withdraw
        env.events().publish(
            (Symbol::new(&env, "withdraw_event"), lender.clone()),
            LendingWithdrawEvent {
                lender: lender,
                amount: amount,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "USDC"),
            },
        );
    }

    fn mint_vusdc_tokens(env: &Env, lender: Address, tokens_to_mint: U256, token_value: U256) {
        let key = TokenDataKey::TokenBalance(lender.clone(), Symbol::new(&env, "vUSDC"));

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

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenClientAddress)
            .unwrap();

        let token_sac = token::StellarAssetClient::new(&env, &token_address);

        let issuer: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenIssuerAddress)
            .unwrap();
        issuer.require_auth();
        // mint tokens to his address.
        token_sac.mint(&lender, &(tokens_to_mint_u128 as i128)); // Mint tokens to recipient

        let current_vusdc_balance: U256 = env.storage().persistent().get(&key).unwrap();
        let new_vusdc_balance = current_vusdc_balance.add(&tokens_to_mint);
        env.storage().persistent().set(&key, &new_vusdc_balance);
        Self::extend_ttl_tokendatakey(&env, key.clone());

        // Update total token balance available right now
        let current_total_token_balance = Self::get_current_total_vusdc_balance(env);
        let new_total_token_balance = current_total_token_balance.add(&tokens_to_mint);
        let key_x = TokenDataKey::CurrentTokenBalance(Symbol::new(&env, "vUSDC"));
        env.storage()
            .persistent()
            .set(&key_x, &new_total_token_balance);
        Self::extend_ttl_tokendatakey(&env, key_x);

        let total_minted = Self::get_total_vusdc_minted(env);
        let new_total_minted = total_minted.add(&tokens_to_mint);
        let key_y = TokenDataKey::TotalTokensMinted(Symbol::new(&env, "vUSDC"));
        env.storage().persistent().set(&key_y, &new_total_minted);
        Self::extend_ttl_tokendatakey(&env, key_y);

        env.events().publish(
            (Symbol::new(&env, "mint_event"), lender.clone()),
            LendingTokenMintEvent {
                lender: lender.clone(),
                token_amount: tokens_to_mint,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "vUSDC"),
                token_value: token_value,
            },
        );
    }

    fn burn_vusdc_tokens(env: &Env, lender: Address, tokens_to_burn: U256, token_value: U256) {
        let key = TokenDataKey::TokenBalance(lender.clone(), Symbol::new(&env, "vUSDC"));
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, LendingTokenError::TokenBalanceNotInitialised);
        }

        let current_vusdc_balance: U256 = env.storage().persistent().get(&key).unwrap();
        // Check if user has enough tokens to burn
        if current_vusdc_balance < tokens_to_burn {
            panic_with_error!(&env, LendingTokenError::InsufficientTokenBalance);
        }

        let new_vusdc_balance = current_vusdc_balance.sub(&tokens_to_burn);
        env.storage().persistent().set(&key, &new_vusdc_balance);
        Self::extend_ttl_tokendatakey(&env, key);

        let tokens_to_burn_u128: u128 = tokens_to_burn
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenClientAddress)
            .unwrap();

        let token_sac = token::TokenClient::new(&env, &token_address);

        let issuer: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenIssuerAddress)
            .unwrap();
        issuer.require_auth();
        // burn tokens from his address.
        token_sac.transfer(
            &lender,
            &env.current_contract_address(),
            &(tokens_to_burn_u128 as i128),
        );

        let current_total_token_balance = Self::get_current_total_vusdc_balance(env);
        let new_total_token_balance = current_total_token_balance.sub(&tokens_to_burn);
        env.storage().persistent().set(
            &TokenDataKey::CurrentTokenBalance(Symbol::new(&env, "vUSDC")),
            &new_total_token_balance,
        );
        Self::extend_ttl_tokendatakey(
            &env,
            TokenDataKey::CurrentTokenBalance(Symbol::new(&env, "vUSDC")),
        );

        let total_burnt = Self::get_total_vusdc_burnt(env);
        let new_total_burnt = total_burnt.add(&tokens_to_burn);
        let key_a = TokenDataKey::TotalTokensBurnt(Symbol::new(&env, "vUSDC"));
        env.storage().persistent().set(&key_a, &new_total_burnt);
        Self::extend_ttl_tokendatakey(&env, key_a);

        env.events().publish(
            (Symbol::new(&env, "burn_event"), lender.clone()),
            LendingTokenBurnEvent {
                lender: lender.clone(),
                token_amount: tokens_to_burn,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "vUSDC"),
                token_value,
            },
        );
    }

    pub fn get_usdc_pool_balance(env: Env) -> U256 {
        Self::is_usdc_pool_initialised(&env, Symbol::new(&env, "USDC"));

        env.storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "USDC")))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    pub fn get_current_total_vusdc_balance(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::CurrentTokenBalance(Symbol::new(
                &env, "vUSDC",
            )))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_vusdc_minted(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensMinted(Symbol::new(&env, "vUSDC")))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_vusdc_burnt(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensBurnt(Symbol::new(&env, "vUSDC")))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    // Helper function to add lender to list
    fn add_lender_to_list_usdc(env: &Env, lender: &Address) {
        let mut lenders: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "USDC")))
            .unwrap_or_else(|| Vec::new(&env));

        if !lenders.contains(lender) {
            lenders.push_back(lender.clone());
            let key_b = PoolDataKey::Lenders(Symbol::new(&env, "USDC"));
            env.storage().persistent().set(&key_b, &lenders);
            Self::extend_ttl_pooldatakey(&env, key_b);
        }
    }

    // Function to get all lenders
    pub fn get_lenders_usdc(env: Env) -> Vec<Address> {
        let list_address: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "USDC")))
            .unwrap_or_else(|| Vec::new(&env));
        list_address
    }

    pub fn is_usdc_pool_initialised(env: &Env, asset: Symbol) -> bool {
        if !env.storage().persistent().has(&PoolDataKey::Pool(asset)) {
            // panic!("Pool not initialised");
            panic_with_error!(&env, LendingError::PoolNotInitialized);
        }
        true
    }

    fn extend_ttl_datakey(env: &Env, key: DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_2YEAR);
    }

    fn extend_ttl_pooldatakey(env: &Env, key: PoolDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_2YEAR);
    }

    fn extend_ttl_tokendatakey(env: &Env, key: TokenDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_2YEAR);
    }

    // #[cfg(test)]
    // fn get_usdc_token_client(env: &Env, admin: Address) -> token::Client {
    //     // Create a mock stellar asset contract that behaves like USDC
    //     // let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(env);
    //     let mock_usdc_id = env.register_stellar_asset_contract_v2(admin);
    //     token::Client::new(env, &mock_usdc_id.address())
    // }

    // #[cfg(not(test))]
    // fn get_usdc_token_client(env: &Env, admin: Address) -> token::Client {
    //     // In production, use the real USDC contract
    //     token::Client::new(
    //         env,
    //         &Address::from_string_bytes(&Bytes::from_array(env, &USDC_CONTRACT_ID)),
    //     )
    //     // &Address::from_string_bytes(&Bytes::from_array(env, &USDC_CONTRACT_ID)),
    // }
}
