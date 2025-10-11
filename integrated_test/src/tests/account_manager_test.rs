//! =======================
//! Vanna Protocol – Account Manager Comprehensive Test Suite
//! =======================
//! This file includes integration + edge case tests for AccountManagerContract
//! Dependencies: registry_contract, smart_account_contract, risk_engine_contract, liquidity pools, oracle, etc.

use account_manager_contract::account_manager::AccountManagerContractClient;
use account_manager_contract::account_manager::{self, AccountManagerContract};
use account_manager_contract::types::AccountManagerError;
use lending_protocol_xlm::liquidity_pool_xlm::{self, LiquidityPoolXLM, LiquidityPoolXLMClient};
use oracle_contract::oracle_service::{OracleContract, OracleContractClient};
use registry_contract::registry::RegistryContract;
use registry_contract::registry::RegistryContractClient;
use risk_engine_contract::risk_engine::RiskEngineContract;
use sep_40_oracle::testutils::{self, Asset, MockPriceOracle, MockPriceOracleClient};
use sep_40_oracle::{Asset as MAsset, PriceData, PriceFeedClient, PriceFeedTrait};
use smart_account_contract::smart_account::SmartAccountContractClient;
use soroban_sdk::token;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address as Addr, String, Symbol, U256};
use soroban_sdk::{Env, Vec, testutils::Address};
use vxlm_token_contract::v_xlm::VXLMToken;
use vxlm_token_contract::v_xlm::VXLMTokenClient;

const SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

pub struct ContractAddresses {
    admin: Addr,
    liquidity_pool_xlm: Addr,
    liquidity_pool_usdc: Addr,
    liquidity_pool_eurc: Addr,
    registry_contract: Addr,
    rate_model_contract: Addr,
    account_manager_contract: Addr,
    oracle_contract: Addr,
    risk_engine_contract: Addr,
    smart_account_contract: Option<Addr>,
    vxlm_token_contract: Addr,
    xlm_address: Addr,
    usdc_address: Addr,
    eurc_address: Addr,
}

pub fn test_initiation(env: &Env) -> ContractAddresses {
    let admin = Addr::generate(&env);
    let liquidity_pool_xlm_addr = Addr::generate(&env);
    let liquidity_pool_usdc_addr = Addr::generate(&env);
    let liquidity_pool_eurc_addr = Addr::generate(&env);

    let registry_contract_id = Addr::generate(&env);
    let account_manager_id = Addr::generate(&env);
    let rate_model_id = Addr::generate(&env);
    let oracle_contract_id = Addr::generate(&env);
    let risk_engine_contract_id = Addr::generate(&env);
    let vxlm_token_contract_id = Addr::generate(&env);
    let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());
    let eurc_token = env.register_stellar_asset_contract_v2(admin.clone());

    let contracts = ContractAddresses {
        admin: admin,
        liquidity_pool_xlm: liquidity_pool_xlm_addr,
        liquidity_pool_usdc: liquidity_pool_usdc_addr,
        liquidity_pool_eurc: liquidity_pool_eurc_addr,
        registry_contract: registry_contract_id,
        rate_model_contract: rate_model_id,
        account_manager_contract: account_manager_id,
        oracle_contract: oracle_contract_id,
        risk_engine_contract: risk_engine_contract_id,
        smart_account_contract: None,
        vxlm_token_contract: vxlm_token_contract_id,
        xlm_address: xlm_token.address(),
        usdc_address: usdc_token.address(),
        eurc_address: eurc_token.address(),
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
    let price_feed_addr = oracle_price_feed_setup(&env, &contracts);
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
    registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
    registry_client.set_oracle_contract_address(&contracts.oracle_contract);
    registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
    registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
    registry_client.set_rate_model_address(&contracts.rate_model_contract);

    contracts
}

