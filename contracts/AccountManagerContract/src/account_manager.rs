use core::panic;

use soroban_sdk::{Address, Env, Symbol, U256, Vec, contract, panic_with_error};

use crate::types::{
    AccountDataKey, AccountDeletionEvent, AccountError, TraderBorrowEvent, TraderLiquidateEvent,
    TraderRepayEvent, TraderSettleAccountEvent,
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

pub mod account_contract {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/account_contract.wasm");
}

#[contract]
pub struct AccountManagerContract;

impl AccountManagerContract {
    pub fn delete_account(env: &Env, user_address: Address) -> Result<(), AccountError> {
        user_address.require_auth();

        if Self::has_debt(env, user_address.clone()) {
            panic!("Cannot delete account with debt, please repay debt first");
        }

        // Set account deletion time
        env.storage().persistent().set(
            &AccountDataKey::AccountDeletedTime(user_address.clone()),
            &env.ledger().timestamp(),
        );

        // remove user's address from list of Margin account user addresses
        let key_d = AccountDataKey::UserAddresses;
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
        Self::extend_ttl_margin_account(&env, key_d);

        let borrowed_tokens_symbols: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&AccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        // Remove balance for each borrowed token
        for symbol in borrowed_tokens_symbols {
            env.storage()
                .persistent()
                .remove(&AccountDataKey::UserBorrowedDebt(
                    user_address.clone(),
                    symbol,
                ));
        }
        // Then remove all borrowed tokens from the list
        env.storage()
            .persistent()
            .remove(&AccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ));

        let collateral_tokens: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&AccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));

        // Remove balance for each collateral token
        for symbolx in collateral_tokens {
            env.storage()
                .persistent()
                .remove(&AccountDataKey::UserCollateralBalance(
                    user_address.clone(),
                    symbolx,
                ));
        }

        // Then remove all collateral tokens from the list
        env.storage()
            .persistent()
            .remove(&AccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ));

        let key_c = AccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key_c, &false);
        Self::extend_ttl_margin_account(&env, key_c);

        let key_d = AccountDataKey::HasDebt(user_address.clone());
        env.storage().persistent().set(&key_d, &false);
        Self::extend_ttl_margin_account(&env, key_d);

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
    ) -> Result<(), AccountError> {
        user_address.require_auth();

        if !Self::get_iscollateral_allowed(&env, token_symbol.clone()) {
            panic!("This token is not allowed as collateral");
        }

        let key_c = AccountDataKey::UserCollateralTokensList(user_address.clone());
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
        Self::extend_ttl_margin_account(&env, key_c);

        let key_a =
            AccountDataKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let new_balance = token_balance.add(&token_amount);
        env.storage().persistent().set(&key_a, &new_balance);
        Self::extend_ttl_margin_account(&env, key_a);

        Ok(())
    }

    pub fn remove_collateral_tokens(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), AccountError> {
        user_address.require_auth();

        let key_a = AccountDataKey::UserCollateralTokensList(user_address.clone());
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| Vec::new(&env));
        let index = collateral_tokens_list
            .first_index_of(token_symbol.clone())
            .unwrap_or_else(|| panic!("Collateral token doesn't exist in the list"));

        let key_b =
            AccountDataKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
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
        Self::extend_ttl_margin_account(&env, key_b);

        if token_amount == token_balance {
            collateral_tokens_list.remove(index);
            env.storage()
                .persistent()
                .set(&key_a, &collateral_tokens_list);

            Self::extend_ttl_margin_account(&env, key_a);
        }

        Ok(())
    }

    pub fn borrow(
        env: &Env,
        borrow_amount: U256,
        token_symbol: Symbol,
        margin_account: Address,
    ) -> Result<(), BorrowError> {
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // This function is faulty, most logic shall be handled by lending pool contract
        // We should only do checks before calling lending pool contract to lend money to trader
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!

        margin_account.require_auth();
        let pool_balance: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(token_symbol.clone()))
            .unwrap_or_else(|| panic!("Pool doesn't exist"));

        if !Self::is_borrow_allowed(
            env,
            token_symbol.clone(),
            borrow_amount.clone(),
            margin_account.clone(),
        )
        .unwrap()
        {
            panic!("Borrowing is not allowed");
        }
        if pool_balance < borrow_amount {
            panic!("Pool balance is not enough to borrow");
        }
        let (client_address, pool_address) =
            Self::get_token_client_and_pool_address(&env, token_symbol.clone());
        let token_client = token::Client::new(&env, &client_address);

        // Allow user to borrow
        // Transfer borrow amount from pool to user
        let borrow_amount_u128 = borrow_amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

        // !!!!!! wrong
        token_client.transfer(
            &pool_address,   // from
            &margin_account, // to
            &(borrow_amount_u128 as i128),
        );

        let new_pool_balance = pool_balance.sub(&borrow_amount.clone());
        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(token_symbol.clone()), &new_pool_balance);
        Self::extend_ttl_pooldatakey(&env, PoolDataKey::Pool(token_symbol.clone()));

        AccountLogicContract::add_borrowed_token_balance(
            &env,
            margin_account.clone(),
            token_symbol.clone(),
            borrow_amount.clone(),
        )
        .unwrap();

        env.events().publish(
            (
                Symbol::new(&env, "Trader Borrow Event"),
                margin_account.clone(),
            ),
            TraderBorrowEvent {
                margin_account,
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
        margin_account: Address,
    ) -> Result<(), BorrowError> {
        margin_account.require_auth();

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // This function is faulty, most logic shall be handled by lending pool contract
        // We should only do checks before calling lending pool contract to receive repay money from trader
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!

        let borrowed_tokens =
            AccountLogicContract::get_all_borrowed_tokens(&env, margin_account.clone())
                .expect("Failed to fetch borrowed tokens list");

        if !borrowed_tokens.contains(token_symbol.clone()) {
            panic!("User doen't have debt in the token symbol passed");
        }

        let debt = AccountLogicContract::get_borrowed_token_debt(
            &env,
            margin_account.clone(),
            token_symbol.clone(),
        )
        .expect("Failed to fetch debt value for user and token_symbol passed");

        let (client_address, pool_address) =
            Self::get_token_client_and_pool_address(&env, token_symbol.clone());

        if repay_amount <= debt {
            let token_client = token::Client::new(&env, &client_address);
            let trader_token_balance = token_client.balance(&margin_account) as u128;

            let repay_amount_u128 = repay_amount
                .to_u128()
                .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

            token_client.transfer(
                &margin_account, // from
                &pool_address,   // to
                &(repay_amount_u128 as i128),
            );

            if U256::from_u128(&env, trader_token_balance) < repay_amount {
                panic!("Trader doesn't have enough balance to repay this token");
            }

            AccountLogicContract::remove_borrowed_token_balance(
                &env,
                margin_account.clone(),
                token_symbol.clone(),
                repay_amount.clone(),
            )
            .unwrap();

            Self::set_last_updated_time(&env, token_symbol.clone());
        }

        env.events().publish(
            (
                Symbol::new(&env, "Trader Repay Event"),
                margin_account.clone(),
            ),
            TraderRepayEvent {
                margin_account,
                token_amount: repay_amount,
                timestamp: env.ledger().timestamp(),
                token_symbol,
                token_value: U256::from_u128(&env, 0),
            },
        );
        Ok(())
    }

    pub fn liquidate(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        let all_borrowed_tokens =
            AccountLogicContract::get_all_borrowed_tokens(&env.clone(), margin_account.clone())
                .unwrap();

        for tokenx in all_borrowed_tokens.iter() {
            let token_debt = AccountLogicContract::get_borrowed_token_debt(
                &env.clone(),
                margin_account.clone(),
                tokenx.clone(),
            )
            .unwrap();
            let (client_address, pool_address) =
                Self::get_token_client_and_pool_address(&env, tokenx.clone());
            let token_client = token::Client::new(&env, &client_address);
            let trader_token_balance = token_client.balance(&margin_account) as u128;

            let liquidate_amount = token_debt
                .to_u128()
                .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));

            if liquidate_amount > trader_token_balance {
                token_client.transfer(
                    &margin_account, // from
                    &pool_address,   // to
                    &(trader_token_balance as i128),
                );
            } else {
                token_client.transfer(
                    &margin_account, // from
                    &pool_address,   // to
                    &(liquidate_amount as i128),
                );
            }

            AccountLogicContract::remove_borrowed_token_balance(
                &env,
                margin_account.clone(),
                tokenx.clone(),
                token_debt,
            )
            .unwrap();
            Self::set_last_updated_time(&env, tokenx);
        }

        let all_collateral_tokens =
            AccountLogicContract::get_all_collateral_tokens(&env.clone(), margin_account.clone())
                .unwrap();
        for coltoken in all_collateral_tokens.iter() {
            let coltokenbalance = AccountLogicContract::get_collateral_token_balance(
                &env,
                margin_account.clone(),
                coltoken.clone(),
            )
            .unwrap();
            let (client_address, pool_address) =
                Self::get_token_client_and_pool_address(&env, coltoken.clone());
            let token_client = token::Client::new(&env, &client_address);

            let col_token_amount = coltokenbalance
                .to_u128()
                .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));
            token_client.transfer(
                &pool_address,   // from
                &margin_account, // to
                &(col_token_amount as i128),
            );

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

    pub fn settle_account(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        margin_account.require_auth();
        let borrowed_tokens =
            AccountLogicContract::get_all_borrowed_tokens(&env, margin_account.clone())
                .expect("Failed to fetch borrowed tokens list");
        for tokenx in borrowed_tokens.iter() {
            let token_debt = AccountLogicContract::get_borrowed_token_debt(
                &env,
                margin_account.clone(),
                tokenx.clone(),
            )
            .expect("Failed to fetch debt value for user and token_symbol passed");
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

    fn extend_ttl_margin_account(env: &Env, key: AccountDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    pub fn set_max_asset_cap(env: &Env, cap: U256) {
        let key = AccountDataKey::AssetCap;
        env.storage().persistent().set(&key, &cap);
        Self::extend_ttl_margin_account(env, key);
    }

    pub fn get_max_asset_cap(env: &Env) -> U256 {
        let key = AccountDataKey::AssetCap;
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("Asset cap not set"))
    }
}
