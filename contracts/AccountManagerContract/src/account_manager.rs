use core::{ops::Add, panic};

use soroban_sdk::{
    Address, BytesN, Env, Symbol, U256, Vec, contract, contractimpl, panic_with_error, xdr::ToXdr,
};

use crate::types::{
    AccountDeletionEvent, AccountManagerError, AccountManagerKey, TraderBorrowEvent,
    TraderLiquidateEvent, TraderRepayEvent, TraderSettleAccountEvent,
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

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
    }

    pub fn create_account(
        env: &Env,
        user_address: Address,
    ) -> Result<Address, AccountManagerError> {
        let mut users = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::UsersList)
            .unwrap_or_else(|| Vec::new(&env));

        if !users.contains(&user_address) {
            users.push_back(user_address);
            env.storage()
                .persistent()
                .set(&AccountManagerKey::UsersList, users);
        }

        let registry_contract_address: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::RegistryContract)
            .expect("Failed to get registry contract key!");
        let registry_client = registry_contract::Client::new(env, &registry_contract_address);
        let smart_account_hash = registry_client.get_smart_account_hash();

        let salt =
            Self::generate_predictable_salt(&env, &user_address, &env.current_contract_address());

        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(registry_contract_address.to_val());
        constructor_args.push_back(env.current_contract_address().to_val());
        constructor_args.push_back(user_address.to_val());

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(smart_account_hash, constructor_args);

        env.storage().persistent().set(
            &AccountManagerKey::SmartAccountAddress(user_address.clone()),
            &deployed_address,
        );

        Ok(deployed_address)
    }

    pub fn delete_account(
        env: &Env,
        smart_account_address: Address,
        user_address: Address,
    ) -> Result<(), AccountManagerError> {
        user_address.require_auth();
        let account_contract_address: Address = Address::from_str(&env, "strkey");
        let account_contract_client =
            smart_account_contract::Client::new(&env, &account_contract_address);

        let has_debt = account_contract_client.has_debt();

        if has_debt {
            panic!("Cannot delete account with debt, please repay debt first");
        }

        // Set account deletion time
        env.storage().persistent().set(
            &AccountManagerKey::AccountDeletedTime(user_address.clone()),
            &env.ledger().timestamp(),
        );

        // remove user's address from list of Margin account user addresses
        let key_d = AccountManagerKey::UserAddresses(user_address.clone());
        let mut user_addresses: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key_d)
            .expect("Account contract not initiated");
        let index = user_addresses
            .first_index_of(user_address.clone())
            .unwrap_or_else(|| panic!("User account not found in list"));
        user_addresses.remove(index);
        env.storage().persistent().set(&key_d, &user_addresses);
        Self::extend_ttl_account_manager(&env, key_d);

        let borrowed_tokens_symbols: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        // Remove balance for each borrowed token
        for symbol in borrowed_tokens_symbols {
            env.storage()
                .persistent()
                .remove(&AccountManagerKey::UserBorrowedDebt(
                    user_address.clone(),
                    symbol,
                ));
        }
        // Then remove all borrowed tokens from the list
        env.storage()
            .persistent()
            .remove(&AccountManagerKey::UserBorrowedTokensList(
                user_address.clone(),
            ));

        let collateral_tokens: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));

        // Remove balance for each collateral token
        for symbolx in collateral_tokens {
            env.storage()
                .persistent()
                .remove(&AccountManagerKey::UserCollateralBalance(
                    user_address.clone(),
                    symbolx,
                ));
        }

        // Then remove all collateral tokens from the list
        env.storage()
            .persistent()
            .remove(&AccountManagerKey::UserCollateralTokensList(
                user_address.clone(),
            ));

        let key_c = AccountManagerKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key_c, &false);
        Self::extend_ttl_account_manager(&env, key_c);

        let key_d = AccountManagerKey::HasDebt(user_address.clone());
        env.storage().persistent().set(&key_d, &false);
        Self::extend_ttl_account_manager(&env, key_d);

        env.events().publish(
            (Symbol::new(&env, "Account_Deleted"), user_address.clone()),
            AccountDeletionEvent {
                margin_account: user_address,
                deletion_time: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn deposit_collateral_tokens(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), AccountManagerError> {
        user_address.require_auth();

        if !Self::get_iscollateral_allowed(&env, token_symbol.clone()) {
            panic!("This token is not allowed as collateral");
        }

        let key_c = AccountManagerKey::UserCollateralTokensList(user_address.clone());
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| Vec::new(&env));

        if U256::from_u32(&env, collateral_tokens_list.len()) >= Self::get_max_asset_cap(&env) {
            panic!("Max asset cap crossed!");
        };

        if !collateral_tokens_list.contains(token_symbol.clone()) {
            collateral_tokens_list.push_back(token_symbol.clone());
        }

        env.storage()
            .persistent()
            .set(&key_c, &collateral_tokens_list);
        Self::extend_ttl_account_manager(&env, key_c);

        let key_a =
            AccountManagerKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let new_balance = token_balance.add(&token_amount);
        env.storage().persistent().set(&key_a, &new_balance);
        Self::extend_ttl_account_manager(&env, key_a);

        Ok(())
    }

    pub fn remove_collateral_tokens(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), AccountManagerError> {
        user_address.require_auth();

        let key_a = AccountManagerKey::UserCollateralTokensList(user_address.clone());
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| Vec::new(&env));
        let index = collateral_tokens_list
            .first_index_of(token_symbol.clone())
            .unwrap_or_else(|| panic!("Collateral token doesn't exist in the list"));

        let key_b =
            AccountManagerKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        if token_amount > token_balance {
            panic!("Insufficient Collateral balance for user in this token to deduct",);
        }
        let new_balance = token_balance.sub(&token_amount);
        env.storage().persistent().set(&key_b, &new_balance);
        Self::extend_ttl_account_manager(&env, key_b);

        if token_amount == token_balance {
            collateral_tokens_list.remove(index);
            env.storage()
                .persistent()
                .set(&key_a, &collateral_tokens_list);

            Self::extend_ttl_account_manager(&env, key_a);
        }
        Ok(())
    }

    pub fn borrow(
        env: &Env,
        borrow_amount: U256,
        token_symbol: Symbol,
        user_account: Address,
    ) -> Result<(), AccountManagerError> {
        user_account.require_auth();

        let registry_address = Self::get_registry_address(env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let risk_engine_address = registry_client.get_risk_engine_address();

        let risk_engine_client = risk_engine_contract::Client::new(&env, &risk_engine_address);

        let smart_account_address: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::SmartAccountAddress(
                user_account.clone(),
            ))
            .expect("Failed to fetch users smart account address");

        if !risk_engine_client.is_borrow_allowed(
            &token_symbol.clone(),
            &borrow_amount,
            &smart_account_address,
        ) {
            panic!("Borrowing is not allowed");
        }

        let borrow_amount_u128 = borrow_amount.to_u128().unwrap_or_else(|| {
            panic_with_error!(&env, AccountManagerError::IntegerConversionError)
        });

        if token_symbol == Symbol::new(&env, "XLM") {
            let pool_xlm_contract = registry_client.get_lendingpool_xlm();

            let xlm_client: lending_protocol_xlm::Client<'_> =
                lending_protocol_xlm::Client::new(&env, &pool_xlm_contract);

            xlm_client.lend_to(&smart_account_address, &borrow_amount);
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let pool_usdc_contract = registry_client.get_lendingpool_usdc();

            let usdc_client: lending_protocol_usdc::Client<'_> =
                lending_protocol_usdc::Client::new(&env, &pool_usdc_contract);
            usdc_client.lend_to(&smart_account_address, &borrow_amount);
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let pool_eurc_contract = registry_client.get_lendingpool_eurc();

            let eurc_client: lending_protocol_eurc::Client<'_> =
                lending_protocol_eurc::Client::new(&env, &pool_eurc_contract);
            eurc_client.lend_to(&smart_account_address, &borrow_amount);
        } else {
            panic!("No lending pool available for given token_symbol");
        }

        // let pool_balance: U256 = env
        //     .storage()
        //     .persistent()
        //     .get(&PoolDataKey::Pool(token_symbol.clone()))
        //     .unwrap_or_else(|| panic!("Pool doesn't exist"));

        // if pool_balance < borrow_amount {
        //     panic!("Pool balance is not enough to borrow");
        // }

        // let new_pool_balance = pool_balance.sub(&borrow_amount.clone());
        // env.storage()
        //     .persistent()
        //     .set(&PoolDataKey::Pool(token_symbol.clone()), &new_pool_balance);
        // Self::extend_ttl_pooldatakey(&env, PoolDataKey::Pool(token_symbol.clone()));

        // AccountLogicContract::add_borrowed_token_balance(
        //     &env,
        //     margin_account.clone(),
        //     token_symbol.clone(),
        //     borrow_amount.clone(),
        // )
        // .unwrap();

        env.events().publish(
            (
                Symbol::new(&env, "Trader Borrow Event"),
                smart_account_address.clone(),
            ),
            TraderBorrowEvent {
                margin_account: smart_account_address,
                token_amount: borrow_amount,
                timestamp: env.ledger().timestamp(),
                token_symbol,
                token_value: U256::from_u128(&env, 0),
            },
        );

        Ok(())
    }

    pub fn repay(
        env: Env,
        repay_amount: U256,
        token_symbol: Symbol,
        user_account: Address,
    ) -> Result<(), AccountManagerError> {
        user_account.require_auth();

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // This function is faulty, most logic shall be handled by lending pool contract
        // We should only do checks before calling lending pool contract to receive repay money from trader
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!

        let registry_address: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::RegistryContract)
            .unwrap();

        let smart_account_address: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::SmartAccountAddress(
                user_account.clone(),
            ))
            .expect("Failed to fetch users smart account address");

        let registry_client = registry_contract::Client::new(&env, &registry_address);

        let smart_account_client =
            smart_account_contract::Client::new(&env, &smart_account_address);

        let borrowed_tokens = smart_account_client.get_all_borrowed_tokens();

        if !borrowed_tokens.contains(token_symbol.clone()) {
            panic!("User doen't have debt in the token symbol passed");
        }

        let debt = smart_account_client.get_borrowed_token_debt(&token_symbol.clone());
        // !! Should we check if the repay amount is greater than the debt amount?

        if token_symbol == Symbol::new(&env, "XLM") {
            let pool_xlm_contract = registry_client.get_lendingpool_xlm();

            let xlm_client: lending_protocol_xlm::Client<'_> =
                lending_protocol_xlm::Client::new(&env, &pool_xlm_contract);
            xlm_client.collect_from(&repay_amount, &smart_account_address);
        } else if token_symbol == Symbol::new(&env, "USDC") {
            let pool_usdc_contract = registry_client.get_lendingpool_usdc();

            let usdc_client: lending_protocol_usdc::Client<'_> =
                lending_protocol_usdc::Client::new(&env, &pool_usdc_contract);
            usdc_client.collect_from(&repay_amount, &smart_account_address);
        } else if token_symbol == Symbol::new(&env, "EURC") {
            let pool_eurc_contract = registry_client.get_lendingpool_eurc();

            let eurc_client: lending_protocol_eurc::Client<'_> =
                lending_protocol_eurc::Client::new(&env, &pool_eurc_contract);
            eurc_client.collect_from(&repay_amount, &smart_account_address);
        } else {
            panic!("No lending pool available for given token_symbol");
        }

        env.events().publish(
            (
                Symbol::new(&env, "Trader Repay Event"),
                smart_account_address.clone(),
            ),
            TraderRepayEvent {
                margin_account: smart_account_address,
                token_amount: repay_amount,
                timestamp: env.ledger().timestamp(),
                token_symbol,
                token_value: U256::from_u128(&env, 0), // fix this
            },
        );
        Ok(())
    }

    pub fn liquidate(env: Env, margin_account: Address) -> Result<(), AccountManagerError> {
        let account_contract_address: Address;

        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &account_contract_address);
        let all_borrowed_tokens = smart_account_contract_client.get_all_borrowed_tokens();

        for tokenx in all_borrowed_tokens.iter() {
            let token_debt = smart_account_contract_client.get_borrowed_token_debt(&tokenx.clone());
            // let (client_address, pool_address) =
            //     Self::get_token_client_and_pool_address(&env, tokenx.clone());
            // let token_client = token::Client::new(&env, &client_address);
            // let trader_token_balance = token_client.balance(&margin_account) as u128;

            let liquidate_amount = token_debt.to_u128().unwrap_or_else(|| {
                panic_with_error!(&env, AccountManagerError::IntegerConversionError)
            });

            // if liquidate_amount > trader_token_balance {
            //     token_client.transfer(
            //         &margin_account, // from
            //         &pool_address,   // to
            //         &(trader_token_balance as i128),
            //     );
            // } else {
            //     token_client.transfer(
            //         &margin_account, // from
            //         &pool_address,   // to
            //         &(liquidate_amount as i128),
            //     );
            // }

            AccountLogicContract::remove_borrowed_token_balance(
                &env,
                margin_account.clone(),
                tokenx.clone(),
                token_debt,
            )
            .unwrap();
            // Self::set_last_updated_time(&env, tokenx);
        }

        let all_collateral_tokens = smart_account_contract_client.get_all_collateral_tokens();
        for coltoken in all_collateral_tokens.iter() {
            let coltokenbalance =
                smart_account_contract_client.get_collateral_token_balance(&coltoken.clone());
            // let (client_address, pool_address) =
            //     Self::get_token_client_and_pool_address(&env, coltoken.clone());
            // let token_client = token::Client::new(&env, &client_address);

            let col_token_amount = coltokenbalance.to_u128().unwrap_or_else(|| {
                panic_with_error!(&env, AccountManagerError::IntegerConversionError)
            });
            // token_client.transfer(
            //     &pool_address,   // from
            //     &margin_account, // to
            //     &(col_token_amount as i128),
            // );

            AccountLogicContract::remove_collateral_token_balance(
                env.clone(),
                margin_account.clone(),
                coltoken,
                coltokenbalance,
            )
            .unwrap();
        }

        env.events().publish(
            (
                Symbol::new(&env, "Trader Liquidate Event"),
                margin_account.clone(),
            ),
            TraderLiquidateEvent {
                margin_account,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn settle_account(env: Env, margin_account: Address) -> Result<(), AccountManagerError> {
        margin_account.require_auth();
        let smart_account_contract_address: Address;
        let smart_account_contract_client =
            smart_account_contract::Client::new(&env, &smart_account_contract_address);
        let borrowed_tokens = smart_account_contract_client.get_all_borrowed_tokens();
        // let borrowed_tokens =
        //     AccountLogicContract::get_all_borrowed_tokens(&env, margin_account.clone())
        //         .expect("Failed to fetch borrowed tokens list");
        for tokenx in borrowed_tokens.iter() {
            let token_debt = smart_account_contract_client.get_borrowed_token_debt(&tokenx.clone());
            Self::repay(env.clone(), token_debt, tokenx, margin_account.clone())
                .expect("Failed to repay while settling the account");
        }

        env.events().publish(
            (
                Symbol::new(&env, "Trader SettleAccount Event"),
                margin_account.clone(),
            ),
            TraderSettleAccountEvent {
                margin_account,
                timestamp: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    fn extend_ttl_account_manager(env: &Env, key: AccountManagerKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    pub fn set_max_asset_cap(env: &Env, cap: U256) {
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

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&AccountManagerKey::RegistryContract)
            .unwrap()
    }

    fn generate_predictable_salt(
        env: &Env,
        native_token: &Address,
        vxlm_token: &Address,
    ) -> BytesN<32> {
        let mut salt_bytes = [0u8; 32];

        // Use hash of token addresses for deterministic salt
        let native_xdr = native_token.to_xdr(env);
        let vxlm_xdr = vxlm_token.to_xdr(env);

        // Copy first 16 bytes from each address
        let native_len = (native_xdr.len() as usize).min(16);
        let vxlm_len = (vxlm_xdr.len() as usize).min(16);

        for i in 0..native_len {
            salt_bytes[i] = native_xdr.get(i as u32).unwrap_or(0);
        }

        for i in 0..vxlm_len {
            salt_bytes[16 + i] = vxlm_xdr.get(i as u32).unwrap_or(0);
        }

        BytesN::from_array(env, &salt_bytes)
    }
}
