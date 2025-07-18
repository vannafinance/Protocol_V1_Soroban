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
pub struct LiquidityPoolXLM;

pub const XLM_CONTRACT_ID: [u8; 32] = [0; 32];

#[contractimpl]
impl LiquidityPoolXLM {
    pub fn initialize_pool_xlm(
        env: Env,
        native_token_address: Address,
        vxlm_token_address: Address,
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
            .set(&TokenDataKey::TokenClientAddress, &vxlm_token_address);

        env.storage().persistent().set(
            &TokenDataKey::NativeTokenClientAddress,
            &native_token_address,
        );

        env.storage()
            .persistent()
            .set(&TokenDataKey::TokenIssuerAddress, &admin.clone());

        env.storage().persistent().set(
            &PoolDataKey::Pool(Symbol::new(&env, "XLM")),
            &U256::from_u128(&env, 0),
        ); // Store the XLM this contract handles
    }

    pub fn deposit_xlm(env: Env, lender: Address, amount: U256) {
        lender.require_auth();
        if amount <= U256::from_u128(&env, 0) {
            panic!("Deposit amount must be positive");
        }
        // Check if pool is initialised
        Self::is_xlm_pool_initialised(&env, Symbol::new(&env, "XLM"));

        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();

        let native_token_address: Address = env
            .storage()
            .persistent()
            .get(&TokenDataKey::NativeTokenClientAddress)
            .unwrap();
        let xlm_token = token::Client::new(&env, &native_token_address);

        // let xlm_token: token::TokenClient<'_> = Self::get_xlm_token_client(&env, admin);

        let user_balance = xlm_token.balance(&lender) as u128;

        if user_balance < amount_u128 {
            panic_with_error!(&env, LendingError::InsufficientBalance);
        }

        // Transfer XLM from user to this contract
        xlm_token.transfer(
            &lender,                         // from
            &env.current_contract_address(), // to
            &(amount_u128 as i128),
        );

        // Update lender list
        Self::add_lender_to_list_xlm(&env, &lender);

        let key = PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "XLM"));

        // Adding amount to Lenders balance, first check current balance, if no balance start with 0
        let current_balance: U256 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(U256::from_u128(&env, 0)); // Use U256::from_u128 or U256::zero to initialize U256

        let new_balance = current_balance.add(&amount);

        env.storage().persistent().set(&key, &new_balance);

        // Adding same amount to Total Pool balance
        let current_pool: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
            .unwrap_or(U256::from_u128(&env, 0));

        let new_pool = current_pool.add(&amount);

        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(Symbol::new(&env, "XLM")), &(new_pool));

        let token_value: U256 = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenValue(Symbol::new(&env, "vXLM")))
            .unwrap();

        // Making sure token_value is not zero before dividing
        if token_value == U256::from_u128(&env, 0) {
            panic!("InvalidTokenValue");
            // panic_with_error!(&env, LendingTokenError::InvalidTokenValue);
        }

        let tokens_to_be_minted = amount.div(&token_value);

        // Now Mint the vXLM tokens that were created for the lender
        Self::mint_vxlm_tokens(&env, lender.clone(), tokens_to_be_minted, token_value);

        env.events().publish(
            (Symbol::new(&env, "deposit_event"), lender.clone()),
            LendingDepositEvent {
                lender: lender.clone(),
                amount: amount,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "XLM"),
            },
        );
    }

    pub fn withdraw_xlm(env: &Env, lender: Address, amount: U256) {
        lender.require_auth();
        // Check if pool is initialised
        Self::is_xlm_pool_initialised(&env, Symbol::new(&env, "XLM"));
        let key = PoolDataKey::LenderBalance(lender.clone(), Symbol::new(&env, "XLM"));

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
        let xlm_token = token::Client::new(&env, &native_token_address);

        let amount_u128: u128 = amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        xlm_token.transfer(
            &env.current_contract_address(), // from
            &lender,                         // to
            &(amount_u128 as i128),
        );

        // First deduct amount from Lenders balance
        let new_balance = current_balance.sub(&amount);
        env.storage().persistent().set(&key, &new_balance);

        let pool_key = PoolDataKey::Pool(Symbol::new(&env, "XLM"));
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

        // Now burn the vXLM tokens that were created for the lender
        // Get token value per unit vxlm
        // When lender wants to withdraw amount of xlm, we need to calculate how many vxlm tokens to burn
        // This is done by dividing the amount of xlm by the token value per unit vxlm
        // token_value is latest value of each vXLM
        let token_value: U256 = env
            .storage()
            .persistent()
            .get(&TokenDataKey::TokenValue(Symbol::new(&env, "vXLM")))
            .unwrap();

        // Making sure token_value is not zero before dividing
        if token_value == U256::from_u128(&env, 0) {
            panic_with_error!(&env, LendingTokenError::InvalidTokenValue);
        }

        let tokens_to_be_burnt = amount.div(&token_value);

        Self::burn_vxlm_tokens(&env, lender.clone(), tokens_to_be_burnt, token_value);

        // emit event after withdraw
        env.events().publish(
            (Symbol::new(&env, "withdraw_event"), lender.clone()),
            LendingWithdrawEvent {
                lender: lender,
                amount: amount,
                timestamp: env.ledger().timestamp(),
                asset_symbol: Symbol::new(&env, "XLM"),
            },
        );
    }

    fn mint_vxlm_tokens(env: &Env, lender: Address, tokens_to_mint: U256, token_value: U256) {
        let key = TokenDataKey::TokenBalance(lender.clone(), Symbol::new(&env, "vXLM"));

        // Check if user has balance initialised, else initialise key for user
        if !env.storage().persistent().has(&key) {
            env.storage()
                .persistent()
                .set(&key, &U256::from_u128(&env, 0));
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

        let current_vxlm_balance: U256 = env.storage().persistent().get(&key).unwrap();
        let new_vxlm_balance = current_vxlm_balance.add(&tokens_to_mint);
        env.storage().persistent().set(&key, &new_vxlm_balance);

        // Update total token balance available right now
        let current_total_token_balance = Self::get_current_total_vxlm_balance(env);
        let new_total_token_balance = current_total_token_balance.add(&tokens_to_mint);
        env.storage().persistent().set(
            &TokenDataKey::CurrentTokenBalance(Symbol::new(&env, "vXLM")),
            &new_total_token_balance,
        );

        let total_minted = Self::get_total_vxlm_minted(env);
        let new_total_minted = total_minted.add(&tokens_to_mint);
        env.storage().persistent().set(
            &TokenDataKey::TotalTokensMinted(Symbol::new(&env, "vXLM")),
            &new_total_minted,
        );

        env.events().publish(
            (Symbol::new(&env, "mint_event"), lender.clone()),
            LendingTokenMintEvent {
                lender: lender.clone(),
                token_amount: tokens_to_mint,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "vXLM"),
                token_value: token_value,
            },
        );
    }

    fn burn_vxlm_tokens(env: &Env, lender: Address, tokens_to_burn: U256, token_value: U256) {
        let key = TokenDataKey::TokenBalance(lender.clone(), Symbol::new(&env, "vXLM"));
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, LendingTokenError::TokenBalanceNotInitialised);
        }

        let current_vxlm_balance: U256 = env.storage().persistent().get(&key).unwrap();
        // Check if user has enough tokens to burn
        if current_vxlm_balance < tokens_to_burn {
            panic_with_error!(&env, LendingTokenError::InsufficientTokenBalance);
        }

        let new_vxlm_balance = current_vxlm_balance.sub(&tokens_to_burn);
        env.storage().persistent().set(&key, &new_vxlm_balance);

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

        let current_total_token_balance = Self::get_current_total_vxlm_balance(env);
        let new_total_token_balance = current_total_token_balance.sub(&tokens_to_burn);
        env.storage().persistent().set(
            &TokenDataKey::CurrentTokenBalance(Symbol::new(&env, "vXLM")),
            &new_total_token_balance,
        );

        let total_burnt = Self::get_total_vxlm_burnt(env);
        let new_total_burnt = total_burnt.add(&tokens_to_burn);
        env.storage().persistent().set(
            &TokenDataKey::TotalTokensBurnt(Symbol::new(&env, "vXLM")),
            &new_total_burnt,
        );

        env.events().publish(
            (Symbol::new(&env, "burn_event"), lender.clone()),
            LendingTokenBurnEvent {
                lender: lender.clone(),
                token_amount: tokens_to_burn,
                timestamp: env.ledger().timestamp(),
                token_symbol: Symbol::new(&env, "vXLM"),
                token_value,
            },
        );
    }

    pub fn get_xlm_pool_balance(env: Env) -> U256 {
        Self::is_xlm_pool_initialised(&env, Symbol::new(&env, "XLM"));

        env.storage()
            .persistent()
            .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    pub fn get_current_total_vxlm_balance(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::CurrentTokenBalance(Symbol::new(
                &env, "vXLM",
            )))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_vxlm_minted(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensMinted(Symbol::new(&env, "vXLM")))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    pub fn get_total_vxlm_burnt(env: &Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TokenDataKey::TotalTokensBurnt(Symbol::new(&env, "vXLM")))
            .unwrap_or_else(|| U256::from_u128(&env, 0))
    }

    // Helper function to add lender to list
    fn add_lender_to_list_xlm(env: &Env, lender: &Address) {
        let mut lenders: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")))
            .unwrap_or_else(|| Vec::new(&env));

        if !lenders.contains(lender) {
            lenders.push_back(lender.clone());
            env.storage()
                .persistent()
                .set(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")), &lenders);
        }
    }

    // Function to get all lenders
    pub fn get_lenders_xlm(env: Env) -> Vec<Address> {
        let list_address: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")))
            .unwrap_or_else(|| Vec::new(&env));
        list_address
    }

    pub fn is_xlm_pool_initialised(env: &Env, asset: Symbol) -> bool {
        if !env.storage().persistent().has(&PoolDataKey::Pool(asset)) {
            // panic!("Pool not initialised");
            panic_with_error!(&env, LendingError::PoolNotInitialized);
        }
        true
    }

    // #[cfg(test)]
    // fn get_xlm_token_client(env: &Env, admin: Address) -> token::Client {
    //     // Create a mock stellar asset contract that behaves like XLM
    //     // let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(env);
    //     let mock_xlm_id = env.register_stellar_asset_contract_v2(admin);
    //     token::Client::new(env, &mock_xlm_id.address())
    // }

    // #[cfg(not(test))]
    // fn get_xlm_token_client(env: &Env, admin: Address) -> token::Client {
    //     // In production, use the real XLM contract
    //     token::Client::new(
    //         env,
    //         &Address::from_string_bytes(&Bytes::from_array(env, &XLM_CONTRACT_ID)),
    //     )
    //     // &Address::from_string_bytes(&Bytes::from_array(env, &XLM_CONTRACT_ID)),
    // }
}
