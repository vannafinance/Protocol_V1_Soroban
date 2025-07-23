use soroban_sdk::{contract, contractimpl, log, panic_with_error, Address, Env, Symbol, Vec};

use crate::{errors::InterestRateError, types::DataKey};

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
    pub fn initialise_interest_rate(env: Env) -> Result<(), InterestRateError> {
        Ok(())
    }

    pub fn get_borrow_rate_per_sec(env: Env) -> Result<(), InterestRateError> {
        Ok(())
    }
    pub fn get_utilisation_ratio(env: Env) -> Result<(), InterestRateError> {
        Ok(())
    }
}
