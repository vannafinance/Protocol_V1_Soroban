use soroban_sdk::{contract, Env, Symbol, Vec, U256};

use crate::{
    borrowing_protocol::borrow_logic::BorrowLogicContract, errors::InterestRateError,
    margin_account::account_logic::AccountLogicContract, types::DataKey,
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const TLL_LEDGERS_MONTH: u32 = 518400;

const C1: u128 = 100000000000000000;
const C2: u128 = 3 * 100000000000000000;
const C3: u128 = 35 * 100000000000000000;
const SECS_PER_YEAR: u128 = 31556952 * 1000000000000000000;

#[contract]
pub struct InterestRateContract;

impl InterestRateContract {
    // pub fn initialise_interest_rate(env: &Env) -> Result<(), InterestRateError> {
    //     Ok(())
    // }

    // pub fn get_rate_factor(env: &Env, token: Symbol) -> Result<u128, InterestRateError> {
    //     let lastupdatetime = BorrowLogicContract::get_last_updated_time(&env, token.clone());
    //     let blocktimestamp = env.ledger().timestamp();
    //     if lastupdatetime == env.ledger().timestamp() {
    //         return Ok(0);
    //     }

    //     let borrows = AccountLogicContract::get_total_debt_in_pool(&env, token.clone());
    //     let liquidity = AccountLogicContract::get_total_liquidity_in_pool(&env, token.clone());

    //     U256::from_u128(&env, (blocktimestamp - lastupdatetime) as u128)
    //         .mul(&(Self::get_borrow_rate_per_sec(&env, liquidity, borrows).unwrap()));

    //     Ok(0)
    // }

    pub fn get_borrow_rate_per_sec(
        env: &Env,
        liquidity: U256,
        borrows: U256,
    ) -> Result<U256, InterestRateError> {
        let util = Self::get_utilisation_ratio(env, liquidity, borrows)
            .expect("Panicked to get utilization ratio");
        let c1_u256 = U256::from_u128(&env, C1);
        let c2_u256 = U256::from_u128(&env, C2);
        let c3_u256 = U256::from_u128(&env, C3);

        let x = (util.pow(32)).mul(&c1_u256);
        let y = (util.pow(64)).mul(&c2_u256);
        let rhs = util.mul(&c1_u256).add(&(x.add(&y)));
        let result = c3_u256.mul(&rhs);

        Ok(result)
    }
    pub fn get_utilisation_ratio(
        env: &Env,
        liquidity: U256,
        borrows: U256,
    ) -> Result<U256, InterestRateError> {
        let total_assets = liquidity.add(&borrows);

        if total_assets == U256::from_u128(&env, 0) {
            Ok(U256::from_u128(&env, 0))
        } else {
            Ok(borrows.div(&total_assets))
        }
    }
}
