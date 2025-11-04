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

    pub fn get_price_latest(env: &Env, symbol: Symbol) -> (u128, u32) {
        #[cfg(not(feature = "testutils"))]
        {
            use soroban_sdk::log;

            log!(&env, "Entered NON TEST mode!!!");

            let reflector_address: Address = env
                .storage()
                .persistent()
                .get(&OracleDataKey::ReflectorAddress)
                .unwrap();

            let reflector_client = ReflectorClient::new(&env, &reflector_address);

            let ticker = ReflectorAsset::Other(symbol.clone());

            let recent = reflector_client.lastprice(&ticker);

            if recent.is_none() {
                panic!("price not available");
            }

            let price = recent.unwrap().price as u128;
            let decimals = reflector_client.decimals();
            log!(
                &env,
                "Price for symbol NON TEST ",
                symbol,
                "is",
                price,
                "decimals",
                decimals
            );
            (price, decimals)
        }

        #[cfg(feature = "testutils")]
        {
            use sep_40_oracle::testutils::Asset;
            use sep_40_oracle::testutils::MockPriceOracleClient;
            use soroban_sdk::log;

            log!(&env, "Entered test mode!!!");
            let reflector_address: Address = env
                .storage()
                .persistent()
                .get(&OracleDataKey::ReflectorAddress)
                .unwrap();

            let test_client = MockPriceOracleClient::new(env, &reflector_address);
            let recent = test_client.lastprice(&Asset::Other(symbol));

            if recent.is_none() {
                panic!("price not available");
            }

            let price = recent.unwrap().price as u128;
            let decimals = reflector_client.decimals();
            log!(
                &env,
                "Price for symbol",
                symbol,
                "is",
                price,
                "decimals",
                decimals
            );
            (price, decimals)
        }
    }

    fn extend_ttl(env: &Env, key: OracleDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
