use soroban_sdk::{Address, Env, U256, contract, contracterror, contractimpl, contracttype};

#[contract]
pub struct RateModelContract;

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum InterestRateError {
    InterestRateNotInitialized = 1,
}

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum RateModelKey {
    RegistryContract,
    Admin,
    IsInitialised,
}

const C1: u128 = 10000000;
const C2: u128 = 3 * 10000000;
const C3: u128 = 35 * 10000000;
const SECS_PER_YEAR: u128 = 31556952 * 10000000;

#[contractimpl]
impl RateModelContract {
    pub fn __constructor(env: &Env, admin: Address, registry_contract: Address) {
        env.storage().persistent().set(&RateModelKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&RateModelKey::RegistryContract, &registry_contract);
        env.storage()
            .persistent()
            .set(&RateModelKey::IsInitialised, &true);
        Self::extend_ttl(&env, RateModelKey::Admin);
        Self::extend_ttl(&env, RateModelKey::RegistryContract);
        Self::extend_ttl(&env, RateModelKey::IsInitialised);
    }

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

    fn extend_ttl(env: &Env, key: RateModelKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
