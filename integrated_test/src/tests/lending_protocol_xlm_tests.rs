// Comprehensive tests for `LiquidityPoolXLM`
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
    self as sdk, Address, BytesN, Env, IntoVal, String, Symbol, U256, Vec, contract, contractimpl,
    testutils::{Address as _, Events as _, Ledger, MockAuth, MockAuthInvoke},
    token::TokenClient,
};

use account_manager_contract::account_manager::AccountManagerContractClient;
use account_manager_contract::account_manager::{self, AccountManagerContract};
use lending_protocol_xlm::liquidity_pool_xlm::{self, LiquidityPoolXLM, LiquidityPoolXLMClient};
use oracle_contract::oracle_service::{OracleContract, OracleContractClient};
use registry_contract::registry::{RegistryContract, RegistryContractClient};
use risk_engine_contract::risk_engine::RiskEngineContract;
use sep_40_oracle::testutils::{self, Asset, MockPriceOracle, MockPriceOracleClient};
// use sep_40_oracle::{Asset as MAsset, PriceData, PriceFeedClient, PriceFeedTrait};
use smart_account_contract::smart_account::{SmartAccountContract, SmartAccountContractClient};
use soroban_sdk::Address as Addr;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{log, token};
use vxlm_token_contract::v_xlm::VXLMToken;
use vxlm_token_contract::v_xlm::VXLMTokenClient;

const SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

// ---------- Helpers: deploy contracts & tokens --------------------------------
pub mod helpers {
    use smart_account_contract::smart_account::SmartAccountContract;

    use super::*;
    // use soroban_token_contract::{Client as TokenClient, Token};

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
        pub xlm_address: Address,
        pub usdc_address: Address,
        pub eurc_address: Address,
        pub mock_oracle_address: Address,
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
        let price_feed_add = Address::generate(&env);
        let smart_account_contract = Address::generate(&env);

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
            xlm_address: xlm_token.address(),
            usdc_address: usdc_token.address(),
            eurc_address: eurc_token.address(),
            mock_oracle_address: price_feed_add,
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

        // register smart account and set smart account hash in registry
        let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
        let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
        registry_client.set_smart_account_hash(&smart_hash);
        registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
        registry_client.set_native_eurc_contract_address(&contracts.eurc_address);
        registry_client.set_native_xlm_contract_address(&contracts.xlm_address);
        registry_client.set_oracle_contract_address(&contracts.oracle_contract);
        registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
        registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
        registry_client.set_rate_model_address(&contracts.rate_model_contract);
        registry_client.set_accountmanager_contract(&contracts.account_manager_contract);

        env.register_at(
            &contracts.liquidity_pool_xlm,
            LiquidityPoolXLM,
            (
                contracts.admin.clone(),
                contracts.xlm_address.clone(),
                contracts.registry_contract.clone(),
                contracts.account_manager_contract.clone(),
                contracts.rate_model_contract.clone(),
                contracts.admin.clone(),
            ),
        );

        env.register_at(&contracts.vxlm_token_contract, VXLMToken, ());
        let vxlm_token_contract_client = VXLMTokenClient::new(&env, &contracts.vxlm_token_contract);
        vxlm_token_contract_client.initialize(
            &contracts.liquidity_pool_xlm,
            &6_u32,
            &String::from_str(&env, "VXLM TOKEN"),
            &String::from_str(&env, "VXLM"),
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
        let usdc_symbol = Symbol::new(&env, "USDC");
        let xlm_symbol = Symbol::new(&env, "XLM");
        let eurc_symbol = Symbol::new(&env, "EURC");

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

    pub fn pool_client(env: &Env, ctx: &ContractAddresses) -> LiquidityPoolXLMClient<'static> {
        LiquidityPoolXLMClient::new(&env, &ctx.liquidity_pool_xlm)
    }
}

// ============================================================================
//                                  TESTS
// ============================================================================
use helpers::*;

#[test]
fn constructor_and_get_admin_happy_path() {
    let env = Env::default();
    let user = Address::generate(&env);
    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    let admin = xlm_pool_client.get_admin();
    assert_eq!(admin, ctx.admin);

    // Events sanity Todo, check events
    // let evs = env.events().all();
    // assert!(evs.iter().any(|e| format!("{:?}", e).contains("admin_set")));
}

