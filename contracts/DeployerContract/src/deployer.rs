use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Env, Symbol, Vec, contract, log};

/// This example demonstrates the 'factory' pattern for programmatically
/// deploying the contracts via `env.deployer()`.
use soroban_sdk::{BytesN, contractimpl, symbol_short};

const REGISTRY_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/registry_contract.wasm");

const RATE_MODEL_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/rate_model_contract.wasm");

const RISK_ENGINE_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/risk_engine_contract.wasm");

const _SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/smart_account_contract.wasm");

const ORACLE_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/oracle_contract.wasm");

const LENDING_POOL_XLM_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_xlm.wasm");

const _LENDING_POOL_USDC_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_usdc.wasm");

const _LENDING_POOL_EURC_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_eurc.wasm");

const ACCOUNT_MANAGER_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/account_manager_contract.wasm");

pub mod registry_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/registry_contract.wasm"
    );
}

const XLM_CONTRACT_ADDRESS_TESTNET: &str =
    "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

const VXLM_CONTRACT_ADDRESS_TESTNET: &str =
    "CDQACBSGEHOSLLEDFGQKSUSDY3M6NTAEXV623L6UPHXECNFZO65E74V2";

const VXLM_TOKEN_ISSUER_TESTNET: &str = "GBKTBXQK3FD7W3RRFL4CQE56WBDJF27HQPHG37CONO2MDKPDTTV4YUYG";

const _XLM_CONTRACT_ADDRESS_MAINNET: &str =
    "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA";
#[contract]
pub struct Deployer;

const ADMIN: Symbol = symbol_short!("admin");

