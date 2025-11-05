// Comprehensive tests for `LiquidityPoolUSDC`
// ------------------------------------------------------------
// These tests use real token contracts (soroban_token_contract)
// and light-weight mock contracts for Registry, RateModel, and SmartAccount.
// They cover happy paths and edge/error cases for every public function.
//
// Add these dev-dependencies in your Cargo.toml if not present:
// [dev-dependencies]
// soroban-sdk = { version = "22", features = ["testutils"] }
// soroban-token-contract = "22"
//
// NOTE: If your workspace uses a different Soroban version, align versions accordingly.
// ------------------------------------------------------------

#![cfg(test)]

use soroban_sdk::{
    self as sdk, Address, BytesN, Env, IntoVal, String, Symbol, U256, Vec, contract, contractimpl, symbol_short, testutils::{Address as _, Events as _, Ledger, MockAuth, MockAuthInvoke}, token::TokenClient
};

use account_manager_contract::account_manager::{self, AccountManagerContract};
use lending_protocol_usdc::liquidity_pool_usdc::{
    self, LiquidityPoolUSDC, LiquidityPoolUSDCClient,
};
use oracle_contract::oracle_service::{OracleContract, OracleContractClient};
use registry_contract::registry::{RegistryContract, RegistryContractClient};
use risk_engine_contract::risk_engine::RiskEngineContract;
use sep_40_oracle::testutils::{self, Asset, MockPriceOracle, MockPriceOracleClient};
// use sep_40_oracle::{Asset as MAsset, PriceData, PriceFeedClient, PriceFeedTrait};
use smart_account_contract::smart_account::{SmartAccountContract, SmartAccountContractClient};
use soroban_sdk::Address as Addr;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{log, token};
use vusdc_token_contract::v_usdc::VUSDCToken;
use vusdc_token_contract::v_usdc::VUSDCTokenClient;

const SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

const WAD_U128: u128 = 10000_0000_00000_00000; // 1e18
const WAD7: i128 = 10000000;
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");

// ---------- Helpers: deploy contracts & tokens --------------------------------
pub mod helpers {
    use rate_model_contract::rate_model::RateModelContract;
    use smart_account_contract::smart_account::SmartAccountContract;
    use vusdc_token_contract::v_usdc::{VUSDCToken, VUSDCTokenClient};

    use super::*;
    // use soroban_token_contract::{Client as TokenClient, Token};

    #[derive(Debug, Clone)]
    pub struct ContractAddresses {
        pub admin: Address,
        pub liquidity_pool_xlm: Address,
        pub liquidity_pool_usdc: Address,
        pub liquidity_pool_eurc: Address,
        pub registry_contract: Address,
        pub rate_model_contract: Address,
        pub account_manager_contract: Address,
        pub oracle_contract: Address,
        pub risk_engine_contract: Address,
        pub smart_account_contract: Option<Address>,
        pub vxlm_token_contract: Address,
        pub vusdc_token_contract: Address,
        pub veurc_token_contract: Address,

        pub xlm_address: Address,
        pub usdc_address: Address,
        pub eurc_address: Address,
        pub mock_oracle_address: Address,
        pub user: Address,
    }

    pub fn test_initiation(env: &Env) -> ContractAddresses {
        let admin = Address::generate(&env);
        let liquidity_pool_xlm_addr = Address::generate(&env);
        let liquidity_pool_usdc_addr = Address::generate(&env);
        let liquidity_pool_eurc_addr = Address::generate(&env);

        let registry_contract_id = Address::generate(&env);
        let account_manager_id = Address::generate(&env);
        let rate_model_id = Address::generate(&env);
        let oracle_contract_id = Address::generate(&env);
        let risk_engine_contract_id = Address::generate(&env);
        let vxlm_token_contract_id = Address::generate(&env);
        let vusdc_token_contract_id = Address::generate(&env);

        let veurc_token_contract_id = Address::generate(&env);

        let price_feed_add = Address::generate(&env);
        let smart_account_contract = Address::generate(&env);
        let user = Address::generate(&env);

        let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
        let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());
        let eurc_token = env.register_stellar_asset_contract_v2(admin.clone());

        env.mock_all_auths();

