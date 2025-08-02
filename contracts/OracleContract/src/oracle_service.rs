use crate::types::OracleDataKey;
use soroban_sdk::{Address, Env, Symbol, Vec, contract, contractimpl};

pub mod std_reference {
    // soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
    soroban_sdk::contractimport!(file = "../../BandOracle/std_reference.wasm");
}

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    pub fn __constructor(env: &Env, admin: Address) {
        env.storage()
            .persistent()
            .set(&OracleDataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&OracleDataKey::IsInitialised, &true);
        Self::extend_ttl(&env, OracleDataKey::Admin);
        Self::extend_ttl(&env, OracleDataKey::IsInitialised);
    }

    pub fn set_std_reference_address(env: &Env, std_reference_address: Address) {
        let admin_address: Address = env
            .storage()
            .persistent()
            .get(&OracleDataKey::Admin)
            .unwrap_or_else(|| panic!("Admin key has not been set"));
        admin_address.require_auth();
        env.storage()
            .persistent()
            .set(&OracleDataKey::StdReferenceAddress, &std_reference_address);
    }

    pub fn get_price_of(env: &Env, symbol_pair: (Symbol, Symbol)) -> u128 {
        let addr = env
            .storage()
            .persistent()
            .get(&OracleDataKey::StdReferenceAddress)
            .unwrap();
        let client = std_reference::Client::new(&env, &addr);
        client
            .get_reference_data(&Vec::from_array(&env, [symbol_pair]))
            .get_unchecked(0)
            .rate
    }

    fn extend_ttl(env: &Env, key: OracleDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