#[contractimpl]
impl Deployer {
    /// Construct the deployer with a provided administrator.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&ADMIN, &admin);
    }

    pub fn deploy_all(
        env: Env,
        // net: Symbol
    ) {
        let native_xlm_token_address = Address::from_str(&env, XLM_CONTRACT_ADDRESS_TESTNET);
        let vxlm_token_address = Address::from_str(&env, VXLM_CONTRACT_ADDRESS_TESTNET);
        let token_issuer = Address::from_str(&env, VXLM_TOKEN_ISSUER_TESTNET);

        // if net == Symbol::new(&env, "testnet") {
        //     native_xlm_token_address = Address::from_str(&env, XLM_CONTRACT_ADDRESS_TESTNET);
        //     vxlm_token_address = Address::from_str(&env, VXLM_CONTRACT_ADDRESS_TESTNET);
        // } else if net == Symbol::new(&env, "mainnet") {
        //     native_xlm_token_address = Address::from_str(&env, XLM_CONTRACT_ADDRESS_MAINNET);
        // } else {
        //     native_xlm_token_address = Address::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        // }

        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let registry_address = Self::deploy_registry_contract(&env);
        log!(&env, "Deployed registry contract: {}", registry_address);
        let rate_model = Self::deploy_rate_model_contract(&env, registry_address.clone());
        log!(&env, "Deployed rate model contract: {}", rate_model);
        let risk_engine = Self::deploy_risk_engine_contract(&env, registry_address.clone());
        log!(&env, "Deployed risk engine contract: {}", risk_engine);
        let oracle = Self::deploy_oracle_contract(&env);
        log!(&env, "Deployed oracle contract: {}", oracle);
        // let smart_account_hash = Self::upload_smart_account(&env);
        // let registry_client = registry_contract::Client::new(&env, &registry_address);
        // registry_client.set_smart_account_hash(&smart_account_hash);
        let account_manager_contract =
            Self::deploy_account_manager_contract(&env, registry_address.clone());
        log!(
            &env,
            "Deployed account manager contract: {}",
            account_manager_contract
        );
        let xlm_pool_contract = Self::deploy_xlm_pool(
            &env,
            native_xlm_token_address,
            vxlm_token_address,
            registry_address,
            account_manager_contract,
            rate_model,
            token_issuer,
        );
        log!(&env, "Deployed xlm pool contract: {}", xlm_pool_contract);

        // let usdc_pool_contract = Self::deploy_usdc_pool(
        //     &env,
        //     usdc_token_address,
        //     vusdc_token_address,
        //     registry_address,
        //     account_manager_contract,
        //     rate_model,
        //     token_issuer,
        // );

        // let eurc_pool_contract = Self::deploy_eurc_pool(
        //     &env,
        //     eurc_token_address,
        //     veurc_token_address,
        //     registry_address,
        //     account_manager_contract,
        //     rate_model,
        //     token_issuer,
        // );
    }

    // pub fn upload_smart_account(env: &Env) -> BytesN<32> {
    //     let smart_contract_wasm_hash: BytesN<32> =
    //         env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
    //     smart_contract_wasm_hash
    // }

    pub fn deploy_registry_contract(env: &Env) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt = Self::generate_predictable_salt(&env, &admin, &env.current_contract_address());

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());

        let registry_contract_wasm_hash = env.deployer().upload_contract_wasm(REGISTRY_WASM);

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(registry_contract_wasm_hash, constructor_args);
        deployed_address
    }

    pub fn deploy_risk_engine_contract(env: &Env, registry: Address) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt = Self::generate_predictable_salt(&env, &admin, &env.current_contract_address());

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(registry.to_val());

        let risk_engine_wasm_hash = env.deployer().upload_contract_wasm(RISK_ENGINE_WASM);

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(risk_engine_wasm_hash, constructor_args);
        deployed_address
    }

    pub fn deploy_rate_model_contract(env: &Env, registry: Address) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt = Self::generate_predictable_salt(&env, &admin, &env.current_contract_address());

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(registry.to_val());

        let rate_model_wasm_hash = env.deployer().upload_contract_wasm(RATE_MODEL_WASM);

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(rate_model_wasm_hash, constructor_args);
        deployed_address
    }

    pub fn deploy_oracle_contract(env: &Env) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt = Self::generate_predictable_salt(&env, &admin, &env.current_contract_address());
        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());

        let oracle_wasm_hash = env.deployer().upload_contract_wasm(ORACLE_WASM);

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(oracle_wasm_hash, constructor_args);
        deployed_address
    }

    pub fn deploy_account_manager_contract(env: &Env, registry: Address) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt = Self::generate_predictable_salt(&env, &admin, &env.current_contract_address());

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(registry.to_val());

        let account_manager_wasm_hash = env.deployer().upload_contract_wasm(ACCOUNT_MANAGER_WASM);

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(account_manager_wasm_hash, constructor_args);
        deployed_address
    }

    /// Deploys the contract on behalf of the `Deployer` contract.
    ///
    /// This has to be authorized by the `Deployer`s administrator.    
    pub fn deploy_xlm_pool(
        env: &Env,
        native_token_address: Address,
        vxlm_token_address: Address,
        registry_contract: Address,
        account_manager: Address,
        rate_model: Address,
        token_issuer: Address,
    ) -> Address {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let salt =
            Self::generate_predictable_salt(&env, &native_token_address, &vxlm_token_address);

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(native_token_address.to_val());
        constructor_args.push_back(vxlm_token_address.to_val());
        constructor_args.push_back(registry_contract.to_val());
        constructor_args.push_back(account_manager.to_val());
        constructor_args.push_back(rate_model.to_val());
        constructor_args.push_back(token_issuer.to_val());

        let lending_pool_xlm_wasm_hash = env.deployer().upload_contract_wasm(LENDING_POOL_XLM_WASM);

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

    // pub fn deploy_usdc_pool(
    //     env: &Env,
    //     native_token_address: Address,
    //     vusdc_token_address: Address,
    //     registry_contract: Address,
    //     account_manager: Address,
    //     rate_model: Address,
    //     token_issuer: Address,
    // ) -> Address {
    //     let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
    //     admin.require_auth();

    //     let salt =
    //         Self::generate_predictable_salt(&env, &native_token_address, &vusdc_token_address);

    //     // Convert all constructor arguments to Val and add to vector
    //     let mut constructor_args = Vec::new(&env);
    //     constructor_args.push_back(admin.to_val());
    //     constructor_args.push_back(native_token_address.to_val());
    //     constructor_args.push_back(vusdc_token_address.to_val());
    //     constructor_args.push_back(registry_contract.to_val());
    //     constructor_args.push_back(account_manager.to_val());
    //     constructor_args.push_back(rate_model.to_val());
    //     constructor_args.push_back(token_issuer.to_val());

    //     let lending_pool_usdc_wasm_hash =
    //         env.deployer().upload_contract_wasm(LENDING_POOL_USDC_WASM);

    //     // Deploy the contract using the uploaded Wasm with given hash on behalf
    //     // of the current contract.
    //     // Note, that not deploying on behalf of the admin provides more
    //     // consistent address space for the deployer contracts - the admin could
    //     // change or it could be a completely separate contract with complex
    //     // authorization rules, but all the contracts will still be deployed
    //     // by the same `Deployer` contract address.
    //     let deployed_address = env
    //         .deployer()
    //         .with_address(env.current_contract_address(), salt)
    //         .deploy_v2(lending_pool_usdc_wasm_hash, constructor_args);
    //     deployed_address
    // }

    // pub fn deploy_eurc_pool(
    //     env: &Env,
    //     native_token_address: Address,
    //     veurc_token_address: Address,
    //     registry_contract: Address,
    //     account_manager: Address,
    //     rate_model: Address,
    //     token_issuer: Address,
    // ) -> Address {
    //     let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
    //     admin.require_auth();

    //     let salt =
    //         Self::generate_predictable_salt(&env, &native_token_address, &veurc_token_address);

    //     // Convert all constructor arguments to Val and add to vector
    //     let mut constructor_args = Vec::new(&env);
    //     constructor_args.push_back(admin.to_val());
    //     constructor_args.push_back(native_token_address.to_val());
    //     constructor_args.push_back(veurc_token_address.to_val());
    //     constructor_args.push_back(registry_contract.to_val());
    //     constructor_args.push_back(account_manager.to_val());
    //     constructor_args.push_back(rate_model.to_val());
    //     constructor_args.push_back(token_issuer.to_val());

    //     let lending_pool_eurc_wasm_hash =
    //         env.deployer().upload_contract_wasm(LENDING_POOL_EURC_WASM);

    //     // Deploy the contract using the uploaded Wasm with given hash on behalf
    //     // of the current contract.
    //     // Note, that not deploying on behalf of the admin provides more
    //     // consistent address space for the deployer contracts - the admin could
    //     // change or it could be a completely separate contract with complex
    //     // authorization rules, but all the contracts will still be deployed
    //     // by the same `Deployer` contract address.
    //     let deployed_address = env
    //         .deployer()
    //         .with_address(env.current_contract_address(), salt)
    //         .deploy_v2(lending_pool_eurc_wasm_hash, constructor_args);
    //     deployed_address
    // }

    fn generate_predictable_salt(
        env: &Env,
        native_token: &Address,
        vxlm_token: &Address,
    ) -> BytesN<32> {
        let mut salt_bytes = [0u8; 32];

        // Use hash of token addresses for deterministic salt
        let native_xdr = native_token.to_xdr(env);
        let vxlm_xdr = vxlm_token.to_xdr(env);

        // Copy first 16 bytes from each address
        let native_len = (native_xdr.len() as usize).min(16);
        let vxlm_len = (vxlm_xdr.len() as usize).min(16);

        for i in 0..native_len {
            salt_bytes[i] = native_xdr.get(i as u32).unwrap_or(0);
        }

        for i in 0..vxlm_len {
            salt_bytes[16 + i] = vxlm_xdr.get(i as u32).unwrap_or(0);
        }

        BytesN::from_array(env, &salt_bytes)
    }
}

