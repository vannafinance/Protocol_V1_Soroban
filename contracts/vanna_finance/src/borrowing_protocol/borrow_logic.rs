use soroban_sdk::{contract, contractimpl, log, panic_with_error, Address, Env, Symbol, Vec, U256};

use crate::{
    errors::{BorrowError, MarginAccountError},
    margin_account::account_logic::AccountLogicContract,
    types::{DataKey, MarginAccountDataKey, PoolDataKey},
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const TLL_LEDGERS_MONTH: u32 = 518400;
const BALANCE_TO_BORROW_THRESHOLD: u128 = 1100000000000000000;

#[contract]
pub struct BorrowLogicContract;

impl BorrowLogicContract {
    pub fn borrow(
        env: &Env,
        borrow_amount: U256,
        symbol: Symbol,
        margin_account: Address,
    ) -> Result<(), BorrowError> {
        let pool_balance: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(symbol.clone()))
            .unwrap_or_else(|| panic!("Pool doesn't exist"));

        if !Self::is_borrow_allowed(
            env,
            symbol.clone(),
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

        // Allow user to borrow
        let new_pool_balance = pool_balance.sub(&borrow_amount);
        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(symbol.clone()), &new_pool_balance);

        AccountLogicContract::add_borrowed_token_balance(
            &env,
            margin_account.clone(),
            symbol,
            borrow_amount,
        )
        .unwrap();
        AccountLogicContract::set_has_debt(&env, margin_account, true).unwrap();

        Ok(())
    }

    pub fn repay(
        env: Env,
        amount: u64,
        symbol: Symbol,
        margin_account: Address,
    ) -> Result<(), BorrowError> {
        Ok(())
    }

    pub fn liquidate(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        Ok(())
    }

    pub fn settle_account(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        Ok(())
    }

    pub fn is_borrow_allowed(
        env: &Env,
        symbol: Symbol,
        borrow_amount: U256,
        margin_account: Address,
    ) -> Result<bool, BorrowError> {
        // Todo!!!! Fetch price from oracle !!!!!!!!!!!!!!!!!!!!!!!!
        let oracle_price = U256::from_u128(&env, 1);
        let borrow_value = borrow_amount.mul(&oracle_price);
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

        let current_account_balance =
            Self::get_current_total_balance(&env, margin_account.clone()).unwrap();
        let current_account_debt =
            Self::get_current_total_borrows(&env, margin_account.clone()).unwrap();
        let res = Self::is_account_healthy(
            env,
            current_account_balance.add(&borrow_value),
            current_account_debt.add(&borrow_value),
        )
        .unwrap();
        Ok(res)
    }

    pub fn is_withdraw_allowed(
        env: &Env,
        symbol: Symbol,
        withdraw_amount: U256,
        margin_account: Address,
    ) -> Result<bool, BorrowError> {
        // Todo!!!! Fetch price from oracle !!!!!!!!!!!!!!!!!!!!!!!!
        let oracle_price = U256::from_u128(&env, 1);
        let withdraw_value = withdraw_amount.mul(&oracle_price);
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

        let current_account_balance =
            Self::get_current_total_balance(&env, margin_account.clone()).unwrap();
        let current_account_debt =
            Self::get_current_total_borrows(&env, margin_account.clone()).unwrap();

        let res = Self::is_account_healthy(
            env,
            current_account_balance.sub(&withdraw_value),
            current_account_debt,
        )
        .unwrap();

        Ok(res)
    }

    pub fn is_account_healthy(
        env: &Env,
        total_account_balance: U256,
        total_account_debt: U256,
    ) -> Result<bool, BorrowError> {
        if total_account_debt == U256::from_u128(&env, 0) {
            return Ok(true);
        } else {
            let res = total_account_balance.div(&total_account_debt)
                > U256::from_u128(&env, BALANCE_TO_BORROW_THRESHOLD);
            return Ok(res);
        }
    }

    pub fn get_current_total_balance(
        env: &Env,
        margin_account: Address,
    ) -> Result<U256, BorrowError> {
        let collateral_token_symbols: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                margin_account.clone(),
            ))
            .unwrap_or_else(|| panic!("User doesn't have any collateral assets"));

        let mut total_account_balance: U256 = U256::from_u128(&env, 0);

        for token in collateral_token_symbols.iter() {
            let token_balance = AccountLogicContract::get_collateral_token_balance(
                &env,
                margin_account.clone(),
                token,
            )
            .unwrap();

            let oracle_token_value = 1; // Fetch oracle price feed
                                        // Todo!!!
            total_account_balance = total_account_balance
                .add(&token_balance.mul(&U256::from_u128(&env, oracle_token_value)));
            // token_balance * token_value in usd
        }
        Ok(total_account_balance)
    }

    pub fn get_current_total_borrows(
        env: &Env,
        margin_account: Address,
    ) -> Result<U256, BorrowError> {
        let borrowed_token_symbols =
            AccountLogicContract::get_all_borrowed_tokens(&env, margin_account.clone()).unwrap();

        let mut total_account_debt: U256 = U256::from_u128(&env, 0);

        for tokenx in borrowed_token_symbols.iter() {
            let token_balance =
                AccountLogicContract::get_borrowed_token_debt(&env, margin_account.clone(), tokenx)
                    .unwrap();

            let oracle_token_value: U256 = U256::from_u128(&env, 1);
            total_account_debt = total_account_debt.add(&token_balance.mul(&oracle_token_value));
        }
        Ok(total_account_debt)
    }
    /// For future integration of trading
    pub fn approve(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        Ok(())
    }
}