fn liquidity_pool_lenders_initialise(env: &Env, contracts: &ContractAddresses) {
    // let xlm_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());

    env.register_at(
        &contracts.registry_contract,
        RegistryContract,
        (contracts.admin.clone(),),
    );

    env.register_at(
        &contracts.account_manager_contract,
        AccountManagerContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );

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

    let lp_xlm_client = LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);

    let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
    registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
    let lender_addr1 = Addr::generate(&env);
    let lender_addr2 = Addr::generate(&env);

    let lender_addr3 = Addr::generate(&env);

    let lender_addr4 = Addr::generate(&env);

    let stellar_asset = StellarAssetClient::new(&env, &contracts.xlm_address);

    stellar_asset.mint(&lender_addr1, &1000000000i128);
    stellar_asset.mint(&lender_addr2, &1000000000i128);
    stellar_asset.mint(&lender_addr3, &1000000000i128);
    stellar_asset.mint(&lender_addr4, &1000000000i128);

    let amount1 = U256::from_u32(&env, 400);
    let amount2 = U256::from_u32(&env, 500);
    let amount3 = U256::from_u32(&env, 600);
    let amount4 = U256::from_u32(&env, 700);

    let x = lp_xlm_client.initialize_pool_xlm(&contracts.vxlm_token_contract);
    // println!("response from : {:?} ", x);

    lp_xlm_client.deposit_xlm(&lender_addr1, &amount1);
    lp_xlm_client.deposit_xlm(&lender_addr2, &amount2);
    lp_xlm_client.deposit_xlm(&lender_addr3, &amount3);
    lp_xlm_client.deposit_xlm(&lender_addr4, &amount4);

    let xlm_token_client = token::TokenClient::new(&env, &contracts.xlm_address);
    println!(
        "Balance after depositing: {:?}",
        xlm_token_client.balance(&lender_addr1)
    );
    assert!(xlm_token_client.balance(&lender_addr1) == 999999600);
    assert!(xlm_token_client.balance(&lender_addr2) == 999999500);
    assert!(xlm_token_client.balance(&lender_addr3) == 999999400);
    assert!(xlm_token_client.balance(&lender_addr4) == 999999300);
}

fn oracle_price_feed_setup(env: &Env, contracts: &ContractAddresses) -> Addr {
    let price_feed_add = Addr::generate(&env);
    let usdc_symbol = Symbol::new(&env, "USDC");
    let xlm_symbol = Symbol::new(&env, "XLM");
    let eurc_symbol = Symbol::new(&env, "EURC");
    let sol_symbol = Symbol::new(&env, "SOL");

    let xlm_address = Addr::generate(&env);

    let wasm_hash = env
        .deployer()
        .upload_contract_wasm(testutils::MockPriceOracleWASM);

    let price_feed_addr = env
        .deployer()
        .with_address(
            price_feed_add,
            AccountManagerContract::generate_predictable_salt(
                &env,
                contracts.admin.clone(),
                contracts.account_manager_contract.clone(),
            ),
        )
        .deploy_v2(wasm_hash, ());

    println!("Price feed contract deployed! at {:?}", price_feed_addr);

    let price_feed_client = MockPriceOracleClient::new(&env, &price_feed_addr);
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
    price_feed_addr
}

