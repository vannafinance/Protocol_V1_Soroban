#![no_std]

use crate::types::DataKey;
use soroban_sdk::{Address, Env, Symbol, Vec, contract, contractimpl};

pub mod std_reference {
    // soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
    soroban_sdk::contractimport!(file = "../../BandOracle/std_reference.wasm");
}

// pub trait StandardReferenceTrait {
//     fn set_std_reference_address(env: Env, std_reference_address: Address);
//     fn get_price_of(env: Env, symbol_pairs: Vec<(Symbol, Symbol)>) -> u32;
// }

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    pub fn set_std_reference_address(env: &Env, std_reference_address: Address) {
        let admin_address: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin key has not been set"));
        admin_address.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::StdReferenceAddress, &std_reference_address);
    }

    pub fn get_price_of(env: &Env, symbol_pair: (Symbol, Symbol)) -> u128 {
        let addr = env
            .storage()
            .persistent()
            .get(&DataKey::StdReferenceAddress)
            .unwrap();
        let client = std_reference::Client::new(&env, &addr);
        client
            .get_reference_data(&Vec::from_array(&env, [symbol_pair]))
            .get_unchecked(0)
            .rate
    }
}
