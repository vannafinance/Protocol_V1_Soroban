#![no_std]
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Env, String, Symbol, U256, Vec, contract, contracterror};

/// This example demonstrates the 'factory' pattern for programmatically
/// deploying the contracts via `env.deployer()`.
use soroban_sdk::{BytesN, Val, contractimpl, symbol_short};

#[contract]
pub struct Deployer;

const ADMIN: Symbol = symbol_short!("admin");

#[contractimpl]
impl Deployer {
    /// Construct the deployer with a provided administrator.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&ADMIN, &admin);
    }

    /// Deploys the contract on behalf of the `Deployer` contract.
    ///
    /// This has to be authorized by the `Deployer`s administrator.    
    pub fn deploy_xlm_pool(
        env: Env,
        lending_pool_xlm_wasm_hash: BytesN<32>,
        native_token_address: Address,
        vxlm_token_address: Address,
        registry_contract: Address,
        account_manager: Address,
        rate_model: Address,
        token_issuer: Address,
    ) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt = Self::generate_salt(&env);

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(native_token_address.to_val());
        constructor_args.push_back(vxlm_token_address.to_val());
        constructor_args.push_back(registry_contract.to_val());
        constructor_args.push_back(account_manager.to_val());
        constructor_args.push_back(rate_model.to_val());
        constructor_args.push_back(token_issuer.to_val());

        // Deploy the contract using the uploaded Wasm with given hash on behalf
        // of the current contract.
        // Note, that not deploying on behalf of the admin provides more
        // consistent address space for the deployer contracts - the admin could
        // change or it could be a completely separate contract with complex
        // authorization rules, but all the contracts will still be deployed
        // by the same `Deployer` contract address.
        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(lending_pool_xlm_wasm_hash, constructor_args);

        deployed_address
    }

    fn generate_salt(env: &Env) -> BytesN<32> {
        let deploy_counter_key = String::from_str(env, "DEPLOY_COUNTER");

        // Get current ledger timestamp
        let timestamp = env.ledger().timestamp();

        // Get and increment deploy counter
        let counter: u64 = env
            .storage()
            .instance()
            .get(&deploy_counter_key)
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&deploy_counter_key, &(counter + 1));

        // Create salt from timestamp + counter
        let mut salt_bytes = [0u8; 32];

        // First 8 bytes: timestamp
        salt_bytes[0..8].copy_from_slice(&timestamp.to_be_bytes());

        // Next 8 bytes: counter
        salt_bytes[8..16].copy_from_slice(&counter.to_be_bytes());

        // Remaining bytes can be filled with contract address or left as zeros
        let contract_addr = env.current_contract_address();
        let addr_bytes = contract_addr.to_xdr(env);
        let copy_len = (addr_bytes.len() as usize).min(16);

        for i in 0..copy_len {
            // i is usize (for array indexing), cast to u32 for Bytes.get()
            salt_bytes[16 + i] = addr_bytes.get(i as u32).unwrap_or(0);
        }

        BytesN::from_array(env, &salt_bytes)
    }
}
