use soroban_sdk::{contract, contractimpl, log, panic_with_error, Address, Env, Symbol, Vec, U256};

use crate::{
    errors::{BorrowError, MarginAccountError},
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
        env: Env,
        amount: u64,
        symbol: Symbol,
        margin_account: Address,
    ) -> Result<(), BorrowError> {
        let pool_balance: U256 = env
            .storage()
            .persistent()
            .get(&PoolDataKey::Pool(symbol))
            .unwrap_or_else(|| panic!("Pool doesn't exist"));

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
        env: Env,
        symbol: Symbol,
        amount: u64,
        margin_account: Address,
    ) -> Result<bool, BorrowError> {
        Ok(false)
    }

    pub fn is_withdraw_allowed(
        env: Env,
        symbol: Symbol,
        amount: u64,
        margin_account: Address,
    ) -> Result<bool, BorrowError> {
        Ok(false)
    }

    pub fn is_account_healthy(env: Env, margin_account: Address) -> Result<bool, BorrowError> {
        let collateral_token_symbols: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                margin_account.clone(),
            ))
            .unwrap_or_else(|| panic!("User doesn't have any collateral assets"));

        for token in collateral_token_symbols.iter() {
            let token_balance = env
                .storage()
                .persistent()
                .get(&MarginAccountDataKey::UserCollateralBalance(
                    margin_account.clone(),
                    token.clone(),
                ))
                .unwrap_or_else(|| {
                    panic!(
                        "User doesn't have collateral balance for this token {:?}",
                        token
                    )
                });
        }

        Ok(false)
    }

    /// For future integration of trading
    pub fn approve(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        Ok(())
    }
}
