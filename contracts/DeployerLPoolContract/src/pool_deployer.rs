use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Env, Symbol, TryIntoVal, Vec, contract, log};

use soroban_sdk::Bytes;
use soroban_sdk::{BytesN, contractimpl, symbol_short};

// const LENDING_POOL_XLM_WASM: &[u8] =
//     include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_xlm.wasm");

// const _LENDING_POOL_USDC_WASM: &[u8] =
//     include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_usdc.wasm");

// const _LENDING_POOL_EURC_WASM: &[u8] =
//     include_bytes!("../../../target/wasm32v1-none/release/lending_protocol_eurc.wasm");

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

    pub fn deploy_lps_and_token_contracts(
        env: Env,
        registry_address: Address,
        account_manager: Address,
        rate_model: Address,
        lending_pool_xlm_hash: BytesN<32>,
        vxlm_contract_hash: BytesN<32>,
        vusdc_contract_hash: BytesN<32>,
        veurc_contract_hash: BytesN<32>,
    ) {
        let native_xlm_token_address = Address::from_str(&env, XLM_CONTRACT_ADDRESS_TESTNET);
        // let vxlm_token_address = Address::from_str(&env, VXLM_CONTRACT_ADDRESS_TESTNET);
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
            // vxlm_token_address,
            registry_address,
            account_manager,
            rate_model,
            token_issuer,
            lending_pool_xlm_hash,
        );
        log!(&env, "Deployed xlm pool contract: {}", xlm_pool_contract);

        let vxlm_token_contract_address =
            Self::deploy_vtoken_contracts(&env, admin, vxlm_contract_hash);

        log!(
            &env,
            "Deployed vxlm token contract : {}",
            vxlm_token_contract_address
        );

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
        // vxlm_token_address: Address,
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
            &env.current_contract_address(),
            lending_pool_xlm_hash.clone(),
        );

        // Convert all constructor arguments to Val and add to vector
        let mut constructor_args = Vec::new(&env);
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(native_token_address.to_val());
        // constructor_args.push_back(vxlm_token_address.to_val());
        constructor_args.push_back(registry_contract.to_val());
        constructor_args.push_back(account_manager.to_val());
        constructor_args.push_back(rate_model.to_val());
        constructor_args.push_back(token_issuer.to_val());
        constructor_args.push_back(admin.to_val());
        constructor_args.push_back(1000.try_into_val(env).unwrap());

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

    pub fn deploy_vtoken_contracts(
        env: &Env,
        admin: Address,
        token_contract_hash: BytesN<32>,
    ) -> Address {
        let salt = Self::generate_predictable_salt(
            &env,
            &admin,
            &env.current_contract_address(),
            token_contract_hash.clone(),
        );

        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(token_contract_hash, ());

        deployed_address
    }

    /// Generates a unique, predictable salt for contract deployment
    /// Uses cryptographic hashing to ensure uniqueness for each unique combination
    /// of admin_address, deployer_address, and hash
    fn generate_predictable_salt(
        env: &Env,
        admin_address: &Address,
        deployer_address: &Address,
        hash: BytesN<32>,
    ) -> BytesN<32> {
        // Convert addresses to XDR for consistent serialization
        let admin_xdr = admin_address.to_xdr(env);
        let deployer_xdr = deployer_address.to_xdr(env);
        let hash_xdr = hash.to_xdr(env);

        // Create a combined buffer to hash all inputs together
        let mut combined = Bytes::new(env);

        // Append admin address bytes
        for i in 0..admin_xdr.len() {
            combined.push_back(admin_xdr.get(i).unwrap());
        }

        // Append deployer address bytes
        for i in 0..deployer_xdr.len() {
            combined.push_back(deployer_xdr.get(i).unwrap());
        }

        // Append hash bytes
        for i in 0..hash_xdr.len() {
            combined.push_back(hash_xdr.get(i).unwrap());
        }

        // Use Soroban's built-in SHA256 hash function
        // This ensures a unique 32-byte output for any unique input combination
        env.crypto().sha256(&combined).into()
    }
}