#[test]
fn all_integrated_tests_start() {
    let env = Env::default();
    env.mock_all_auths();

    let contracts = test_initiation(&env);

    let xlm_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());
    // let vxlm_token =
    //     env.register_stellar_asset_contract_v2(contracts.liquidity_pool_xlm.clone());

    println!("vxlm token address is {:?}", contracts.vxlm_token_contract);

    env.register_at(
        &contracts.registry_contract,
        RegistryContract,
        (contracts.admin.clone(),),
    );

    env.register_at(
        &contracts.account_manager_contract,
        AccountManagerContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );

    env.register_at(
        &contracts.liquidity_pool_xlm,
        LiquidityPoolXLM,
        (
            contracts.admin.clone(),
            xlm_token.address(),
            // vxlm_token.address(),
            contracts.registry_contract.clone(),
            contracts.account_manager_contract.clone(),
            contracts.rate_model_contract,
            contracts.admin.clone(),
        ),
    );

    let vxlm_token_contract_address =
        env.register_at(&contracts.vxlm_token_contract, VXLMToken, ());

    let vxlm_token_contract_client = VXLMTokenClient::new(&env, &vxlm_token_contract_address);
    vxlm_token_contract_client.initialize(
        &contracts.liquidity_pool_xlm,
        &6_u32,
        &String::from_str(&env, "VXLM TOKEN"),
        &String::from_str(&env, "VXLM"),
    );

    let lp_xlm_client =
        liquidity_pool_xlm::LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);

    let lender_addr = Addr::generate(&env);
    let stellar_asset = StellarAssetClient::new(&env, &xlm_token.address());

    let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
    registry_client.set_native_xlm_contract_adddress(&xlm_token.address());

    stellar_asset.mint(&lender_addr, &1000000000i128);

    let amount = U256::from_u32(&env, 400);
    let amountx = U256::from_u32(&env, 40);

    let x = lp_xlm_client.initialize_pool_xlm(&vxlm_token_contract_address);
    // println!("response from : {:?} ", x);

    lp_xlm_client.deposit_xlm(&lender_addr, &amount);

    // println!(
    //     " VXLM balance after depositing : {:?}",
    //     vxlm_token_contract_client.balance(&lender_addr)
    // );

    lp_xlm_client.redeem_vxlm(&lender_addr, &amountx);

    // println!(
    //     " VXLM balance after redeeming : {:?}",
    //     vxlm_token_contract_client.balance(&lender_addr)
    // );

    let registry_contract_client =
        RegistryContractClient::new(&env, &&contracts.registry_contract.clone());

    let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

    registry_contract_client.set_smart_account_hash(&smart_account_hash);

    let account_manager_client =
        account_manager_contract::account_manager::AccountManagerContractClient::new(
            &env,
            &contracts.account_manager_contract,
        );

    let trader_address = Addr::generate(&env);

    let add = account_manager_client.create_account(&trader_address);
    println!("Created margin account address is  : {:?}", add);

    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    stellar_asset.mint(&trader_address, &10000i128);
}

#[test]
fn account_manager_flows() {
    let env = Env::default();
    env.mock_all_auths();

    let contracts = test_initiation(&env);
    let reflector_address = Addr::generate(&env);
    let usdc_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());
    let xlm_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());

    env.register_at(
        &contracts.registry_contract,
        RegistryContract,
        (contracts.admin.clone(),),
    );

    env.register_at(
        &contracts.account_manager_contract,
        AccountManagerContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );

    env.register_at(
        &contracts.risk_engine_contract,
        RiskEngineContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );

    env.register_at(
        &contracts.oracle_contract,
        OracleContract,
        (contracts.admin.clone(), reflector_address),
    );

    let account_manager_client =
        account_manager_contract::account_manager::AccountManagerContractClient::new(
            &env,
            &contracts.account_manager_contract,
        );

    let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

    let registry_contract_client =
        RegistryContractClient::new(&env, &contracts.registry_contract.clone());
    registry_contract_client.set_smart_account_hash(&smart_account_hash);
    registry_contract_client.set_native_usdc_contract_address(&usdc_token.address());
    registry_contract_client.set_native_xlm_contract_adddress(&xlm_token.address());
    registry_contract_client.set_risk_engine_address(&contracts.risk_engine_contract);
    registry_contract_client.set_oracle_contract_address(&contracts.oracle_contract);

    let stellar_asset_usdc = StellarAssetClient::new(&env, &usdc_token.address());
    let stellar_asset_xlm = StellarAssetClient::new(&env, &xlm_token.address());

    let trader_address = Addr::generate(&env);
    let trader_address2 = Addr::generate(&env);

    println!("TRader address 1 {:?}", trader_address);
    println!("Trader address 2 {:?}", trader_address2);

    let margin_acc1 = account_manager_client.create_account(&trader_address);

    let margin_acc2 = account_manager_client.create_account(&trader_address2);

    println!("CReated margin account addres is {:?}", margin_acc1);
    println!("CReated margin account addres is {:?}", margin_acc2);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));

    assert!(
        account_manager_client
            .get_max_asset_cap()
            .eq(&U256::from_u32(&env, 10))
    );

    stellar_asset_xlm.mint(&trader_address, &10000i128);
    stellar_asset_xlm.mint(&trader_address2, &10000i128);

    println!(
        "Account manager address is {:?}",
        &contracts.account_manager_contract
    );

    let margin_client1 =
        smart_account_contract::smart_account::SmartAccountContractClient::new(&env, &margin_acc1);

    let margin_client2 =
        smart_account_contract::smart_account::SmartAccountContractClient::new(&env, &margin_acc2);

    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    let collateral_balx = margin_client1.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    println!("COllateral balance before XLM is {:?}", collateral_balx);

    account_manager_client.deposit_collateral_tokens(
        &margin_acc1,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 100),
    );

    account_manager_client.deposit_collateral_tokens(
        &margin_acc2,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 999),
    );

    let collateral_bal = margin_client1.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    println!("COllateral balance after XLM is {:?}", collateral_bal);

    let collateral_bal = margin_client2.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    println!(
        "COllateral balance of Trader 2 after XLM is {:?}",
        collateral_bal
    );

    account_manager_client.withdraw_collateral_balance(
        &margin_acc1,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 80),
    );

    // account_manager_client.borrow(
    //     &trader_address,
    //     &U256::from_u32(&env, 1000),
    //     &Symbol::new(&env, "USDC"),
    // );

    // account_manager_client.approve();
}