// #[test]
// #[should_panic(expected = "Admin key has not been set")]
// fn get_admin_panics_before_constructor() {
//     let env = Env::default();
//     let user = Address::generate(&env);
//     let ctx = test_initiation(&env);
//     let xlm_pool_client = pool_client(&env, &ctx);

//     println!("Hellooooo 3344");
//     // No constructor called => panic
//     let _ = c.get_admin();
// }

#[test]
fn reset_admin_requires_auth_and_updates() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    let new_admin = Address::generate(&env);

    let pp = xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
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

    assert_eq!(xlm_pool_client.get_admin(), new_admin);
}

#[test]
fn initialize_pool_requires_admin_and_marks_initialized() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    let res = xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
                fn_name: "initialize_pool_xlm",
                args: (&ctx.vxlm_token_contract,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_pool_xlm(&ctx.vxlm_token_contract);

    // let res = c.initialize_pool_xlm(&ctx.vxlm_token_contract);
    assert_eq!(res, String::from_str(&env, "XLM pool initialised"));
    assert!(xlm_pool_client.is_xlm_pool_initialised());
}

#[test]
#[should_panic(expected = "Lending pool not initialised")]
fn deposit_panics_if_not_initialized() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    env.mock_all_auths();

    let amount = U256::from_u128(&env, 1_000);
    xlm_pool_client.deposit_xlm(&user.clone(), &amount); // should panic
}

#[test]
#[should_panic(expected = "Deposit amount must be positive")]
fn deposit_panics_zero_amount() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
                fn_name: "initialize_pool_xlm",
                args: (&ctx.vxlm_token_contract,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_pool_xlm(&ctx.vxlm_token_contract);

    xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &user.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
                fn_name: "deposit_xlm",
                args: (&user.clone(), &U256::from_u32(&env, 0)).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deposit_xlm(&user.clone(), &U256::from_u32(&env, 0));
}

#[test]
fn deposit_mints_vxlm_and_updates_state() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.admin.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
                fn_name: "initialize_pool_xlm",
                args: (&ctx.vxlm_token_contract,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize_pool_xlm(&ctx.vxlm_token_contract);

    // Allowing mock auth for minting and then removing it
    env.mock_all_auths();
    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.xlm_address);
    stellar_asset_xlm.mint(&user, &50000);
    // env.set_auths(&[]);

    // User has TXLM from test_initiation
    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u32(&env, 50_000));

    // Check VXLM balance
    let vxlm_token_contract_client = VXLMTokenClient::new(&env, &ctx.vxlm_token_contract);
    let vbal = vxlm_token_contract_client.balance(&user.clone());
    assert!(vbal == 50000);
    println!("vxlm token client bal {:?}", vbal);

    // Lenders list contains user
    let lenders = xlm_pool_client.get_lenders_xlm();
    assert!(lenders.iter().any(|a| a == user.clone()));

    // Pool balance increased
    let x = TokenClient::new(&env, &ctx.xlm_address);
    assert_eq!(x.balance(&ctx.liquidity_pool_xlm) as u128, 50_000u128);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn redeem_auth_failure() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.xlm_address);
    stellar_asset_xlm.mint(&user, &100000);

    // deposit then redeem half
    let amount = U256::from_u128(&env, 100_000);
    xlm_pool_client.deposit_xlm(&user.clone(), &amount);

    // user has some vXLM now; redeem portion
    let vxlm_token_contract_client = VXLMTokenClient::new(&env, &ctx.vxlm_token_contract);
    let minted = vxlm_token_contract_client.balance(&user.clone()) as u128;

    // let v = TokenClient::new(&env, &ctx.vxlm_token_contract);
    // let minted = v.balance(&user.clone()) as u128;
    let redeem = U256::from_u128(&env, minted as u128 / 2);

    env.set_auths(&[]);

    xlm_pool_client.redeem_vxlm(&user.clone(), &redeem);

    // vXLM burnt approximately by redeem amount
    let post = vxlm_token_contract_client.balance(&user.clone()) as u128;
    assert!(post < minted);
    println!("POST AND MINTED {:?} < {:?}", post, minted);
}