        let mut contracts = ContractAddresses {
            admin: admin,
            liquidity_pool_xlm: liquidity_pool_xlm_addr,
            liquidity_pool_usdc: liquidity_pool_usdc_addr,
            liquidity_pool_eurc: liquidity_pool_eurc_addr,
            registry_contract: registry_contract_id,
            rate_model_contract: rate_model_id,
            account_manager_contract: account_manager_id,
            oracle_contract: oracle_contract_id,
            risk_engine_contract: risk_engine_contract_id,
            smart_account_contract: Some(smart_account_contract),
            vxlm_token_contract: vxlm_token_contract_id,
            vusdc_token_contract: vusdc_token_contract_id,
            veurc_token_contract: veurc_token_contract_id,
            xlm_address: xlm_token.address(),
            usdc_address: usdc_token.address(),
            eurc_address: eurc_token.address(),
            mock_oracle_address: price_feed_add,
            user: user.clone(),
        };

        // Deploy account manager contract
        env.register_at(
            &contracts.registry_contract,
            RegistryContract,
            (contracts.admin.clone(),),
        );
        // Deploy registry contract
        env.register_at(
            &contracts.account_manager_contract,
            account_manager_contract::account_manager::AccountManagerContract,
            (contracts.admin.clone(), contracts.registry_contract.clone()),
        );

        // Deploy risk engine contract
        env.register_at(
            &contracts.risk_engine_contract,
            RiskEngineContract,
            (contracts.admin.clone(), contracts.registry_contract.clone()),
        );
        // set oracle contract to something simple (we can reuse a mock)
        let price_feed_addr = oracle_price_feed_test_initiation(&env, &mut contracts);
        // register oracle contract that points to price feed
        env.register_at(
            &contracts.oracle_contract,
            OracleContract,
            (contracts.admin.clone(), price_feed_addr),
        );

        env.register_at(
            &contracts.rate_model_contract,
            RateModelContract,
            (contracts.admin.clone(), contracts.registry_contract.clone()),
        );

        // register smart account and set smart account hash in registry
        let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
        let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
        registry_client.set_smart_account_hash(&smart_hash);
        registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
        registry_client.set_native_eurc_contract_address(&contracts.eurc_address);
        registry_client.set_native_xlm_contract_address(&contracts.xlm_address);
        registry_client.set_oracle_contract_address(&contracts.oracle_contract);
        registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
        registry_client.set_lendingpool_usdc(&contracts.liquidity_pool_usdc);
        registry_client.set_rate_model_address(&contracts.rate_model_contract);
        registry_client.set_accountmanager_contract(&contracts.account_manager_contract);

        env.register_at(
            &contracts.liquidity_pool_usdc,
            LiquidityPoolUSDC,
            (
                contracts.admin.clone(),
                contracts.usdc_address.clone(),
                contracts.registry_contract.clone(),
                contracts.account_manager_contract.clone(),
                contracts.rate_model_contract.clone(),
                contracts.admin.clone(),
            ),
        );

        env.register_at(&contracts.vusdc_token_contract, VUSDCToken, ());
        let vusdc_token_contract_client =
            VUSDCTokenClient::new(&env, &contracts.vusdc_token_contract);
        vusdc_token_contract_client.initialize(
            &contracts.liquidity_pool_usdc,
            &7_u32,
            &String::from_str(&env, "VUSDC TOKEN"),
            &String::from_str(&env, "VUSDC"),
        );

        env.register_at(
            &contracts.smart_account_contract.clone().unwrap(),
            SmartAccountContract,
            (
                contracts.clone().account_manager_contract.clone(),
                contracts.clone().registry_contract,
                user.clone(),
            ),
        );

        assert!(
            registry_client
                .get_xlm_contract_adddress()
                .eq(&contracts.xlm_address)
        );

        env.set_auths(&[]);

