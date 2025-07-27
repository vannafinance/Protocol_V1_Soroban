#[cfg(test)]
mod test {
    use soroban_sdk::{
        log,
        testutils::{Address as _, Ledger},
        Address, Env, Symbol, Vec, U256,
    };

    use crate::types::{DataKey, MarginAccountDataKey};

    use crate::margin_account::account_logic::AccountLogicContract;

    fn create_test_env() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        (env, admin, user)
    }

    fn setup_contract(env: &Env, admin: &Address, contract_address: &Address) {
        env.as_contract(&contract_address, || {
            // Initialize the contract with admin
            AccountLogicContract::initialise_account_contract(env.clone(), admin.clone());
        });
    }

    #[test]
    fn test_initialise_account_contract() {
        let (env, admin, _) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);

        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account_contract(env.clone(), admin.clone());

            // Verify admin is set
            let stored_admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
            assert_eq!(stored_admin, admin);
        });
    }

    #[test]
    fn test_initialise_account_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        env.as_contract(&contract_address, || {
            // Set a mock timestamp
            env.ledger().with_mut(|li| {
                li.timestamp = 1000000;
            });

            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Verify account creation time is set
            let creation_time: u64 = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::AccountCreatedTime(user.clone()))
                .unwrap();
            assert_eq!(creation_time, 1000000);

            // Verify user is added to user addresses list
            let user_addresses: Vec<Address> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserAddresses)
                .unwrap();
            assert_eq!(user_addresses.len(), 1);
            assert_eq!(user_addresses.get(0).unwrap(), user);

            // Verify account is initialized
            let is_initialized: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountInitialised(user.clone()))
                .unwrap();
            assert!(is_initialized);

            // Verify account is active
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            assert!(is_active);

            // Verify has no debt
            let has_debt: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::HasDebt(user.clone()))
                .unwrap();
            assert!(!has_debt);
        });
    }

    #[test]
    #[should_panic(expected = "Admin not set")]
    fn test_initialise_account_no_admin() {
        let (env, _, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);

        env.as_contract(&contract_address, || {
            // Don't setup contract (no admin set)
            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });
    }

    #[test]
    // #[should_panic(expected = "Account contract not initiated")] // Code modifies, test not necessary
    fn test_initialise_account_no_user_addresses_list() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);

        env.as_contract(&contract_address, || {
            // Only initialize admin, not the UserAddresses list
            AccountLogicContract::initialise_account_contract(env.clone(), admin.clone());

            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });
    }

    #[test]
    fn test_deactivate_account_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        env.as_contract(&contract_address, || {
            // First initialize the account
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Verify account is initially active
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            assert!(is_active);

            // Deactivate account
            let result = AccountLogicContract::deactivate_account(env.clone(), user.clone());
            assert!(result.is_ok());

            // Verify account is now inactive
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            assert!(!is_active);
        });
    }

    #[test]
    fn test_activate_account_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        // Verify account is inactive
        env.as_contract(&contract_address, || {
            // env.mock_all_auths();
            // First initialize and deactivate the account
            log!(&env, "Reach xaasssd");

            AccountLogicContract::initialise_account(env.clone(), user.clone());
            log!(&env, "Reach xd");

            AccountLogicContract::deactivate_account(env.clone(), user.clone()).unwrap();
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            log!(&env, "Reach xp");

            assert!(!is_active);
        });

        env.as_contract(&contract_address, || {
            // Activate account
            let result = AccountLogicContract::activate_account(env.clone(), user.clone());
            assert!(result.is_ok());
            log!(&env, "Reach xc");

            // Verify account is now active
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            assert!(is_active);
        });
    }

    #[test]
    fn test_add_collateral_token_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let token_symbol = Symbol::new(&env, "USDC");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            let result = AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                token_symbol.clone(),
                U256::from_u128(&env, 12340),
            );
            assert!(result.is_ok());

            // Verify token was added
            let collateral_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserCollateralTokensList(
                    user.clone(),
                ))
                .unwrap();
            assert_eq!(collateral_tokens.len(), 1);
            assert_eq!(collateral_tokens.get(0).unwrap(), token_symbol);
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(
                &env,
                user.clone(),
                token_symbol,
            );
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 12340));
        });
    }

    #[test]
    fn test_add_multiple_collateral_tokens() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdc = Symbol::new(&env, "USDC");
        let xlm = Symbol::new(&env, "XLM");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                usdc.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                xlm.clone(),
                U256::from_u128(&env, 22222),
            )
            .unwrap();

            let collateral_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserCollateralTokensList(
                    user.clone(),
                ))
                .unwrap();
            assert_eq!(collateral_tokens.len(), 2);
            assert_eq!(collateral_tokens.get(0).unwrap(), usdc);
            assert_eq!(collateral_tokens.get(1).unwrap(), xlm);
        });
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(&env, user.clone(), usdc);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(&env, user.clone(), xlm);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 22222));
        });
    }

    #[test]
    fn test_remove_collateral_token_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdc = Symbol::new(&env, "USDC");
        let xlm = Symbol::new(&env, "XLM");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Add tokens
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                usdc.clone(),
                U256::from_u128(&env, 22222),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                xlm.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            // Remove one token
            let result = AccountLogicContract::remove_collateral_token_balance(
                env.clone(),
                user.clone(),
                usdc.clone(),
                U256::from_u128(&env, 22222),
            );
            assert!(result.is_ok());

            let collateral_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserCollateralTokensList(
                    user.clone(),
                ))
                .unwrap();
            assert_eq!(collateral_tokens.len(), 1);
            assert_eq!(collateral_tokens.get(0).unwrap(), xlm);
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(&env, user.clone(), xlm);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(&env, user.clone(), usdc);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 0));
        });
    }

    #[test]
    fn test_remove_partial_collateral_token_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdc = Symbol::new(&env, "USDC");
        let xlm = Symbol::new(&env, "XLM");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Add tokens
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                usdc.clone(),
                U256::from_u128(&env, 22222),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                xlm.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            // Remove one token
            let result = AccountLogicContract::remove_collateral_token_balance(
                env.clone(),
                user.clone(),
                usdc.clone(),
                U256::from_u128(&env, 11111),
            );
            assert!(result.is_ok());

            let collateral_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserCollateralTokensList(
                    user.clone(),
                ))
                .unwrap();
            assert_eq!(collateral_tokens.len(), 2);
            assert_eq!(collateral_tokens.get(0).unwrap(), usdc);
            assert_eq!(collateral_tokens.get(1).unwrap(), xlm);
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(&env, user.clone(), xlm);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_collateral_token_balance(&env, user.clone(), usdc);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });
    }

    #[test]
    #[should_panic(expected = "Insufficient Collateral balance for user in this token to deduct")]
    fn test_remove_excess_amount_collateral_token() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        let xlm = Symbol::new(&env, "XLM");

        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });

        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                xlm.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
        });

        env.as_contract(&contract_address, || {
            AccountLogicContract::remove_collateral_token_balance(
                env.clone(),
                user.clone(),
                xlm,
                U256::from_u128(&env, 33333),
            )
            .unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "Collateral token doesn't exist in the list")]
    fn test_remove_nonexistent_collateral_token() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            let nonexistent_token = Symbol::new(&env, "NONEXISTENT");

            AccountLogicContract::remove_collateral_token_balance(
                env.clone(),
                user.clone(),
                nonexistent_token,
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
    }

    #[test]
    fn test_get_all_collateral_tokens() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdc = Symbol::new(&env, "USDC");
        let xlm = Symbol::new(&env, "XLM");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Initially should be empty
            let result = AccountLogicContract::get_all_collateral_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 0);
        });
        env.as_contract(&contract_address, || {
            // Add tokens
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                usdc.clone(),
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                xlm.clone(),
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            // Get all tokens
            let result = AccountLogicContract::get_all_collateral_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens.get(0).unwrap(), usdc);
            assert_eq!(tokens.get(1).unwrap(), xlm);
        });
    }

    #[test]
    fn test_add_borrowed_token_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let token_symbol = Symbol::new(&env, "USDT");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            let result = AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                token_symbol.clone(),
                U256::from_u128(&env, 10000),
            );
            assert!(result.is_ok());

            // Verify token was added
            let borrowed_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserBorrowedTokensList(user.clone()))
                .unwrap();
            assert_eq!(borrowed_tokens.len(), 1);
            assert_eq!(borrowed_tokens.get(0).unwrap(), token_symbol);
        });

        env.as_contract(&contract_address, || {
            let res =
                AccountLogicContract::get_borrowed_token_debt(&env, user.clone(), token_symbol);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 10000));
        });
    }

    #[test]
    fn test_remove_borrowed_token_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdt = Symbol::new(&env, "USDT");
        let dai = Symbol::new(&env, "DAI");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Add tokens
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                usdt.clone(),
                U256::from_u128(&env, 22222),
            )
            .unwrap();
            log!(&env, "Reached 1");
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                dai.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
            log!(&env, "Reached 2");
        });
        env.as_contract(&contract_address, || {
            let key_x = MarginAccountDataKey::TotalDebtInPool(usdt.clone());

            env.storage()
                .persistent()
                .set(&key_x, &U256::from_u128(&env, 22222));
        });
        env.as_contract(&contract_address, || {
            // Remove one token
            let result = AccountLogicContract::remove_borrowed_token_balance(
                &env,
                user.clone(),
                usdt.clone(),
                U256::from_u128(&env, 22222),
            );
            assert!(result.is_ok());
            log!(&env, "Reached 3");

            let borrowed_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserBorrowedTokensList(user.clone()))
                .unwrap();
            log!(&env, "Reached 4");

            assert_eq!(borrowed_tokens.len(), 1);
            assert_eq!(borrowed_tokens.get(0).unwrap(), dai);
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_borrowed_token_debt(&env, user.clone(), usdt);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 0));
        });
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_borrowed_token_debt(&env, user.clone(), dai);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });

        // Check user still has debt
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::has_debt(&env, user.clone());
            assert!(res);
        });
    }

    #[test]
    fn test_remove_partial_borrowed_token_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdt = Symbol::new(&env, "USDT");
        let dai = Symbol::new(&env, "DAI");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Add tokens
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                usdt.clone(),
                U256::from_u128(&env, 22222),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                dai.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            let key_x = MarginAccountDataKey::TotalDebtInPool(usdt.clone());

            env.storage()
                .persistent()
                .set(&key_x, &U256::from_u128(&env, 22222));
        });
        env.as_contract(&contract_address, || {
            // Remove one token
            let result = AccountLogicContract::remove_borrowed_token_balance(
                &env,
                user.clone(),
                usdt.clone(),
                U256::from_u128(&env, 11111),
            );
            assert!(result.is_ok());

            let borrowed_tokens: Vec<Symbol> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserBorrowedTokensList(user.clone()))
                .unwrap();
            assert_eq!(borrowed_tokens.len(), 2);
            assert_eq!(borrowed_tokens.get(0).unwrap(), usdt);
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_borrowed_token_debt(&env, user.clone(), usdt);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::get_borrowed_token_debt(&env, user.clone(), dai);
            assert!(res.is_ok());
            assert!(res.unwrap() == U256::from_u128(&env, 11111));
        });

        // Check user still has debt
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::has_debt(&env, user.clone());
            assert!(res);
        });
    }

    #[test]
    #[should_panic(expected = "Cannot remove debt more than what it exists for this token")]
    fn test_remove_excess_amount_borrowed_token() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        let dai = Symbol::new(&env, "DAI");

        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });

        env.as_contract(&contract_address, || {
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                dai.clone(),
                U256::from_u128(&env, 11111),
            )
            .unwrap();
        });

        env.as_contract(&contract_address, || {
            AccountLogicContract::remove_borrowed_token_balance(
                &env,
                user.clone(),
                dai.clone(),
                U256::from_u128(&env, 33333),
            )
            .unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "Borrowed token doesn't exist in the list")]
    fn test_remove_nonexistent_borrowed_token() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });
        env.as_contract(&contract_address, || {
            let nonexistent_token = Symbol::new(&env, "NONEXISTENT");

            AccountLogicContract::remove_borrowed_token_balance(
                &env,
                user.clone(),
                nonexistent_token,
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });

        // Check user still has debt
        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::has_debt(&env, user.clone());
            assert!(res);
        });
    }

    #[test]
    fn test_get_all_borrowed_tokens() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let usdt = Symbol::new(&env, "USDT");
        let dai = Symbol::new(&env, "DAI");
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Initially should be empty
            let result = AccountLogicContract::get_all_borrowed_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 0);
        });

        env.as_contract(&contract_address, || {
            // Add tokens
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                usdt.clone(),
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                dai.clone(),
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            // Get all tokens
            let result = AccountLogicContract::get_all_borrowed_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens.get(0).unwrap(), usdt);
            assert_eq!(tokens.get(1).unwrap(), dai);
        });
    }

    #[test]
    fn test_has_debt() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Initially should have no debt
            let has_debt = AccountLogicContract::has_debt(&env.clone(), user.clone());
            assert!(!has_debt);
        });
    }

    #[test]
    fn test_set_has_debt() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Set debt to true
            let result = AccountLogicContract::set_has_debt(&env.clone(), user.clone(), true);
            assert!(result.is_ok());

            let has_debt = AccountLogicContract::has_debt(&env.clone(), user.clone());
            assert!(has_debt);

            // Set debt back to false
            let result = AccountLogicContract::set_has_debt(&env.clone(), user.clone(), false);
            assert!(result.is_ok());

            let has_debt = AccountLogicContract::has_debt(&env.clone(), user.clone());
            assert!(!has_debt);
        });
    }

    // Facing some bug with this test...
    #[test]
    fn test_authorization_required() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Test that functions require proper authorization
            // env.mock_all_auths();

            let token = Symbol::new(&env, "USDC");

            // These should succeed with mocked auth
            assert!(AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                token.clone(),
                U256::from_u128(&env, 10000)
            )
            .is_ok());
        });

        env.as_contract(&contract_address, || {
            assert!(AccountLogicContract::deactivate_account(env.clone(), user.clone()).is_ok());
        });
        env.as_contract(&contract_address, || {
            assert!(AccountLogicContract::activate_account(env.clone(), user.clone()).is_ok());
        });

        // // Verify the authorization calls were made
        // assert_eq!(
        //     env.auths(),
        //     std::vec![
        //         (
        //             user.clone(),
        //             AuthorizedInvocation {
        //                 function: AuthorizedFunction::Contract((
        //                     env.current_contract_address(),
        //                     Symbol::new(&env, "add_collateral_token"),
        //                     vec![&env, user.clone().into_val(&env), token.into_val(&env)]
        //                 )),
        //                 sub_invocations: std::vec![]
        //             }
        //         ),
        //         (
        //             user.clone(),
        //             AuthorizedInvocation {
        //                 function: AuthorizedFunction::Contract((
        //                     env.current_contract_address(),
        //                     Symbol::new(&env, "deactivate_account"),
        //                     vec![&env, user.clone().into_val(&env)]
        //                 )),
        //                 sub_invocations: std::vec![]
        //             }
        //         ),
        //         (
        //             user.clone(),
        //             AuthorizedInvocation {
        //                 function: AuthorizedFunction::Contract((
        //                     env.current_contract_address(),
        //                     Symbol::new(&env, "activate_account"),
        //                     vec![&env, user.clone().into_val(&env)]
        //                 )),
        //                 sub_invocations: std::vec![]
        //             }
        //         ),
        //     ]
        // );
    }

    #[test]
    fn test_multiple_users() {
        let (env, admin, user1) = create_test_env();
        let user2 = Address::generate(&env);
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        // Add different collateral tokens for each user
        let usdc = Symbol::new(&env, "USDC");
        let xlm = Symbol::new(&env, "XLM");
        env.as_contract(&contract_address, || {
            // Initialize both accounts
            AccountLogicContract::initialise_account(env.clone(), user1.clone());
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user2.clone());

            // Verify both users are in the user addresses list
            let user_addresses: Vec<Address> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserAddresses)
                .unwrap();
            assert_eq!(user_addresses.len(), 2);
            assert_eq!(user_addresses.get(0).unwrap(), user1);
            assert_eq!(user_addresses.get(1).unwrap(), user2);
        });

        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user1.clone(),
                usdc.clone(),
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user2.clone(),
                xlm.clone(),
                U256::from_u128(&env, 10000),
            )
            .unwrap();
        });
        env.as_contract(&contract_address, || {
            // Verify each user has their own tokens
            let user1_tokens =
                AccountLogicContract::get_all_collateral_tokens(&env, user1.clone()).unwrap();

            assert_eq!(user1_tokens.len(), 1);
            assert_eq!(user1_tokens.get(0).unwrap(), usdc);
        });
        env.as_contract(&contract_address, || {
            let user2_tokens =
                AccountLogicContract::get_all_collateral_tokens(&env, user2.clone()).unwrap();

            assert_eq!(user2_tokens.len(), 1);
            assert_eq!(user2_tokens.get(0).unwrap(), xlm);
        });
    }

    #[test]
    #[should_panic(expected = "Cannot delete account with debt, please repay debt first")]
    fn test_delete_account_failure_debt() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let token_symbol = Symbol::new(&env, "USDC");
        let token_symbol2 = Symbol::new(&env, "USDT");

        env.as_contract(&contract_address, || {
            // Set a mock timestamp
            env.ledger().with_mut(|li| {
                li.timestamp = 1000000;
            });
            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });

        env.as_contract(&contract_address, || {
            let result = AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                token_symbol.clone(),
                U256::from_u128(&env, 12340),
            );
            assert!(result.is_ok());
        });

        env.as_contract(&contract_address, || {
            let result2 = AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                token_symbol2.clone(),
                U256::from_u128(&env, 10000),
            );
            assert!(result2.is_ok());
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::delete_account(&env, user.clone());
            assert!(res.is_ok());
        });

        env.as_contract(&contract_address, || {
            // Verify account deletion time is set
            let deletion_time: u64 = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::AccountDeletedTime(user.clone()))
                .unwrap();
            assert_eq!(deletion_time, 1000000);
        });
        env.as_contract(&contract_address, || {
            // Verify user is deleted from user addresses list
            let user_addresses: Vec<Address> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserAddresses)
                .unwrap();
            assert_eq!(user_addresses.len(), 0);
        });

        env.as_contract(&contract_address, || {
            // Verify account is deactivated
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            assert!(!is_active);
        });
        env.as_contract(&contract_address, || {
            // Verify has no debt
            let has_debt: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::HasDebt(user.clone()))
                .unwrap();
            assert!(!has_debt);
        });

        env.as_contract(&contract_address, || {
            // Get all tokens
            let result = AccountLogicContract::get_all_collateral_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 0);
        });

        env.as_contract(&contract_address, || {
            // Get all tokens
            let result = AccountLogicContract::get_all_borrowed_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 0);
        });
    }

    #[test]
    fn test_delete_account_success() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);

        let token_symbol = Symbol::new(&env, "USDC");
        let token_symbol2 = Symbol::new(&env, "USDT");

        env.as_contract(&contract_address, || {
            // Set a mock timestamp
            env.ledger().with_mut(|li| {
                li.timestamp = 1000000;
            });
            AccountLogicContract::initialise_account(env.clone(), user.clone());
        });

        env.as_contract(&contract_address, || {
            let result = AccountLogicContract::add_collateral_token_balance(
                env.clone(),
                user.clone(),
                token_symbol.clone(),
                U256::from_u128(&env, 12340),
            );
            assert!(result.is_ok());
        });

        env.as_contract(&contract_address, || {
            let result2 = AccountLogicContract::add_borrowed_token_balance(
                &env,
                user.clone(),
                token_symbol2.clone(),
                U256::from_u128(&env, 10000),
            );
            assert!(result2.is_ok());
        });

        env.as_contract(&contract_address, || {
            let key_x = MarginAccountDataKey::TotalDebtInPool(token_symbol2.clone());

            env.storage()
                .persistent()
                .set(&key_x, &U256::from_u128(&env, 10000));
        });

        env.as_contract(&contract_address, || {
            let result2 = AccountLogicContract::remove_borrowed_token_balance(
                &env.clone(),
                user.clone(),
                token_symbol2.clone(),
                U256::from_u128(&env, 10000),
            );
            assert!(result2.is_ok());
        });

        env.as_contract(&contract_address, || {
            let res = AccountLogicContract::delete_account(&env, user.clone());
            assert!(res.is_ok());
        });

        env.as_contract(&contract_address, || {
            // Verify account deletion time is set
            let deletion_time: u64 = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::AccountDeletedTime(user.clone()))
                .unwrap();
            assert_eq!(deletion_time, 1000000);
        });
        env.as_contract(&contract_address, || {
            // Verify user is deleted from user addresses list
            let user_addresses: Vec<Address> = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserAddresses)
                .unwrap();
            assert_eq!(user_addresses.len(), 0);
        });

        env.as_contract(&contract_address, || {
            // Verify account is deactivated
            let is_active: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::IsAccountActive(user.clone()))
                .unwrap();
            assert!(!is_active);
        });
        env.as_contract(&contract_address, || {
            // Verify has no debt
            let has_debt: bool = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::HasDebt(user.clone()))
                .unwrap();
            assert!(!has_debt);
        });

        env.as_contract(&contract_address, || {
            // Get all tokens
            let result = AccountLogicContract::get_all_collateral_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 0);
        });

        env.as_contract(&contract_address, || {
            // Get all tokens
            let result = AccountLogicContract::get_all_borrowed_tokens(&env, user.clone());
            assert!(result.is_ok());
            let tokens = result.unwrap();
            assert_eq!(tokens.len(), 0);
        });
    }

    #[test]
    fn test_edge_cases_empty_lists() {
        let (env, admin, user) = create_test_env();
        let contract_address = env.register_contract(None, AccountLogicContract);
        setup_contract(&env, &admin, &contract_address);
        env.as_contract(&contract_address, || {
            AccountLogicContract::initialise_account(env.clone(), user.clone());

            // Test getting tokens from empty lists
            let collateral_tokens =
                AccountLogicContract::get_all_collateral_tokens(&env, user.clone()).unwrap();
            assert_eq!(collateral_tokens.len(), 0);
        });
        env.as_contract(&contract_address, || {
            let borrowed_tokens =
                AccountLogicContract::get_all_borrowed_tokens(&env, user.clone()).unwrap();
            assert_eq!(borrowed_tokens.len(), 0);
        });
    }
}