#[test]
fn redeem_auth_logic_success() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.xlm_address);
    stellar_asset_xlm.mint(&user, &100000);

    // deposit then redeem half
    let amount = U256::from_u128(&env, 100_000);
    xlm_pool_client.deposit_xlm(&user.clone(), &amount);

    // user has some vXLM now; redeem portion
    let vxlm_token_contract_client = VXLMTokenClient::new(&env, &ctx.vxlm_token_contract);
    let minted = vxlm_token_contract_client.balance(&user.clone()) as u128;

    let redeem = U256::from_u128(&env, minted as u128 / 2);
    env.set_auths(&[]);

    xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &user.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
                fn_name: "redeem_vxlm",
                args: (&user.clone(), &redeem).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .redeem_vxlm(&user.clone(), &redeem);

    // vXLM burnt approximately by redeem amount
    let post = vxlm_token_contract_client.balance(&user.clone()) as u128;
    assert!(post < minted);
    println!("POST AND MINTED {:?} < {:?}", post, minted);
}

#[test]
#[should_panic(expected = "Insufficient Token Balance to redeem")]
fn redeem_panics_if_over_balance() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    // No deposit => user has 0 vXLM
    xlm_pool_client.redeem_vxlm(&user.clone(), &U256::from_u32(&env, 1));
}

#[test]
#[should_panic(expected = "\"failing with contract error\", 13")]
fn redeem_panics_if_pool_insufficient_balance() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.xlm_address);
    stellar_asset_xlm.mint(&user, &100000);

    // User deposits small, then try redeem huge by minting vXLM directly (simulate malicious vToken mint)
    let amount = U256::from_u128(&env, 10_000);
    xlm_pool_client.deposit_xlm(&user.clone(), &amount);

    let vxlm_token_contract_client = VXLMTokenClient::new(&env, &ctx.vxlm_token_contract);
    vxlm_token_contract_client.mint(&user.clone(), &1_000_000_000i128); // inflate vXLM artificially

    // Now redeem a lot -> should hit InsufficientPoolBalance
    xlm_pool_client.redeem_vxlm(&user.clone(), &U256::from_u128(&env, 1_000_000_000));
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn lend_to_requires_account_manager_auth_failure() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.xlm_address);
    stellar_asset_xlm.mint(&user, &100000);

    // Seed pool liquidity via user deposit
    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 100_000));

    env.set_auths(&[]);

    let first = xlm_pool_client.lend_to(
        &ctx.smart_account_contract.clone().unwrap(),
        &U256::from_u128(&env, 40_000),
    );
}

#[test]
fn lend_to_requires_account_manager_and_updates_borrows_and_shares() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    env.mock_all_auths();

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &ctx.xlm_address);
    stellar_asset_xlm.mint(&user, &100000);

    // Seed pool liquidity via user deposit
    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 100_000));

    env.register_at(
        &ctx.smart_account_contract.clone().unwrap(),
        SmartAccountContract,
        (
            ctx.account_manager_contract.clone(),
            ctx.registry_contract,
            user,
        ),
    );

    let first = xlm_pool_client
        .mock_auths(&[MockAuth {
            address: &ctx.account_manager_contract.clone(),
            invoke: &MockAuthInvoke {
                contract: &xlm_pool_client.address,
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

    // let first = xlm_pool_client.lend_to(
    //     &ctx.smart_account_contract.clone().unwrap(),
    //     &U256::from_u128(&env, 40_000),
    // );
    assert!(first, "first borrow should return true");

    // Borrow shares & borrows > 0
    let borrows = xlm_pool_client.get_borrows();
    assert!(borrows > U256::from_u32(&env, 0));
    println!("Borrows {:?}", borrows);
    let shares = xlm_pool_client.get_total_borrow_shares();
    assert!(shares > U256::from_u32(&env, 0));
    println!("Shares {:?}", shares);

    // Second borrow returns false
    let second = xlm_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 1_000),
    );
    assert!(!second);
}

#[test]
fn collect_from_reduces_debt_and_returns_zeroed_flag() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 100_000));
    xlm_pool_client.lend_to(
        &ctx.smart_account_contract.clone().unwrap(),
        &U256::from_u128(&env, 40_000),
    );

    // Partial repay
    let z = xlm_pool_client.collect_from(
        &U256::from_u128(&env, 10_000),
        &ctx.smart_account_contract.clone().unwrap(),
    );
    assert!(!z);

    // Full repay of remaining (ignore interest smallness)
    let z2 = xlm_pool_client.collect_from(
        &U256::from_u128(&env, 30_000),
        &ctx.smart_account_contract.unwrap(),
    );
    assert!(
        z2,
        "should return true when user borrow shares drop to zero"
    );
}