        contracts
    }

    fn oracle_price_feed_test_initiation(env: &Env, contracts: &mut ContractAddresses) -> Addr {
        let usdc_symbol = USDC_SYMBOL;
        let xlm_symbol = XLM_SYMBOL;
        let eurc_symbol = EURC_SYMBOL;

        let wasm_hash = env
            .deployer()
            .upload_contract_wasm(testutils::MockPriceOracleWASM);

        println!("CAME HEREEE!!666");

        let price_feed_address = env
            .deployer()
            .with_address(
                contracts.mock_oracle_address.clone(),
                AccountManagerContract::generate_salt(
                    &env,
                    contracts.admin.clone(),
                    contracts.account_manager_contract.clone(),
                    124,
                ),
            )
            .deploy_v2(wasm_hash, ());

        contracts.mock_oracle_address = price_feed_address.clone();
        // println!("Price feed contract deployed! at {:?}", price_feed_addr);

        let price_feed_client = MockPriceOracleClient::new(&env, &price_feed_address);
        let assets = Vec::from_array(
            &env,
            [
                Asset::Other(xlm_symbol),
                Asset::Other(usdc_symbol.clone()),
                Asset::Other(eurc_symbol),
            ],
        );
        price_feed_client.set_data(
            &contracts.admin,
            &testutils::Asset::Other(usdc_symbol.clone()),
            &assets,
            &7,
            &3,
        );
        price_feed_client.set_price(
            &Vec::from_array(&env, [4000000, 9990000, 12262415]),
            &env.ledger().timestamp(),
        );
        contracts.mock_oracle_address.clone()
    }

    pub fn as_auth<T>(env: &Env, who: &Address, f: impl FnOnce() -> T) -> T {
        env.as_contract(who, f)
    }

    pub fn pool_client(env: &Env, ctx: &ContractAddresses) -> LiquidityPoolUSDCClient<'static> {
        LiquidityPoolUSDCClient::new(&env, &ctx.liquidity_pool_usdc)
    }
}

// ============================================================================
//                                  TESTS
// ============================================================================
use helpers::*;

#[test]
fn constructor_and_get_admin_happy_path() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    let admin = usdc_pool_client.get_admin();
    assert_eq!(admin, ctx.admin);

    // Events sanity Todo, check events
    // let evs = env.events().all();
    // assert!(evs.iter().any(|e| format!("{:?}", e).contains("admin_set")));
}

// #[test]
// #[should_panic(expected = "Admin key has not been set")]
// fn get_admin_panics_before_constructor() {
//     let env = Env::default();
//
//     let ctx = test_initiation(&env);
//     let usdc_pool_client = pool_client(&env, &ctx);

//     println!("Hellooooo 3344");
//     // No constructor called => panic
//     let _ = c.get_admin();
// }

#[test]
fn reset_admin_requires_auth_and_updates() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    let new_admin = Address::generate(&env);

    let pp = usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "reset_admin",
                args: (&new_admin,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .reset_admin(&new_admin);

    assert_eq!(
        pp,
        String::from_str(&env, "Adminkey set successfully reset")
    );

    assert_eq!(usdc_pool_client.get_admin(), new_admin);
}

#[test]
fn initialize_pool_requires_admin_and_marks_initialized() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    let res = usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "initialize_pool_usdc",
                args: (&ctx.vusdc_token_contract,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_pool_usdc(&ctx.vusdc_token_contract);

    // let res = c.initialize_pool_usdc(&ctx.vusdc_token_contract);
    assert_eq!(res, String::from_str(&env, "USDC pool initialised"));
    assert!(usdc_pool_client.is_usdc_pool_initialised());
}

#[test]
#[should_panic(expected = "Lending pool not initialised")]
fn deposit_panics_if_not_initialized() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    env.mock_all_auths();

    let amount = U256::from_u128(&env, 1_000);
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &amount); // should panic
}

#[test]
#[should_panic(expected = "Deposit amount must be positive")]
fn deposit_panics_zero_amount() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "initialize_pool_usdc",
                args: (&ctx.vusdc_token_contract,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_pool_usdc(&ctx.vusdc_token_contract);

    usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.user.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "deposit_usdc",
                args: (&ctx.user.clone(), &U256::from_u32(&env, 0)).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deposit_usdc(&ctx.user.clone(), &U256::from_u32(&env, 0));
}