// fn generate_salt(env: &Env) -> BytesN<32> {
//     let deploy_counter_key = String::from_str(env, "DEPLOY_COUNTER");

//     // Get current ledger timestamp
//     let timestamp = env.ledger().timestamp();

//     // Get and increment deploy counter
//     let counter: u64 = env
//         .storage()
//         .instance()
//         .get(&deploy_counter_key)
//         .unwrap_or(0);

//     env.storage()
//         .instance()
//         .set(&deploy_counter_key, &(counter + 1));

//     // Create salt from timestamp + counter
//     let mut salt_bytes = [0u8; 32];

//     // First 8 bytes: timestamp
//     salt_bytes[0..8].copy_from_slice(&timestamp.to_be_bytes());

//     // Next 8 bytes: counter
//     salt_bytes[8..16].copy_from_slice(&counter.to_be_bytes());

//     // Remaining bytes can be filled with contract address or left as zeros
//     let contract_addr = env.current_contract_address();
//     let addr_bytes = contract_addr.to_xdr(env);
//     let copy_len = (addr_bytes.len() as usize).min(16);

//     for i in 0..copy_len {
//         // i is usize (for array indexing), cast to u32 for Bytes.get()
//         salt_bytes[16 + i] = addr_bytes.get(i as u32).unwrap_or(0);
//     }

//     BytesN::from_array(env, &salt_bytes)
// }
