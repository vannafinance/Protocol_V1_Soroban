use soroban_sdk::{Env, U256, contract, contracterror};

#[contract]
pub struct RateModelContract;

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum InterestRateError {
    InterestRateNotInitialized = 1,
}

const C1: u128 = 100000000000000000;
const C2: u128 = 3 * 100000000000000000;
const C3: u128 = 35 * 100000000000000000;
const SECS_PER_YEAR: u128 = 31556952 * 1000000000000000000;

impl RateModelContract {
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
        let secs_per_year = U256::from_u128(&env, SECS_PER_YEAR);

        let x = (util.pow(32)).mul(&c1_u256);
        let y = (util.pow(64)).mul(&c2_u256);
        let rhs = util.mul(&c1_u256).add(&(x.add(&y)));
        let result = c3_u256.mul(&rhs);
        let res = result.div(&secs_per_year);

        Ok(res)
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