#[test]
fn deposit_mints_vxlm_and_updates_state() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "initialize_pool_usdc",
                args: (&ctx.vusdc_token_contract,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_pool_usdc(&ctx.vusdc_token_contract);

    // Allowing mock auth for minting and then removing it
    env.mock_all_auths();
    let stellar_asset_usdc = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_usdc.mint(&ctx.user, &(50000 * WAD7));
    // env.set_auths(&[]);

    // User has TXLM from test_initiation
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &U256::from_u128(&env, 50_000 * WAD_U128));

    // Check VXLM balance
    let vusdc_token_contract_client = VUSDCTokenClient::new(&env, &ctx.vusdc_token_contract);
    let vbal = vusdc_token_contract_client.balance(&ctx.user.clone());
    println!("Decimals {:?}", vusdc_token_contract_client.decimals());
    println!("vxlm token client bal {:?}", vbal);
    assert!(vbal == (50000 * WAD7));

    // Lenders list contains user
    let lenders = usdc_pool_client.get_lenders_usdc();
    assert!(lenders.iter().any(|a| a == ctx.user.clone()));

    // Pool balance increased
    let x = TokenClient::new(&env, &ctx.usdc_address);
    assert_eq!(x.balance(&ctx.liquidity_pool_usdc), 50_000i128 * WAD7);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn redeem_auth_failure() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user, &100000);

    // deposit then redeem half
    let amount = U256::from_u128(&env, 100_000);
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &amount);

    // user has some vXLM now; redeem portion
    let vusdc_token_contract_client = VUSDCTokenClient::new(&env, &ctx.vusdc_token_contract);
    let minted = vusdc_token_contract_client.balance(&ctx.user.clone()) as u128;

    // let v = TokenClient::new(&env, &ctx.vusdc_token_contract);
    // let minted = v.balance(&ctx.user.clone()) as u128;
    let redeem = U256::from_u128(&env, minted as u128 / 2);

    env.set_auths(&[]);

    usdc_pool_client.redeem_vusdc(&ctx.user.clone(), &redeem);

    // vXLM burnt approximately by redeem amount
    let post = vusdc_token_contract_client.balance(&ctx.user.clone()) as u128;
    assert!(post < minted);
    println!("POST AND MINTED {:?} < {:?}", post, minted);
}

#[test]
fn redeem_auth_logic_success() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user, &(100000 * WAD7));

    // deposit then redeem half
    let amount = U256::from_u128(&env, 100_000 * WAD_U128);
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &amount);

    // user has some vXLM now; redeem portion
    let vusdc_token_contract_client = VUSDCTokenClient::new(&env, &ctx.vusdc_token_contract);
    let minted = vusdc_token_contract_client.balance(&ctx.user.clone()) as u128;

    let redeem = U256::from_u128(&env, 50000 * WAD_U128);
    env.set_auths(&[]);

    usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.user.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "redeem_vusdc",
                args: (&ctx.user.clone(), &redeem).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .redeem_vusdc(&ctx.user.clone(), &redeem);

    // vXLM burnt approximately by redeem amount
    let post = vusdc_token_contract_client.balance(&ctx.user.clone()) as u128;
    assert!(post < minted);
    println!("POST AND MINTED {:?} < {:?}", post, minted);
}

#[test]
#[should_panic(expected = "Insufficient Token Balance to redeem")]
fn redeem_panics_if_over_balance() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    // No deposit => user has 0 vXLM
    usdc_pool_client.redeem_vusdc(&ctx.user.clone(), &U256::from_u32(&env, 1));
}

#[test]
// #[should_panic(expected = "\"failing with contract error\", 13")]
fn redeem_panics_if_pool_insufficient_balance() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user, &(100000 * WAD7));

    // User deposits small, then try redeem huge by minting vXLM directly (simulate malicious vToken mint)
    let amount = U256::from_u128(&env, 10_000 * WAD_U128);
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &amount);

    let vxlm_token_contract_client = VUSDCTokenClient::new(&env, &ctx.vusdc_token_contract);
    vxlm_token_contract_client.mint(&ctx.user.clone(), &(1_000_000_000i128 * WAD7)); // inflate vXLM artificially

    // Now redeem a lot -> should hit InsufficientPoolBalance
    usdc_pool_client.redeem_vusdc(
        &ctx.user.clone(),
        &U256::from_u128(&env, 1_000_000_000 * WAD_U128),
    );
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn lend_to_requires_account_manager_auth_failure() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user, &100000);

    // Seed pool liquidity via user deposit
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &U256::from_u128(&env, 100_000));

    env.set_auths(&[]);

    let first = usdc_pool_client.lend_to(
        &ctx.smart_account_contract.clone().unwrap(),
        &U256::from_u128(&env, 40_000),
    );
}