#[test]
fn test_oracle_price() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    let price_feed_add = Addr::generate(&env);
    let usdc_symbol = Symbol::new(&env, "USDC");
    let xlm_symbol = Symbol::new(&env, "XLM");
    let eurc_symbol = Symbol::new(&env, "EURC");

    let wasm_hash = env
        .deployer()
        .upload_contract_wasm(testutils::MockPriceOracleWASM);

    let price_feed_addr = env
        .deployer()
        .with_address(
            price_feed_add,
            AccountManagerContract::generate_predictable_salt(
                &env,
                contracts.admin.clone(),
                contracts.account_manager_contract.clone(),
            ),
        )
        .deploy_v2(wasm_hash, ());

    let price_feed_client = MockPriceOracleClient::new(&env, &price_feed_addr);
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
        &2,
        &6,
    );
    price_feed_client.set_price(
        &Vec::from_array(&env, [1000000, 28437629, 3000000]),
        &9988229,
    );
    let recent = price_feed_client.lastprice(&testutils::Asset::Other(usdc_symbol.clone()));
    println!("Recent price {:?}", recent.unwrap().price);

    // Check if oracle test mode is fetching the same data added into the price feed
    let oracle_address = env.register_at(
        &contracts.oracle_contract,
        OracleContract,
        (contracts.admin.clone(), price_feed_addr),
    );

    let oracle_client = OracleContractClient::new(&env, &oracle_address);

    let (price, decimals) = oracle_client.get_price_latest(&Symbol::new(&env, "USDC"));
    println!("Oracle price : {:?}", price);
    assert!(price == 28437629 && decimals == 2);
}

