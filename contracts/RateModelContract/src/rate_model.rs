use soroban_sdk::{Address, Env, U256, contract, contracterror, contractimpl, contracttype, log};

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

// ---------- Fixed-point (WAD) constants ----------
const WAD_U128: u128 = 10000_0000_00000_00000; // 1e18
const WAD_17_U128: u128 = 1000_0000_00000_00000; // 1e17
const C1_U128: u128 = WAD_17_U128; // 1e17
const C2_U128: u128 = 3 * WAD_17_U128; // 3e1
const C3_U128: u128 = 35 * WAD_17_U128; // 35e17 Keep identical.
const SECS_PER_YEAR_U128: u128 = 31_556_952 * WAD_U128; // 31_556_952 * 1e18

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

    // Borrow Rate Per Second (WAD):
    // c3 * (util*c1 + (util^32)*c1 + (util^64)*c2) / secsPerYear
    // where util = borrows / (liquidity + borrows), all in WAD.
    pub fn get_borrow_rate_per_sec(
        env: &Env,
        liquidity_wad: U256,
        borrows_wad: U256,
    ) -> Result<U256, InterestRateError> {
        let util = Self::get_utilisation_ratio(&env, liquidity_wad, borrows_wad)?;
        log!(&env, "util (WAD)", util);

        let c1 = u256(&env, C1_U128);
        let c2 = u256(&env, C2_U128);
        let c3 = u256(&env, C3_U128);
        let secs_per_year = u256(&env, SECS_PER_YEAR_U128);

        // util*c1
        let term1 = mul_wad_down(&env, &util, &c1);

        // util^32 * c1
        let u32w = rpow_wad(&env, &util, 32);
        let term2 = mul_wad_down(&env, &u32w, &c1);

        // util^64 * c2
        let u64w = rpow_wad(&env, &util, 64);
        let term3 = mul_wad_down(&env, &u64w, &c2);

        let sum = term1.add(&term2).add(&term3); // WAD

        // c3.mulDivDown(sum, secsPerYear)
        let numerator = c3.mul(&sum); // WAD * WAD = WAD^2
        let rate = numerator.div(&secs_per_year); // (WAD^2) / WAD = WAD

        log!(&env, "borrow_rate_per_sec (WAD)", rate);
        Ok(rate)
    }

    // util = borrows.divWadDown(liquidity + borrows), WAD-scaled
    pub fn get_utilisation_ratio(
        env: &Env,
        liquidity_wad: U256,
        borrows_wad: U256,
    ) -> Result<U256, InterestRateError> {
        log!(
            &env,
            "liquidity_wad, borrows_wad in util",
            liquidity_wad,
            borrows_wad
        );
        let total_assets_wad = liquidity_wad.add(&borrows_wad);
        if is_zero(env, &total_assets_wad) {
            return Ok(U256::from_u32(env, 0));
        }
        log!(
            &env,
            "Borrows wad, total assets _wad",
            borrows_wad,
            total_assets_wad
        );
        let util = div_wad_down(&env, &borrows_wad, &total_assets_wad);
        log!(&env, "returning util", util);
        Ok(util)
    }

    fn extend_ttl(env: &Env, key: RateModelKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}

fn u256(env: &Env, v: u128) -> U256 {
    U256::from_u128(env, v)
}

fn is_zero(env: &Env, x: &U256) -> bool {
    *x == U256::from_u32(&env, 0)
}

// a.mulWadDown(b) = floor(a * b / WAD)
fn mul_wad_down(env: &Env, a: &U256, b: &U256) -> U256 {
    a.mul(b).div(&u256(env, WAD_U128))
}

// a.divWadDown(b) = floor(a * WAD / b)
fn div_wad_down(env: &Env, a: &U256, b: &U256) -> U256 {
    if is_zero(env, b) {
        return U256::from_u32(&env, 0);
    }
    a.mul(&u256(env, WAD_U128)).div(b)
}

// Exponentiation by squaring in WAD space.
// Returns x^n with WAD scaling preserved (i.e., result is WAD).
fn rpow_wad(env: &Env, x: &U256, mut n: u128) -> U256 {
    let mut base = x.clone();
    let mut result = u256(env, WAD_U128); // 1.0 in WAD

    while n > 0 {
        if (n & 1) == 1 {
            result = mul_wad_down(env, &result, &base);
        }
        if n > 1 {
            base = mul_wad_down(env, &base, &base);
        }
        n >>= 1;
    }
    result
}