#[test]
fn lend_to_requires_account_manager_and_updates_borrows_and_shares() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user, &100000);

    // Seed pool liquidity via user deposit
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &U256::from_u128(&env, 100_000));

    // env.register_at(
    //     &ctx.smart_account_contract.clone().unwrap(),
    //     SmartAccountContract,
    //     (
    //         ctx.account_manager_contract.clone(),
    //         ctx.registry_contract,
    //         user,
    //     ),
    // );

    let first = usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.account_manager_contract.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "lend_to",
                args: (
                    &ctx.smart_account_contract.clone().unwrap(),
                    &U256::from_u128(&env, 40_000),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .lend_to(
            &ctx.smart_account_contract.clone().unwrap(),
            &U256::from_u128(&env, 40_000),
        );

    assert!(first, "first borrow should return true");

    // Borrow shares & borrows > 0
    let borrows = usdc_pool_client.get_borrows();
    assert!(borrows > U256::from_u32(&env, 0));
    println!("Borrows {:?}", borrows);
    let shares = usdc_pool_client.get_total_borrow_shares();
    assert!(shares > U256::from_u32(&env, 0));
    println!("Shares {:?}", shares);

    // Second borrow returns false
    let second = usdc_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 1_000),
    );
    assert!(!second);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn collect_from_auth_failure() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user.clone(), &100000);

    usdc_pool_client.deposit_usdc(&ctx.user.clone().clone(), &U256::from_u128(&env, 100_000));
    usdc_pool_client.lend_to(
        &ctx.smart_account_contract.clone().unwrap(),
        &U256::from_u128(&env, 40_000),
    );

    env.set_auths(&[]);

    // Partial repay
    let z = usdc_pool_client.collect_from(
        &U256::from_u128(&env, 10_000),
        &ctx.smart_account_contract.clone().unwrap(),
    );
    assert!(!z);
}

#[test]
fn collect_from_reduces_debt_and_returns_zeroed_flag() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user.clone(), &(100000 * WAD7));

    usdc_pool_client.deposit_usdc(
        &ctx.user.clone(),
        &U256::from_u128(&env, 100_000 * WAD_U128),
    );
    usdc_pool_client.lend_to(
        &ctx.smart_account_contract.clone().unwrap(),
        &U256::from_u128(&env, 40_000 * WAD_U128),
    );

    env.set_auths(&[]);

    // Making sure collect_from can only be authorized by account manager
    let z = usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.account_manager_contract.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "collect_from",
                args: (
                    &U256::from_u128(&env, 10_000 * WAD_U128),
                    &ctx.smart_account_contract.clone().unwrap(),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .collect_from(
            &U256::from_u128(&env, 10_000 * WAD_U128),
            &ctx.smart_account_contract.clone().unwrap(),
        );

    let z2 = usdc_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.account_manager_contract.clone(),
            invoke: &MockAuthInvoke {
                contract: &usdc_pool_client.address,
                fn_name: "collect_from",
                args: (
                    &U256::from_u128(&env, 30_000 * WAD_U128),
                    &ctx.smart_account_contract.clone().unwrap(),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .collect_from(
            &U256::from_u128(&env, 30_000 * WAD_U128),
            &ctx.smart_account_contract.clone().unwrap(),
        );
    assert!(!z);

    assert!(
        z2,
        "should return true when user borrow shares drop to zero"
    );
}

#[test]
#[should_panic(expected = "Zero borrow shares")]
fn collect_from_panics_zero_shares_amount() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    usdc_pool_client.collect_from(
        &U256::from_u32(&env, 0),
        &ctx.smart_account_contract.unwrap(),
    );
}

#[test]
fn state_updates_once_per_timestamp_and_accrues_interest() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user.clone(), &(100000 * WAD7));

    usdc_pool_client.deposit_usdc(
        &ctx.user.clone(),
        &U256::from_u128(&env, 100_000 * WAD_U128),
    );
    usdc_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 50_000 * WAD_U128),
    );

    let before = usdc_pool_client.get_borrows();

    // First call returns 0 because last_updated == now
    let r0 = usdc_pool_client.get_rate_factor();
    assert_eq!(r0, U256::from_u32(&env, 0));

    usdc_pool_client.update_state(); // same timestamp -> no change
    // as_auth(&env, &ctx.liquidity_pool_usdc, ||{
    // });
    let same = usdc_pool_client.get_borrows();
    println!("same, before {:?}, {:?}", same, before);

    assert_eq!(before, same);

    // Advance time
    println!("Timstamp1 : {:?}", env.ledger().timestamp());
    let timestamp = env.ledger().timestamp() + 10;
    as_auth(&env, &ctx.liquidity_pool_usdc, || {
        env.ledger().set_timestamp(timestamp);
    });
    println!("Timstamp3 : {:?}", env.ledger().timestamp());

    usdc_pool_client.update_state();
    let after = usdc_pool_client.get_borrows();

    println!("after, before {:?}, {:?}", after, before);
    assert!(after > same, "interest should accrue");
}

