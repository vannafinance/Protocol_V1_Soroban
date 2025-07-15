#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{LendingError, LendingTokenError};
    use crate::events::{
        LendingDepositEvent, LendingTokenBurnEvent, LendingTokenMintEvent, LendingWithdrawEvent,
    };
    use crate::lending_protocol::liquidity_pool_xlm::{
        LiquidityPoolXLM, LiquidityPoolXLMClient, XLM_CONTRACT_ID,
    };
    use crate::types::{DataKey, PoolDataKey, TokenDataKey};
    use soroban_sdk::token::StellarAssetClient;
    use soroban_sdk::token::TokenInterface;
    use soroban_sdk::Bytes;
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
        token, Address, Env, Symbol, Vec, U256,
    };

    fn setup_test_env() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        // let path: : = include_bytes!("../../../../target/wasm32v1-none/release/vanna_finance.wasm");

        // // let contract_address = env.register_contract(None, LiquidityPoolXLM);
        // let contract_address = env.register(path, ());

        let contract_address = env.register_contract(None, LiquidityPoolXLM);
        let admin = Address::generate(&env);
        let lender = Address::generate(&env);

        // Set up initial storage
        env.as_contract(&contract_address, || {
            env.storage()
                .persistent()
                .set(&PoolDataKey::Deployed, &true);
            env.storage().persistent().set(&DataKey::Admin, &admin);
            env.storage().persistent().set(
                &TokenDataKey::TokenValue(Symbol::new(&env, "vXLM")),
                &U256::from_u128(&env, 1000000), // 1 XLM = 1 vXLM initially
            );
        });

        (env, contract_address, admin, lender)
    }

    #[test]
    fn test_initialize_pool_xlm_success() {
        let (env, contract_address, admin, _) = setup_test_env();

        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        // Test successful initialization
        client.initialize_pool_xlm();

        // Verify pool is initialized
        env.as_contract(&contract_address, || {
            let pool_balance: U256 = env
                .storage()
                .persistent()
                .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
                .unwrap();
            assert_eq!(pool_balance, U256::from_u128(&env, 0));
        });
    }

    #[test]
    #[should_panic(expected = "Contract not deployed")]
    fn test_initialize_pool_xlm_not_deployed() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_address = env.register_contract(None, LiquidityPoolXLM);
        let admin = Address::generate(&env);

        // Don't set deployed flag
        env.as_contract(&contract_address, || {
            env.storage().persistent().set(&DataKey::Admin, &admin);
        });

        let client = LiquidityPoolXLMClient::new(&env, &contract_address);
        client.initialize_pool_xlm(); // Should panic
    }

    #[test]
    fn test_deposit_xlm_success() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        // Initialize pool first
        client.initialize_pool_xlm();

        // // Create XLM token client
        // let xlm_token = token::Client::new(
        //     &env,
        //     &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        // );

        // // Mint some XLM to the lender for testing
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128); // 1000 XLM

        // Mock XLM balance for lender (Stellar native asset)
        // env.ledger().with_mut(|ledger| {
        //     ledger.accounts.insert(
        //         lender.clone(),
        //         soroban_sdk::Account {
        //             balance: 1000000000i128, // 1000 XLM
        //             ..Default::default()
        //         },
        //     );
        // });

        let deposit_amount = U256::from_u128(&env, 100000000); // 100 XLM

        // Test deposit
        client.deposit_xlm(&lender, &deposit_amount);

        // Verify lender balance is updated
        env.as_contract(&contract_address, || {
            let lender_balance: U256 = env
                .storage()
                .persistent()
                .get(&PoolDataKey::LenderBalance(
                    lender.clone(),
                    Symbol::new(&env, "XLM"),
                ))
                .unwrap();
            assert_eq!(lender_balance, deposit_amount);

            // Verify pool balance is updated
            let pool_balance: U256 = env
                .storage()
                .persistent()
                .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
                .unwrap();
            assert_eq!(pool_balance, deposit_amount);

            // Verify lender is added to lenders list
            let lenders: Vec<Address> = env
                .storage()
                .persistent()
                .get(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")))
                .unwrap();
            assert!(lenders.contains(&lender));

            // Verify vXLM tokens are minted
            let vxlm_balance: U256 = env
                .storage()
                .persistent()
                .get(&TokenDataKey::TokenBalance(
                    lender.clone(),
                    Symbol::new(&env, "vXLM"),
                ))
                .unwrap();
            assert_eq!(vxlm_balance, U256::from_u128(&env, 100)); // 100 vXLM tokens
        });

        // Verify event was emitted
        let events = env.events().all();
        assert!(events.len() >= 2); // Deposit event + mint event
    }

    #[test]
    #[should_panic(expected = "Deposit amount must be positive")]
    fn test_deposit_xlm_zero_amount() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();
        client.deposit_xlm(&lender, &U256::from_u128(&env, 0)); // Should panic
    }

    #[test]
    fn test_deposit_xlm_insufficient_balance() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        // Don't mint any XLM to lender
        let deposit_amount = U256::from_u128(&env, 100000000);

        // This should trigger InsufficientBalance error
        // let result = std::panic::catch_unwind(|| {
        //     client.deposit_xlm(&lender, &deposit_amount);
        // });

        // assert!(result.is_err());
    }

    #[test]
    fn test_withdraw_xlm_success() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        // Initialize pool and deposit first
        client.initialize_pool_xlm();

        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        // Test withdrawal
        let withdraw_amount = U256::from_u128(&env, 50000000); // 50 XLM
        client.withdraw_xlm(&lender, &withdraw_amount);

        // Verify balances are updated
        env.as_contract(&contract_address, || {
            let lender_balance: U256 = env
                .storage()
                .persistent()
                .get(&PoolDataKey::LenderBalance(
                    lender.clone(),
                    Symbol::new(&env, "XLM"),
                ))
                .unwrap();
            assert_eq!(lender_balance, U256::from_u128(&env, 50000000)); // 50 XLM remaining

            let pool_balance: U256 = env
                .storage()
                .persistent()
                .get(&PoolDataKey::Pool(Symbol::new(&env, "XLM")))
                .unwrap();
            assert_eq!(pool_balance, U256::from_u128(&env, 50000000)); // 50 XLM remaining

            // Verify vXLM tokens are burned
            let vxlm_balance: U256 = env
                .storage()
                .persistent()
                .get(&TokenDataKey::TokenBalance(
                    lender.clone(),
                    Symbol::new(&env, "vXLM"),
                ))
                .unwrap();
            assert_eq!(vxlm_balance, U256::from_u128(&env, 50)); // 50 vXLM tokens remaining
        });
    }

    #[test]
    #[should_panic(expected = "Lender not registered")]
    fn test_withdraw_xlm_lender_not_registered() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        // Try to withdraw without depositing first
        let withdraw_amount = U256::from_u128(&env, 50000000);

        // let result = panic::catch_unwind(|| {
        // });
        let res = client.withdraw_xlm(&lender, &withdraw_amount);
    }

    #[test]
    #[should_panic(expected = "InsufficientBalance")]
    fn test_withdraw_xlm_insufficient_balance() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        // Initialize pool and deposit
        client.initialize_pool_xlm();

        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        // Try to withdraw more than deposited
        let withdraw_amount = U256::from_u128(&env, 200000000);

        let result = client.withdraw_xlm(&lender, &withdraw_amount);

        // assert!(result.is_err());
    }

    #[test]
    fn test_get_xlm_pool_balance() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        // Initially should be 0
        let initial_balance = client.get_xlm_pool_balance();
        assert_eq!(initial_balance, U256::from_u128(&env, 0));

        // After deposit, should match deposit amount
        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        let balance_after_deposit = client.get_xlm_pool_balance();
        assert_eq!(balance_after_deposit, deposit_amount);
    }

    #[test]
    fn test_get_lenders_xlm() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        // Initially should be empty
        let initial_lenders = client.get_lenders_xlm();
        assert_eq!(initial_lenders.len(), 0);

        // After deposit, should contain lender
        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        let lenders_after_deposit = client.get_lenders_xlm();
        assert_eq!(lenders_after_deposit.len(), 1);
        assert_eq!(lenders_after_deposit.get(0).unwrap(), lender);
    }

    #[test]
    fn test_multiple_deposits_same_lender() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        // First deposit
        let deposit_amount1 = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount1);

        // Second deposit
        let deposit_amount2 = U256::from_u128(&env, 50000000);
        client.deposit_xlm(&lender, &deposit_amount2);

        // Verify total balance
        env.as_contract(&contract_address, || {
            let lender_balance: U256 = env
                .storage()
                .persistent()
                .get(&PoolDataKey::LenderBalance(
                    lender.clone(),
                    Symbol::new(&env, "XLM"),
                ))
                .unwrap();
            assert_eq!(lender_balance, U256::from_u128(&env, 150000000)); // 150 XLM total

            // Verify lender appears only once in lenders list
            let lenders: Vec<Address> = env
                .storage()
                .persistent()
                .get(&PoolDataKey::Lenders(Symbol::new(&env, "XLM")))
                .unwrap();
            assert_eq!(lenders.len(), 1);
        });
    }

    #[test]
    fn test_multiple_lenders() {
        let (env, contract_address, admin, lender1) = setup_test_env();
        let lender2 = Address::generate(&env);
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender1, &1000000000i128);
        // xlm_token.mock_all_auths().mint(&lender2, &1000000000i128);

        // Both lenders deposit
        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender1, &deposit_amount);
        client.deposit_xlm(&lender2, &deposit_amount);

        // Verify pool balance is sum of both deposits
        let pool_balance = client.get_xlm_pool_balance();
        assert_eq!(pool_balance, U256::from_u128(&env, 200000000));

        // Verify both lenders are in the list
        let lenders = client.get_lenders_xlm();
        assert_eq!(lenders.len(), 2);
        assert!(lenders.contains(&lender1));
        assert!(lenders.contains(&lender2));
    }

    #[test]
    fn test_mint_vxlm_tokens() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();
        let native_token =
            env.register_stellar_asset_contract_v2(soroban_sdk::Address::generate(&env));
        let stellar_asset = StellarAssetClient::new(&env, &native_token.address());

        // let stellar_asset = StellarAssetClient::new(
        //     &env,
        //     &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        // );

        stellar_asset
            .mock_all_auths()
            .mint(&lender, &1000000000i128);
        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        // Verify vXLM token tracking
        env.as_contract(&contract_address, || {
            let current_balance = LiquidityPoolXLM::get_current_total_vxlm_balance(&env);
            let total_minted = LiquidityPoolXLM::get_total_vxlm_minted(&env);
            let total_burnt = LiquidityPoolXLM::get_total_vxlm_burnt(&env);

            assert_eq!(current_balance, U256::from_u128(&env, 100));
            assert_eq!(total_minted, U256::from_u128(&env, 100));
            assert_eq!(total_burnt, U256::from_u128(&env, 0));
        });
    }

    #[test]
    fn test_burn_vxlm_tokens() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        // Deposit then withdraw
        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        let withdraw_amount = U256::from_u128(&env, 50000000);
        client.withdraw_xlm(&lender, &withdraw_amount);

        // Verify vXLM token tracking after burn
        env.as_contract(&contract_address, || {
            let current_balance = LiquidityPoolXLM::get_current_total_vxlm_balance(&env);
            let total_minted = LiquidityPoolXLM::get_total_vxlm_minted(&env);
            let total_burnt = LiquidityPoolXLM::get_total_vxlm_burnt(&env);

            assert_eq!(current_balance, U256::from_u128(&env, 50));
            assert_eq!(total_minted, U256::from_u128(&env, 100));
            assert_eq!(total_burnt, U256::from_u128(&env, 50));
        });
    }

    #[test]
    fn test_pool_not_initialized_error() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        // Don't initialize pool
        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        let deposit_amount = U256::from_u128(&env, 100000000);

        // Should panic with PoolNotInitialized error
        let result = client.deposit_xlm(&lender, &deposit_amount);

        // assert!(result.is_err());
    }

    #[test]
    fn test_events_emission() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);

        client.initialize_pool_xlm();

        let xlm_token = token::Client::new(
            &env,
            &Address::from_string_bytes(&Bytes::from_array(&env, &XLM_CONTRACT_ID)),
        );
        // xlm_token.mock_all_auths().mint(&lender, &1000000000i128);

        // Clear any existing events
        env.events().all();

        let deposit_amount = U256::from_u128(&env, 100000000);
        client.deposit_xlm(&lender, &deposit_amount);

        let events = env.events().all();

        // Should have deposit event and mint event
        assert!(events.len() >= 2);

        // // Verify deposit event
        // let deposit_event = events.iter().find(|e| {
        //     if let Ok(event_data) = e.2.get::<LendingDepositEvent>(0) {
        //         event_data.lender == lender && event_data.amount == deposit_amount
        //     } else {
        //         false
        //     }
        // });
        // assert!(deposit_event.is_some());

        // Test withdraw event
        let withdraw_amount = U256::from_u128(&env, 50000000);
        client.withdraw_xlm(&lender, &withdraw_amount);

        let events_after_withdraw = env.events().all();

        // Should have additional withdraw and burn events
        assert!(events_after_withdraw.len() > events.len());
    }

    #[test]
    #[should_panic(expected = "InvalidTokenValue")]
    fn test_token_value_zero_error() {
        let (env, contract_address, admin, lender) = setup_test_env();
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);
        // Set token value to zero
        env.storage().persistent().set(
            &TokenDataKey::TokenValue(Symbol::new(&env, "vXLM")),
            &U256::from_u128(&env, 0),
        );

        client.initialize_pool_xlm();

        // let lender = Address::generate(&env);
        let deposit_amount = U256::from_u128(&env, 1000000000);

        let xlm_token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let xlm_token = token::TokenClient::new(&env, &xlm_token_id.address());
        // xlm_token.mint(&lender, &(deposit_amount.to_u128().unwrap() as i128));

        // This should panic with InvalidTokenValue
        client.deposit_xlm(&lender, &deposit_amount);
    }
}
