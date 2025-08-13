use crate::reflector::{Asset as ReflectorAsset, ReflectorClient};
use crate::types::OracleDataKey;
use soroban_sdk::{Address, Env, Symbol, contract, contractimpl}; // Import Reflector interface

// pub mod std_reference {
//     // soroban_sdk::contractimport!(file = "../../dist/std_reference.wasm");
//     soroban_sdk::contractimport!(file = "../../BandOracle/std_reference.wasm");
// }

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

// Just for future reference
const _TESTNET_REFLECTOR_ADDRESS: &str = "CCYOZJCOPG34LLQQ7N24YXBM7LL62R7ONMZ3G6WZAAYPB5OYKOMJRN63";
const _MAINNET_REFLECTOR_ADDRESS: &str = "CAFJZQWSED6YAWZU3GWRTOCNPPCGBN32L7QV43XX5LZLFTK6JLN34DLN";

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    pub fn __constructor(env: &Env, admin: Address, reflector_address: Address) {
        env.storage()
            .persistent()
            .set(&OracleDataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&OracleDataKey::ReflectorAddress, &reflector_address);
        Self::extend_ttl(&env, OracleDataKey::Admin);
        Self::extend_ttl(&env, OracleDataKey::ReflectorAddress);
    }

    // pub fn set_std_reference_address(env: &Env, std_reference_address: Address) {
    //     let admin_address: Address = env
    //         .storage()
    //         .persistent()
    //         .get(&OracleDataKey::Admin)
    //         .unwrap_or_else(|| panic!("Admin key has not been set"));
    //     admin_address.require_auth();
    //     env.storage()
    //         .persistent()
    //         .set(&OracleDataKey::StdReferenceAddress, &std_reference_address);
    // }

    // pub fn get_price_of(env: &Env, symbol_pair: (Symbol, Symbol)) -> u128 {
    //     let addr = env
    //         .storage()
    //         .persistent()
    //         .get(&OracleDataKey::StdReferenceAddress)
    //         .unwrap();
    //     let client = std_reference::Client::new(&env, &addr);
    //     client
    //         .get_reference_data(&Vec::from_array(&env, [symbol_pair]))
    //         .get_unchecked(0)
    //         .rate
    // }

    pub fn get_price_latest(env: &Env, symbol: Symbol) -> u128 {
        let reflector_address: Address = env
            .storage()
            .persistent()
            .get(&OracleDataKey::ReflectorAddress)
            .unwrap();

        // let oracle_address = Address::from_str(&env, TESTNET_REFLECTOR_ADDRESS);
        let reflector_client = ReflectorClient::new(&env, &reflector_address);
        let ticker = ReflectorAsset::Other(symbol);
        let recent = reflector_client.lastprice(&ticker);

        if recent.is_none() {
            panic!("price not available");
        }

        let price = recent.unwrap().price;

        // Do not forget for price precision, get decimals from the oracle
        // (this value can be also hardcoded once the price feed has been
        // selected because decimals never change in live oracles)
        let price_decimals = reflector_client.decimals();

        let final_price = price / 10i128.pow(price_decimals);

        final_price as u128
    }

    fn extend_ttl(env: &Env, key: OracleDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
