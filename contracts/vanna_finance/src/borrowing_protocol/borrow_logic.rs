use soroban_sdk::{
    contract, contractimpl, log, panic_with_error, token, Address, Env, Symbol, Vec, U256,
};

use crate::{
    borrowing_protocol::oracle::PriceConsumerContract,
    errors::{BorrowError, LendingError, MarginAccountError},
    lending_protocol::{
        liquidity_pool_eurc::LiquidityPoolEURC, liquidity_pool_usdc::LiquidityPoolUSDC,
        liquidity_pool_xlm::LiquidityPoolXLM,
    },
    margin_account::account_logic::AccountLogicContract,
    types::{DataKey, MarginAccountDataKey, PoolDataKey, TokenDataKey},
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const TLL_LEDGERS_MONTH: u32 = 518400;
const BALANCE_TO_BORROW_THRESHOLD: u128 = 1100000000000000000;
const DECIMALS: u128 = 1000000000000000000;

#[contract]
pub struct BorrowLogicContract;

impl BorrowLogicContract {
    pub fn borrow(
        env: &Env,
        borrow_amount: U256,
        token_symbol: Symbol,
        margin_account: Address,
    ) -> Result<(), BorrowError> {
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

        let xlm_symbol = Symbol::new(&env, "XLM");
        let usdc_symbol = Symbol::new(&env, "USDC");
        let eurc_symbol = Symbol::new(&env, "EURC");
        let client_address: Address;
        let pool_address: Address;

        if token_symbol.clone().eq(&xlm_symbol) {
            client_address = LiquidityPoolXLM::get_native_xlm_client_address(&env);
            pool_address = LiquidityPoolXLM::get_xlm_pool_address(&env);
        } else if token_symbol.clone().eq(&usdc_symbol) {
            client_address = LiquidityPoolUSDC::get_usdc_client_address(&env);
            pool_address = LiquidityPoolUSDC::get_usdc_pool_address(&env);
        } else if token_symbol.clone().eq(&eurc_symbol) {
            client_address = LiquidityPoolEURC::get_eurc_client_address(&env);
            pool_address = LiquidityPoolEURC::get_eurc_pool_address(&env);
        } else {
            panic!("Pool doesn't exist for this token to repay");
        }

        // Allow user to borrow
        // Transfer borrow amount from pool to user
        let borrow_amount_u128 = borrow_amount
            .to_u128()
            .unwrap_or_else(|| panic_with_error!(&env, LendingError::IntegerConversionError));
        let token_client = token::Client::new(&env, &client_address);

        token_client.transfer(
            &pool_address,   // from
            &margin_account, // to
            &(borrow_amount_u128 as i128),
        );

        let new_pool_balance = pool_balance.sub(&borrow_amount);
        env.storage()
            .persistent()
            .set(&PoolDataKey::Pool(token_symbol.clone()), &new_pool_balance);

        AccountLogicContract::add_borrowed_token_balance(
            &env,
            margin_account.clone(),
            token_symbol,
            borrow_amount,
        )
        .unwrap();

        Ok(())
    }

    pub fn repay(
        env: Env,
        repay_amount: U256,
        token_symbol: Symbol,
        margin_account: Address,
    ) -> Result<(), BorrowError> {
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
        let xlm_symbol = Symbol::new(&env, "XLM");
        let usdc_symbol = Symbol::new(&env, "USDC");
        let eurc_symbol = Symbol::new(&env, "EURC");
        let client_address: Address;
        let pool_address: Address;

        if token_symbol.clone().eq(&xlm_symbol) {
            client_address = LiquidityPoolXLM::get_native_xlm_client_address(&env);
            pool_address = LiquidityPoolXLM::get_xlm_pool_address(&env);
        } else if token_symbol.clone().eq(&usdc_symbol) {
            client_address = LiquidityPoolUSDC::get_usdc_client_address(&env);
            pool_address = LiquidityPoolUSDC::get_usdc_pool_address(&env);
        } else if token_symbol.clone().eq(&eurc_symbol) {
            client_address = LiquidityPoolEURC::get_eurc_client_address(&env);
            pool_address = LiquidityPoolEURC::get_eurc_pool_address(&env);
        } else {
            panic!("Pool doesn't exist for this token to repay");
        }

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
                margin_account,
                token_symbol.clone(),
                repay_amount,
            )
            .unwrap();
        }

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
        //  Fetch price from oracle !!!!!!!!!!!!!!!!!!!!!!!!
        let price = PriceConsumerContract::get_price_of(env, (symbol, Symbol::new(&env, "USD")));
        let oracle_price = U256::from_u128(&env, price);
        let borrow_value = borrow_amount.mul(&oracle_price);

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
        if !AccountLogicContract::has_debt(&env, margin_account.clone()) {
            return Ok(true);
        }

        //  Fetch price from oracle !!!!!!!!!!!!!!!!!!!!!!!!
        let price = PriceConsumerContract::get_price_of(env, (symbol, Symbol::new(&env, "USD")));
        let oracle_price = U256::from_u128(&env, price);
        let withdraw_value = withdraw_amount.mul(&oracle_price);

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
                token.clone(),
            )
            .unwrap();

            let oracle_price_usd =
                PriceConsumerContract::get_price_of(&env, (token, Symbol::new(&env, "USD")));

            total_account_balance = total_account_balance
                .add(&token_balance.mul(&U256::from_u128(&env, oracle_price_usd)));
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
            let token_balance = AccountLogicContract::get_borrowed_token_debt(
                &env,
                margin_account.clone(),
                tokenx.clone(),
            )
            .unwrap();

            let oracle_price_usd =
                PriceConsumerContract::get_price_of(&env, (tokenx, Symbol::new(&env, "USD")));

            total_account_debt = total_account_debt
                .add(&token_balance.mul(&U256::from_u128(&env, oracle_price_usd)));
        }
        Ok(total_account_debt)
    }
    /// For future integration of trading
    pub fn approve(env: Env, margin_account: Address) -> Result<(), BorrowError> {
        Ok(())
    }
}