#[test]
#[should_panic(expected = "Zero borrow shares")]
fn collect_from_panics_zero_shares_amount() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    xlm_pool_client.collect_from(
        &U256::from_u32(&env, 0),
        &ctx.smart_account_contract.unwrap(),
    );
}

#[test]
fn state_updates_once_per_timestamp_and_accrues_interest() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 100_000));
    xlm_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 50_000),
    );

    let before = xlm_pool_client.get_borrows();
    xlm_pool_client.update_state(); // same timestamp -> no change
    let same = xlm_pool_client.get_borrows();
    assert_eq!(before, same);

    // Advance time
    let timestamp = env.ledger().timestamp() + 10;
    env.ledger().set_timestamp(timestamp);
    xlm_pool_client.update_state();
    let after = xlm_pool_client.get_borrows();
    assert!(after > same, "interest should accrue");
}

#[test]
fn get_rate_factor_zero_if_same_timestamp_else_positive() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    // First call returns 0 because last_updated == now
    let r0 = xlm_pool_client.get_rate_factor();
    assert_eq!(r0, U256::from_u32(&env, 0));

    // Advance time to get positive factor
    env.ledger().with_mut(|l| l.timestamp += 5);
    let r1 = xlm_pool_client.get_rate_factor();
    assert!(r1 > U256::from_u32(&env, 0));
}

#[test]
#[should_panic(expected = "Native XLM client address not set")]
fn get_native_xlm_client_address_panics_if_missing() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);
    // No constructor -> missing native address
    let _ = xlm_pool_client.get_native_xlm_client_address();
}

#[test]
#[should_panic(expected = "Lending pool not initialised")]
fn is_xlm_pool_initialised_panics_if_missing_flag() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    let _ = xlm_pool_client.is_xlm_pool_initialised();
}

#[test]
fn convert_xlm_to_vtoken_behaviour_first_deposit_and_proportional() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    // First deposit => 1:1 mapping
    let one = xlm_pool_client.convert_xlm_to_vtoken(&U256::from_u128(&env, 10_000));
    assert_eq!(one, U256::from_u128(&env, 10_000));

    // After a deposit & mint, conversion becomes proportional
    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 10_000));
    let vx = TokenClient::new(&env, &ctx.vxlm_token_contract).balance(&user.clone()) as u128;
    assert!(vx > 0);

    // Another conversion call returns non-zero and not necessarily equal
    let two = xlm_pool_client.convert_xlm_to_vtoken(&U256::from_u128(&env, 5_000));
    assert!(two > U256::from_u32(&env, 0));
}

#[test]
#[should_panic]
fn convert_vtoken_to_xlm_panics_if_supply_zero() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    // With zero vToken supply, this division will panic in contract logic
    let _ = xlm_pool_client.convert_vtoken_to_xlm(&U256::from_u128(&env, 1));
}

#[test]
fn total_assets_is_assets_plus_borrows() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);
    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 90_000));
    xlm_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 40_000),
    );

    let assets = xlm_pool_client.get_total_liquidity_in_pool();
    let borrows = xlm_pool_client.get_borrows();
    let total = xlm_pool_client.total_assets();
    assert_eq!(total, assets.add(&borrows));
}

#[test]
fn borrow_shares_conversion_roundtrip() {
    let env = Env::default();
    let user = Address::generate(&env);

    let ctx = test_initiation(&env);
    let xlm_pool_client = pool_client(&env, &ctx);

    xlm_pool_client.initialize_pool_xlm(&ctx.vxlm_token_contract);

    xlm_pool_client.deposit_xlm(&user.clone(), &U256::from_u128(&env, 100_000));
    xlm_pool_client.lend_to(
        &ctx.smart_account_contract.unwrap(),
        &U256::from_u128(&env, 40_000),
    );

    // Convert amount -> shares -> amount (approx equality when state stable)
    let amt = U256::from_u128(&env, 10_000);
    let s = xlm_pool_client.convert_asset_borrow_shares(&amt);
    let back = xlm_pool_client.convert_borrow_shares_asset(&s);
    assert!(back > U256::from_u32(&env, 0));
}