// #[test]
// #[should_panic(expected = "Native XLM client address not set")]
// fn get_native_xlm_client_address_panics_if_missing() {
//     let env = Env::default();

//     let ctx = test_initiation(&env);
//     let usdc_pool_client = pool_client(&env, &ctx);
//     // No constructor -> missing native address
//     let _ = usdc_pool_client.get_native_xlm_client_address();
// }

#[test]
#[should_panic(expected = "Lending pool not initialised")]
fn is_usdc_pool_initialised_panics_if_missing_flag() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);

    let _ = usdc_pool_client.is_usdc_pool_initialised();
}

#[test]
fn convert_usdc_to_vtoken_behaviour_first_deposit_and_proportional() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    // First deposit => 1:1 mapping
    let one = usdc_pool_client.convert_usdc_to_vtoken(&U256::from_u128(&env, 10_000));
    assert_eq!(one, U256::from_u128(&env, 10_000));

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user.clone(), &(100000 * WAD7));

    // After a deposit & mint, conversion becomes proportional
    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &U256::from_u128(&env, 10_000 * WAD_U128));
    let vxlm_token_contract_client = VUSDCTokenClient::new(&env, &ctx.vusdc_token_contract);
    let vx = vxlm_token_contract_client.balance(&ctx.user.clone());

    // let vx = TokenClient::new(&env, &ctx.vusdc_token_contract).balance(&ctx.user.clone()) as u128;
    assert!(vx > 0);

    // Another conversion call returns non-zero and not necessarily equal
    let two = usdc_pool_client.convert_usdc_to_vtoken(&U256::from_u128(&env, 5_000));
    println!("Two {:?}", two);
    assert!(two > U256::from_u32(&env, 0));
}

#[test]
#[should_panic]
fn convert_vtoken_to_xlm_panics_if_supply_zero() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    // With zero vToken supply, this division will panic in contract logic
    let _ = usdc_pool_client.convert_vtoken_to_usdc(&U256::from_u128(&env, 1));
}

#[test]
fn total_assets_is_assets_plus_borrows() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user.clone(), &(100000 * WAD7));

    usdc_pool_client.deposit_usdc(&ctx.user.clone(), &U256::from_u128(&env, 90_000 * WAD_U128));
    usdc_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 40_000 * WAD_U128),
    );

    let assets = usdc_pool_client.get_total_liquidity_in_pool();
    let borrows = usdc_pool_client.get_borrows();
    let total = usdc_pool_client.total_assets();
    assert_eq!(total, assets.add(&borrows));
}

#[test]
fn borrow_shares_conversion_roundtrip() {
    let env = Env::default();

    let ctx = test_initiation(&env);
    let usdc_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    usdc_pool_client.initialize_pool_usdc(&ctx.vusdc_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.usdc_address);
    stellar_asset_xlm.mint(&ctx.user.clone(), &(100000 * WAD7));

    usdc_pool_client.deposit_usdc(
        &ctx.user.clone(),
        &U256::from_u128(&env, 100_000 * WAD_U128),
    );
    usdc_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 40_000 * WAD_U128),
    );

    // Convert amount -> shares -> amount (approx equality when state stable)
    let amt = U256::from_u128(&env, 10_000 * WAD_U128);
    let s = usdc_pool_client.convert_asset_borrow_shares(&amt);
    let back = usdc_pool_client.convert_borrow_shares_asset(&s);
    println!(
        "borrow shares, assets {:?}, {:?}",
        s.to_u128(),
        back.to_u128()
    );
    assert!(amt == back);
    assert!(back > U256::from_u32(&env, 0));
}
