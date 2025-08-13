use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Env, Symbol, Vec, contract, log};

use soroban_sdk::{BytesN, contractimpl, symbol_short};

const LENDING_POOL_XLM_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_xlm.wasm");

const _LENDING_POOL_USDC_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_usdc.wasm");

const _LENDING_POOL_EURC_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_eurc.wasm");

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
        env.storage().persistent().set(&ADMIN, &admin);
    }

    pub fn deploy_liquidity_pools(
        env: Env,
        registry_address: Address,
        account_manager: Address,
        rate_model: Address,
        lending_pool_xlm_hash: BytesN<32>,
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

        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        let xlm_pool_contract = Self::deploy_xlm_pool(
            &env,
            native_xlm_token_address,
            vxlm_token_address,
            registry_address,
            account_manager,
            rate_model,
            token_issuer,
            lending_pool_xlm_hash,
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
        // log!(&env, "Deployed usdc pool contract: {}", usdc_pool_contract);

        // let eurc_pool_contract = Self::deploy_eurc_pool(
        //     &env,
        //     eurc_token_address,
        //     veurc_token_address,
        //     registry_address,
        //     account_manager_contract,
        //     rate_model,
        //     token_issuer,
        // );
        // log!(&env, "Deployed eurc pool contract: {}", eurc_pool_contract);
    }

    /// Deploys the contract on behalf of the `Deployer` contract.
    ///
    /// This has to be authorized by the `Deployer`s administrator.    
    fn deploy_xlm_pool(
        env: &Env,
        native_token_address: Address,
        vxlm_token_address: Address,
        registry_contract: Address,
        account_manager: Address,
        rate_model: Address,
        token_issuer: Address,
        lending_pool_xlm_hash: BytesN<32>,
    ) -> Address {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();

        let salt = Self::generate_predictable_salt(
            &env,
            &native_token_address,
            &vxlm_token_address,
            lending_pool_xlm_hash.clone(),
        );

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
            .deploy_v2(lending_pool_xlm_hash, constructor_args);
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
    //     let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
    //     let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        hash: BytesN<32>,
    ) -> BytesN<32> {
        let mut salt_bytes = [0u8; 32];

        // Use hash of token addresses for deterministic salt
        let native_xdr = native_token.to_xdr(env);
        let vxlm_xdr = vxlm_token.to_xdr(env);
        let hash_xdr = hash.clone().to_xdr(env);

        // Copy first 16 bytes from each address
        let native_len = (native_xdr.len() as usize).min(8);
        let vxlm_len = (vxlm_xdr.len() as usize).min(8);
        let hash_len = (hash.len() as usize).min(16);

        for i in 0..native_len {
            salt_bytes[i] = native_xdr.get(i as u32).unwrap_or(0);
        }

        for i in 0..vxlm_len {
            salt_bytes[8 + i] = vxlm_xdr.get(i as u32).unwrap_or(0);
        }

        for i in 0..hash_len {
            salt_bytes[16 + i] = hash_xdr.get(i as u32).unwrap_or(0);
        }

        BytesN::from_array(env, &salt_bytes)
    }
}