#[test]
fn test_trader_borrow_logic() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    liquidity_pool_lenders_initialise(&env, &contracts);

    let price_feed_addr = oracle_price_feed_setup(&env, &contracts);

    env.register_at(
        &contracts.risk_engine_contract,
        RiskEngineContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );

    env.register_at(
        &contracts.oracle_contract,
        OracleContract,
        (contracts.admin.clone(), price_feed_addr),
    );

    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

    let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

    let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract.clone());
    registry_client.set_smart_account_hash(&smart_account_hash);
    registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
    registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
    registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
    registry_client.set_oracle_contract_address(&contracts.oracle_contract);
    registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
    registry_client.set_rate_model_address(&contracts.rate_model_contract);

    let stellar_asset_usdc = StellarAssetClient::new(&env, &contracts.usdc_address);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &contracts.xlm_address);

    let trader_address = Addr::generate(&env);
    println!("Trader address 1 {:?}", trader_address);

    let margin_acc1 = account_manager_client.create_account(&trader_address);
    println!("Created margin account addres is {:?}", margin_acc1);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));

    assert!(
        account_manager_client
            .get_max_asset_cap()
            .eq(&U256::from_u32(&env, 10))
    );

    stellar_asset_usdc.mint(&trader_address, &10000i128);
    let usdc_symbol = Symbol::new(&env, "USDC");
    let xlm_symbol = Symbol::new(&env, "XLM");

    let margin_client1 = SmartAccountContractClient::new(&env, &margin_acc1);
    account_manager_client.set_iscollateral_allowed(&usdc_symbol);

    let collateral_balx = margin_client1.get_collateral_token_balance(&usdc_symbol);
    assert!(collateral_balx.eq(&U256::from_u128(&env, 0)));

    account_manager_client.deposit_collateral_tokens(
        &margin_acc1,
        &usdc_symbol,
        &U256::from_u128(&env, 100),
    );

    let collateral_bal = margin_client1.get_collateral_token_balance(&usdc_symbol);
    assert!(collateral_bal.eq(&U256::from_u128(&env, 100)));

    account_manager_client.withdraw_collateral_balance(
        &margin_acc1,
        &usdc_symbol,
        &U256::from_u128(&env, 10),
    );

    let collateral_balxy = margin_client1.get_collateral_token_balance(&usdc_symbol);
    assert!(collateral_balxy.eq(&U256::from_u128(&env, 90)));

    account_manager_client.borrow(&margin_acc1, &U256::from_u128(&env, 10), &xlm_symbol);
    let borrowd_xlm = margin_client1.get_borrowed_token_debt(&xlm_symbol);
    assert!(borrowd_xlm.eq(&U256::from_u128(&env, 10)));

    let xlm = token::Client::new(&env, &contracts.xlm_address);
    println!("Balance before repay{:?}", xlm.balance(&margin_acc1));

    account_manager_client.repay(&U256::from_u128(&env, 8), &xlm_symbol, &margin_acc1);
    assert!(xlm.balance(&margin_acc1).eq(&2));
}

#[test]
// #[should_panic(expected = "assertion failed")]
fn test_trader_borrow_failures() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    liquidity_pool_lenders_initialise(&env, &contracts);

    let price_feed_addr = oracle_price_feed_setup(&env, &contracts);

    env.register_at(
        &contracts.risk_engine_contract,
        RiskEngineContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );

    env.register_at(
        &contracts.oracle_contract,
        OracleContract,
        (contracts.admin.clone(), price_feed_addr),
    );

    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

    let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

    let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract.clone());
    registry_client.set_smart_account_hash(&smart_account_hash);
    registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
    registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
    registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
    registry_client.set_oracle_contract_address(&contracts.oracle_contract);
    registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
    registry_client.set_rate_model_address(&contracts.rate_model_contract);

    let stellar_asset_usdc = StellarAssetClient::new(&env, &contracts.usdc_address);
    let stellar_asset_xlm = StellarAssetClient::new(&env, &contracts.xlm_address);

    let trader_address = Addr::generate(&env);
    let margin_acc1 = account_manager_client.create_account(&trader_address);

    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));

    stellar_asset_usdc.mint(&trader_address, &10000i128);
    let usdc_symbol = Symbol::new(&env, "USDC");
    let xlm_symbol = Symbol::new(&env, "XLM");

    let margin_client1 = SmartAccountContractClient::new(&env, &margin_acc1);
    account_manager_client.set_iscollateral_allowed(&usdc_symbol);

    margin_client1.get_collateral_token_balance(&usdc_symbol);

    account_manager_client.deposit_collateral_tokens(
        &margin_acc1,
        &usdc_symbol,
        &U256::from_u128(&env, 100),
    );

    margin_client1.get_collateral_token_balance(&usdc_symbol);

    account_manager_client.withdraw_collateral_balance(
        &margin_acc1,
        &usdc_symbol,
        &U256::from_u128(&env, 90),
    );

    let lp_xlm_client = LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);
    let pool_borrows = lp_xlm_client.get_borrows();
    println!("XLM Pool borrows before {:?} ", pool_borrows);

    account_manager_client.borrow(&margin_acc1, &U256::from_u128(&env, 10), &xlm_symbol);
    let borrowd_xlm = margin_client1.get_borrowed_token_debt(&xlm_symbol);
    let borrows_after = lp_xlm_client.get_borrows();

    println!("XLM Pool borrows after {:?} ", borrows_after);

    assert!(borrowd_xlm.eq(&U256::from_u128(&env, 10)));
}

// /// ----------- Account Manager Tests start here ------------

#[test]
#[should_panic(expected = "trying to get non-existing value for contract instance")]
fn test_account_creation_without_registry_should_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let account_manager_contract = Addr::generate(&env);
    let registry_contract = Addr::generate(&env);
    let admin = Addr::generate(&env);

    // no registry set
    env.register_at(
        &account_manager_contract,
        AccountManagerContract,
        (admin, registry_contract),
    );
    let am_client = AccountManagerContractClient::new(&env, &account_manager_contract);
    let trader = Addr::generate(&env);
    am_client.create_account(&trader);
}

#[test]
#[should_panic(expected = "Trader already has a smart account!")]
fn account_creation_and_duplicate_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let contracts = test_initiation(&env);

    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

    let trader = Addr::generate(&env);
    let trader2 = Addr::generate(&env);
    let smart_addr = account_manager_client.create_account(&trader);
    // calling again for duplicate must return same address (create_account uses deterministic salt)
    let smart_addr2 = account_manager_client.create_account(&trader2);

    assert_ne!(smart_addr, smart_addr2);

    // creating second smart account for same trader
    account_manager_client.create_account(&trader2);
}

#[test]
#[should_panic(expected = "Cannot deposit a zero amount")]
fn deposit_xlm_failure() {
    let env = Env::default();
    env.mock_all_auths();

    let contracts = test_initiation(&env);

    // register token contracts (already registered in test_initiation), use them
    let xlm_token = StellarAssetClient::new(&env, &contracts.xlm_address);

    // set admin values and allow XLM as collateral
    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 5));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    // create trader and account
    let trader = Addr::generate(&env);
    xlm_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);

    // deposit 100 XLM (account_manager will transfer from trader -> smart_account)
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 0),
    );
}

#[test]
#[should_panic(expected = "Cannot withdraw a zero amount")]
fn withdraw_xlm_failure() {
    let env = Env::default();
    env.mock_all_auths();

    let contracts = test_initiation(&env);

    // register token contracts (already registered in test_initiation), use them
    let xlm_token = StellarAssetClient::new(&env, &contracts.xlm_address);

    // set admin values and allow XLM as collateral
    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 5));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    // create trader and account
    let trader = Addr::generate(&env);
    xlm_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);

    // deposit 100 XLM (account_manager will transfer from trader -> smart_account)
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 100),
    );

    // check smart account collateral balance
    let smart_client = SmartAccountContractClient::new(&env, &smart_acc);
    let bal = smart_client.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    assert_eq!(bal, U256::from_u128(&env, 100));

    // attempt to withdraw 50 XLM (since we mocked auths & prices reasonable, risk engine should allow)
    account_manager_client.withdraw_collateral_balance(
        &smart_acc,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 0),
    );
}

#[test]
fn deposit_xlm_and_withdraw_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contracts = test_initiation(&env);

    // register token contracts (already registered in test_initiation), use them
    let xlm_token = StellarAssetClient::new(&env, &contracts.xlm_address);

    // set admin values and allow XLM as collateral
    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 5));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    // create trader and account
    let trader = Addr::generate(&env);
    xlm_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);

    // deposit 100 XLM (account_manager will transfer from trader -> smart_account)
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 100),
    );

    // check smart account collateral balance
    let smart_client = SmartAccountContractClient::new(&env, &smart_acc);
    let bal = smart_client.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    assert_eq!(bal, U256::from_u128(&env, 100));

    // attempt to withdraw 50 XLM (since we mocked auths & prices reasonable, risk engine should allow)
    account_manager_client.withdraw_collateral_balance(
        &smart_acc,
        &Symbol::new(&env, "XLM"),
        &U256::from_u128(&env, 50),
    );

    let bal_after = smart_client.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    assert_eq!(bal_after, U256::from_u128(&env, 50));
}

#[test]
#[should_panic(expected = "User doesn't have collateral in this token")]
fn withdraw_without_collateral_should_panic() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);
    let trader = Addr::generate(&env);
    let smart_acc = account_manager_client.create_account(&trader);

    // immediately attempt withdraw (no collateral) -> should panic
    let e = account_manager_client.withdraw_collateral_balance(
        &smart_acc,
        &Symbol::new(&env, "USDC"),
        &U256::from_u128(&env, 10),
    );

    // assert!(e.eq(AccountManagerError::UserDoesntHaveCollateralToken));
}

#[test]
#[should_panic(expected = "Cannot borrow a zero amount")]
fn borrow_xlm_failure() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    // initialize lending pool and fund it
    liquidity_pool_lenders_initialise(&env, &contracts);

    // Build account manager client
    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

    // create trader + smart account and allow USDC/XLM as collaterals
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    let trader = Addr::generate(&env);
    // mint some usdc to trader (so they can deposit as collateral)
    let usdc_token = StellarAssetClient::new(&env, &contracts.usdc_address);
    usdc_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);

    // deposit 100 USDC collateral
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "USDC"),
        &U256::from_u128(&env, 100),
    );

    // Now attempt to borrow XLM (risk engine consults oracle + collateral)
    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 0),
        &Symbol::new(&env, "XLM"),
    );
}

#[test]
#[should_panic(expected = "Cannot repay a zero amount")]
fn repay_xlm_failure() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    // initialize lending pool and fund it
    liquidity_pool_lenders_initialise(&env, &contracts);

    // Build account manager client
    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

    // create trader + smart account and allow USDC/XLM as collaterals
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    let trader = Addr::generate(&env);
    // mint some usdc to trader (so they can deposit as collateral)
    let usdc_token = StellarAssetClient::new(&env, &contracts.usdc_address);
    usdc_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);

    // deposit 100 USDC collateral
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "USDC"),
        &U256::from_u128(&env, 100),
    );

    // Now attempt to borrow XLM (risk engine consults oracle + collateral)
    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 10),
        &Symbol::new(&env, "XLM"),
    );

    // After borrow, the smart_account should have a borrow recorded via pool logic.
    let smart_client = SmartAccountContractClient::new(&env, &smart_acc);
    // borrowed tokens list should contain XLM
    let borrowed = smart_client.get_all_borrowed_tokens();
    assert!(borrowed.contains(Symbol::new(&env, "XLM")));

    // Smart account should have received XLM tokens from pool; confirm balance > 0
    let xlm_token = token::Client::new(&env, &contracts.xlm_address);
    let bal = xlm_token.balance(&smart_acc);
    assert!(bal > 0);

    // Now repay partial amount (repay 5 XLM)
    account_manager_client.repay(
        &U256::from_u128(&env, 0),
        &Symbol::new(&env, "XLM"),
        &smart_acc,
    );

    // After repay, borrow shares/debt reduced — check that collect_from executed without panic.
}

#[test]
fn borrow_and_repay_xlm_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    // initialize lending pool and fund it
    liquidity_pool_lenders_initialise(&env, &contracts);

    // Build account manager client
    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

    // create trader + smart account and allow USDC/XLM as collaterals
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

    let trader = Addr::generate(&env);
    // mint some usdc to trader (so they can deposit as collateral)
    let usdc_token = StellarAssetClient::new(&env, &contracts.usdc_address);
    usdc_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);

    // deposit 100 USDC collateral
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "USDC"),
        &U256::from_u128(&env, 100),
    );

    // Now attempt to borrow XLM (risk engine consults oracle + collateral)
    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 10),
        &Symbol::new(&env, "XLM"),
    );

    // After borrow, the smart_account should have a borrow recorded via pool logic.
    let smart_client = SmartAccountContractClient::new(&env, &smart_acc);
    // borrowed tokens list should contain XLM
    let borrowed = smart_client.get_all_borrowed_tokens();
    assert!(borrowed.contains(Symbol::new(&env, "XLM")));

    // Smart account should have received XLM tokens from pool; confirm balance > 0
    let xlm_token = token::Client::new(&env, &contracts.xlm_address);
    let bal = xlm_token.balance(&smart_acc);
    assert!(bal == 10_i128);

    // Now repay partial amount (repay 5 XLM)
    account_manager_client.repay(
        &U256::from_u128(&env, 5),
        &Symbol::new(&env, "XLM"),
        &smart_acc,
    );

    // After repay, borrow shares/debt reduced — check that collect_from executed without panic.
}

#[test]
#[should_panic(expected = "Cannot delete account with debt, please repay debt first")]
fn delete_account_with_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    // initialize pool + fund
    liquidity_pool_lenders_initialise(&env, &contracts);

    // register risk engine + oracle
    env.register_at(
        &contracts.risk_engine_contract,
        RiskEngineContract,
        (contracts.admin.clone(), contracts.registry_contract.clone()),
    );
    let price_feed_addr = oracle_price_feed_setup(&env, &contracts);
    env.register_at(
        &contracts.oracle_contract,
        OracleContract,
        (contracts.admin.clone(), price_feed_addr),
    );

    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));

    // create trader & deposit USDC collateral, then borrow
    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &contracts.usdc_address);
    usdc_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "USDC"),
        &U256::from_u128(&env, 100),
    );

    // borrow some XLM -> smart account will have debt
    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 10),
        &Symbol::new(&env, "XLM"),
    );

    // now attempt to delete account -> should panic because smart_account.has_debt() == true
    account_manager_client.delete_account(&smart_acc);
}

#[test]
fn settle_account_invokes_repay_for_all_borrows() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    liquidity_pool_lenders_initialise(&env, &contracts);

    let account_manager_client =
        AccountManagerContractClient::new(&env, &contracts.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));

    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &contracts.usdc_address);
    usdc_token.mint(&trader, &10_000i128);

    let smart_acc = account_manager_client.create_account(&trader);
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &Symbol::new(&env, "USDC"),
        &U256::from_u128(&env, 100),
    );

    // Borrow & then settle: settle_account calls repay for each borrowed token
    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 5),
        &Symbol::new(&env, "XLM"),
    );

    // Call settle_account (should call repay internally for outstanding tokens)
    let res = account_manager_client.settle_account(&smart_acc);
    assert!(res);

    // After settle account, the smart_account should have a borrow recorded via pool logic.
    let smart_client = SmartAccountContractClient::new(&env, &smart_acc);
    // borrowed tokens list should not contain XLM after repay
    let borrowed = smart_client.get_all_borrowed_tokens();
    assert!(!borrowed.contains(Symbol::new(&env, "XLM")));

    // Smart account should have 0 XLM tokens after repay
    let xlm_token = token::Client::new(&env, &contracts.xlm_address);
    let bal = xlm_token.balance(&smart_acc);
    assert!(bal == 0_i128);
}

#[test]
fn predictable_salt_is_consistent() {
    let env = Env::default();
    env.mock_all_auths();
    let contracts = test_initiation(&env);

    let trader = Addr::generate(&env);
    let salt_a = AccountManagerContract::generate_predictable_salt(
        &env,
        trader.clone(),
        contracts.account_manager_contract.clone(),
    );
    let salt_b = AccountManagerContract::generate_predictable_salt(
        &env,
        trader.clone(),
        contracts.account_manager_contract.clone(),
    );
    assert_eq!(salt_a, salt_b);
}
