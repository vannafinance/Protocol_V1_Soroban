#![cfg(test)]
#![allow(clippy::needless_return)]

use lending_protocol_eurc::liquidity_pool_eurc::{LiquidityPoolEURC, LiquidityPoolEURCClient};
use lending_protocol_usdc::liquidity_pool_usdc::{LiquidityPoolUSDC, LiquidityPoolUSDCClient};
use risk_engine_contract::types::RiskEngineKey;
use soroban_sdk::testutils::storage::Persistent;
use soroban_sdk::{Address, BytesN, Env, Symbol, U256, Vec, testutils::Address as _};

// --- Bring the contract under test into scope
use account_manager_contract::account_manager::AccountManagerContractClient;
use account_manager_contract::account_manager::{self, AccountManagerContract};
use lending_protocol_xlm::liquidity_pool_xlm::{self, LiquidityPoolXLM, LiquidityPoolXLMClient};
use oracle_contract::oracle_service::{OracleContract, OracleContractClient};
use registry_contract::registry::{RegistryContract, RegistryContractClient};
use risk_engine_contract::risk_engine::{
    BALANCE_TO_BORROW_THRESHOLD, RiskEngineContract, RiskEngineContractClient, WAD_U128,
};
use sep_40_oracle::testutils::{self, Asset, MockPriceOracle, MockPriceOracleClient};
// use sep_40_oracle::{Asset as MAsset, PriceData, PriceFeedClient, PriceFeedTrait};
use rate_model_contract::rate_model::RateModelContract;
use smart_account_contract::smart_account::{SmartAccountContract, SmartAccountContractClient};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address as Addr, symbol_short};
use soroban_sdk::{String, log, token};
use veurc_token_contract::v_eurc::{VEURCToken, VEURCTokenClient};
use vusdc_token_contract::v_usdc::{VUSDCToken, VUSDCTokenClient};
use vxlm_token_contract::v_xlm::VXLMToken;
use vxlm_token_contract::v_xlm::VXLMTokenClient;

const SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

const LARGE_AMOUNT: i128 = (1000000 * WAD7) as i128;
const WAD16_U128: u128 = 10000_0000_00000_000; // 1e16

const WAD7: i128 = 10000000;
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");

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
    pub treasury: Address,
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
    let treasury = Address::generate(&env);

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
        treasury: treasury.clone(),
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
    registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
    registry_client.set_lendingpool_eurc(&contracts.liquidity_pool_eurc);
    registry_client.set_lendingpool_usdc(&contracts.liquidity_pool_usdc);
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
            contracts.treasury.clone(),
            U256::from_u128(&env, 1 * WAD16_U128),
        ),
    );

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
            contracts.treasury.clone(),
            U256::from_u128(&env, 1 * WAD16_U128),
        ),
    );

    env.register_at(
        &contracts.liquidity_pool_eurc,
        LiquidityPoolEURC,
        (
            contracts.admin.clone(),
            contracts.eurc_address.clone(),
            contracts.registry_contract.clone(),
            contracts.account_manager_contract.clone(),
            contracts.rate_model_contract.clone(),
            contracts.admin.clone(),
            contracts.treasury.clone(),
            U256::from_u128(&env, 1 * WAD16_U128),
        ),
    );

    env.register_at(&contracts.vxlm_token_contract, VXLMToken, ());
    let vxlm_token_contract_client = VXLMTokenClient::new(&env, &contracts.vxlm_token_contract);
    vxlm_token_contract_client.initialize(
        &contracts.liquidity_pool_xlm,
        &7_u32,
        &String::from_str(&env, "VXLM TOKEN"),
        &String::from_str(&env, "VXLM"),
    );

    env.register_at(&contracts.vusdc_token_contract, VUSDCToken, ());
    let vusdc_token_contract_client = VUSDCTokenClient::new(&env, &contracts.vusdc_token_contract);
    vusdc_token_contract_client.initialize(
        &contracts.liquidity_pool_usdc,
        &7_u32,
        &String::from_str(&env, "VUSDC TOKEN"),
        &String::from_str(&env, "VUSDC"),
    );

    env.register_at(&contracts.veurc_token_contract, VEURCToken, ());
    let veurc_token_contract_client = VEURCTokenClient::new(&env, &contracts.veurc_token_contract);
    veurc_token_contract_client.initialize(
        &contracts.liquidity_pool_eurc,
        &7_u32,
        &String::from_str(&env, "VEURC TOKEN"),
        &String::from_str(&env, "VEURC"),
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

fn initialise_lenders(env: &Env, contracts: &ContractAddresses) {
    let lender_addr1 = Addr::generate(&env);
    let lender_addr2 = Addr::generate(&env);
    let lender_addr3 = Addr::generate(&env);
    let lender_addr4 = Addr::generate(&env);

    let stellar_asset_xlm = StellarAssetClient::new(&env, &contracts.xlm_address);
    let stellar_asset_usdc = StellarAssetClient::new(&env, &contracts.usdc_address);
    let stellar_asset_eurc = StellarAssetClient::new(&env, &contracts.eurc_address);

    // let large_amount = (1000000 * WAD_U128) as i128;

    stellar_asset_xlm.mint(&lender_addr1, &LARGE_AMOUNT);
    stellar_asset_xlm.mint(&lender_addr2, &LARGE_AMOUNT);
    stellar_asset_xlm.mint(&lender_addr3, &LARGE_AMOUNT);
    stellar_asset_xlm.mint(&lender_addr4, &LARGE_AMOUNT);

    stellar_asset_usdc.mint(&lender_addr1, &LARGE_AMOUNT);
    stellar_asset_usdc.mint(&lender_addr2, &LARGE_AMOUNT);
    stellar_asset_usdc.mint(&lender_addr3, &LARGE_AMOUNT);
    stellar_asset_usdc.mint(&lender_addr4, &LARGE_AMOUNT);

    stellar_asset_eurc.mint(&lender_addr1, &LARGE_AMOUNT);
    stellar_asset_eurc.mint(&lender_addr2, &LARGE_AMOUNT);
    stellar_asset_eurc.mint(&lender_addr3, &LARGE_AMOUNT);
    stellar_asset_eurc.mint(&lender_addr4, &LARGE_AMOUNT);

    let amount1 = U256::from_u128(&env, 400 * WAD_U128);
    let amount2 = U256::from_u128(&env, 500 * WAD_U128);
    let amount3 = U256::from_u128(&env, 600 * WAD_U128);
    let amount4 = U256::from_u128(&env, 700 * WAD_U128);

    let lp_xlm_client = LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);
    let lp_usdc_client = LiquidityPoolUSDCClient::new(&env, &contracts.liquidity_pool_usdc);
    let lp_eurc_client = LiquidityPoolEURCClient::new(&env, &contracts.liquidity_pool_eurc);

    lp_xlm_client.initialize_pool_xlm(&contracts.vxlm_token_contract);
    lp_usdc_client.initialize_pool_usdc(&contracts.vusdc_token_contract);
    lp_eurc_client.initialize_pool_eurc(&contracts.veurc_token_contract);

    lp_xlm_client.deposit_xlm(&lender_addr1, &amount1);
    lp_xlm_client.deposit_xlm(&lender_addr2, &amount2);
    lp_xlm_client.deposit_xlm(&lender_addr3, &amount3);
    lp_xlm_client.deposit_xlm(&lender_addr4, &amount4);

    lp_usdc_client.deposit_usdc(&lender_addr1, &amount1);
    lp_usdc_client.deposit_usdc(&lender_addr2, &amount2);
    lp_usdc_client.deposit_usdc(&lender_addr3, &amount3);
    lp_usdc_client.deposit_usdc(&lender_addr4, &amount4);

    lp_eurc_client.deposit_eurc(&lender_addr1, &amount1);
    lp_eurc_client.deposit_eurc(&lender_addr2, &amount2);
    lp_eurc_client.deposit_eurc(&lender_addr3, &amount3);
    lp_eurc_client.deposit_eurc(&lender_addr4, &amount4);

    let xlm_token_client = token::TokenClient::new(&env, &contracts.xlm_address);
    println!(
        "Balance after depositing: {:?}",
        xlm_token_client.balance(&lender_addr1)
    );
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

#[test]
fn constructor_sets_admin_and_registry() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let registry = Address::generate(&env);
    let contract_id = Address::generate(&env);

    env.register_at(
        &contract_id,
        RiskEngineContract,
        (admin.clone(), registry.clone()),
    );

    as_auth(&env, &contract_id, || {
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&RiskEngineKey::Admin)
            .unwrap();
        let stored_registry: Address = env
            .storage()
            .persistent()
            .get(&RiskEngineKey::RegistryContract)
            .unwrap();

        assert_eq!(stored_admin, admin);
        assert_eq!(stored_registry, registry);
    });
}

#[test]
fn withdraw_allowed_returns_true_when_no_debt() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();

    let usdc_symbol = USDC_SYMBOL;
    let xlm_symbol = XLM_SYMBOL;

    let sa_client =
        SmartAccountContractClient::new(&env, &ctx.smart_account_contract.clone().unwrap());
    sa_client.set_has_debt(&false);

    // Oracle data won't even be used because function exits early
    let risk_client = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let ok = risk_client.is_withdraw_allowed(
        &usdc_symbol,
        &U256::from_u32(&env, 1_000),
        &ctx.smart_account_contract.unwrap(),
    );
    assert!(ok, "should allow withdraw when account has no debt");
}

#[test]
#[should_panic(expected = "\"failing with contract error\", 2")]
fn withdraw_allowed_panics_if_balance_or_debt_queries_error() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    let xlm_symbol = XLM_SYMBOL;
    env.mock_all_auths();

    let sa_client =
        SmartAccountContractClient::new(&env, &ctx.smart_account_contract.clone().unwrap());
    sa_client.set_has_debt(&true); // path that executes full logic

    // // Setup lists so get_current_total_* iterate, but break the oracle by missing price for symbol → panic
    let sym = Symbol::new(&env, "BTC");

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    // No price set for BTC → MockOracle.get_price_latest will panic →
    // is_withdraw_allowed unwraps get_current_total_balance/borrows → overall panic
    let _ = risk.is_withdraw_allowed(
        &sym,
        &U256::from_u32(&env, 1_000),
        &ctx.smart_account_contract.unwrap(),
    );
}

#[test]
fn get_current_total_borrows_sums_prices_and_handles_empty_list() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();
    initialise_lenders(&env, &ctx);

    // Empty borrowed list → expect 0
    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let zero = risk.get_current_total_borrows(&ctx.smart_account_contract.clone().unwrap());
    assert_eq!(zero, U256::from_u32(&env, 0));

    let account_manager_client =
        AccountManagerContractClient::new(&env, &ctx.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&USDC_SYMBOL);

    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &ctx.usdc_address);
    usdc_token.mint(&trader, &LARGE_AMOUNT);

    let smart_acc = account_manager_client.create_account(&trader);
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &USDC_SYMBOL,
        &U256::from_u128(&env, 100 * WAD_U128),
    );

    // Borrow & then settle: settle_account calls repay for each borrowed token
    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 5 * WAD_U128),
        &XLM_SYMBOL,
    );

    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 10 * WAD_U128),
        &EURC_SYMBOL,
    );

    let total = risk.get_current_total_borrows(&smart_acc);
    // Expected = (4_000_000 * 5) + (12262415 * 10) with mul_wad_down using same WAD denominator cancels out → 142624150
    assert_eq!(total, U256::from_u128(&env, 142624150 * 100000000000));
}

#[test]
fn borrow_allowed_returns_true_when_healthy() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();
    initialise_lenders(&env, &ctx);
    let sym = XLM_SYMBOL;

    let account_manager_client =
        AccountManagerContractClient::new(&env, &ctx.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&USDC_SYMBOL);

    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &ctx.usdc_address);
    usdc_token.mint(&trader, &LARGE_AMOUNT);

    let smart_acc = account_manager_client.create_account(&trader);

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let zero = risk.get_current_total_borrows(&smart_acc);
    assert_eq!(zero, U256::from_u32(&env, 0));

    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &USDC_SYMBOL,
        &U256::from_u128(&env, 100 * WAD_U128),
    );

    let risk_client = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let result = risk_client.is_borrow_allowed(
        &XLM_SYMBOL,
        &U256::from_u128(&env, 1 * WAD_U128),
        &smart_acc,
    );
    assert!(result, "borrow should be allowed for healthy account");
}

#[test]
fn borrow_disallowed_when_health_drops_below_threshold() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();
    initialise_lenders(&env, &ctx);
    let sym = XLM_SYMBOL;

    let account_manager_client =
        AccountManagerContractClient::new(&env, &ctx.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&USDC_SYMBOL);

    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &ctx.usdc_address);
    usdc_token.mint(&trader, &LARGE_AMOUNT);

    let smart_acc = account_manager_client.create_account(&trader);

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let zero = risk.get_current_total_borrows(&smart_acc);
    assert_eq!(zero, U256::from_u32(&env, 0));

    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &USDC_SYMBOL,
        &U256::from_u128(&env, 10 * WAD_U128),
    );

    let risk_client = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let result = risk_client.is_borrow_allowed(
        &XLM_SYMBOL,
        &U256::from_u128(&env, 300 * WAD_U128),
        &smart_acc,
    );
    assert!(!result, "borrow should be disallowed if ratio < threshold");
}

#[test]
#[should_panic(expected = "Cannot withdraw more value than the current collateral value")]
fn withdraw_panics_when_exceeding_collateral() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();
    initialise_lenders(&env, &ctx);
    let sym = XLM_SYMBOL;

    let account_manager_client =
        AccountManagerContractClient::new(&env, &ctx.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&USDC_SYMBOL);

    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &ctx.usdc_address);
    usdc_token.mint(&trader, &LARGE_AMOUNT);

    let smart_acc = account_manager_client.create_account(&trader);

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let zero = risk.get_current_total_borrows(&smart_acc);
    assert_eq!(zero, U256::from_u32(&env, 0));

    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &USDC_SYMBOL,
        &U256::from_u128(&env, 10 * WAD_U128),
    );

    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 10 * WAD_U128),
        &XLM_SYMBOL,
    );

    let risk_client = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let result = risk_client.is_withdraw_allowed(
        &USDC_SYMBOL,
        &U256::from_u128(&env, 3000 * WAD_U128),
        &smart_acc,
    );
}

#[test]
fn mul_wad_down_handles_large_values() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);

    let large = U256::from_u128(&env, u128::MAX / 10);
    let res = risk.mul_wad_down(&large, &large);
    println!("Res {:?}", res);
    assert!(res > U256::from_u128(&env, u128::MAX));
}

#[test]
fn is_account_healthy_handles_zero_debt_and_threshold_logic() {
    let env = Env::default();
    let ctx = test_initiation(&env);

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);

    // Zero debt → healthy
    assert!(risk.is_account_healthy(&U256::from_u32(&env, 0), &U256::from_u32(&env, 0)));

    // Balance / Debt > threshold → healthy, Just keep balance 1 above threshold
    let bal = U256::from_u128(&env, BALANCE_TO_BORROW_THRESHOLD as u128 + 1);
    let debt = U256::from_u128(&env, 1 * WAD_U128);
    assert!(risk.is_account_healthy(&bal, &debt));

    // Balance / Debt <= threshold → unhealthy (false) Just keep balance 1 below threshold
    let bal2 = U256::from_u128(&env, BALANCE_TO_BORROW_THRESHOLD as u128 - 1);
    let debt2 = U256::from_u128(&env, 1 * WAD_U128);
    assert!(!risk.is_account_healthy(&bal2, &debt2));
}

#[test]
fn mul_wad_down_basic_properties() {
    let env = Env::default();
    let ctx = test_initiation(&env);

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);

    // a * 1 == a (down)
    let a = U256::from_u128(&env, 123_456_789);
    let one = U256::from_u128(&env, WAD_U128 as u128);
    assert_eq!(risk.mul_wad_down(&a, &one), a);

    // 0 * b == 0
    let zero = U256::from_u32(&env, 0);
    assert_eq!(risk.mul_wad_down(&zero, &a), zero);
}

#[test]
#[should_panic(expected = "trying to get non-existing value for contract instance")]
fn borrow_allowed_panics_if_oracle_contract_missing_in_registry() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();

    let sym = XLM_SYMBOL;
    let registry_client = RegistryContractClient::new(&env, &ctx.registry_contract);

    // Placing some fake oracle address to panic
    registry_client.set_oracle_contract_address(&Address::generate(&env));

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let _ = risk.is_borrow_allowed(
        &sym,
        &U256::from_u32(&env, 1),
        &ctx.smart_account_contract.unwrap(),
    );
}

#[test]
fn withdraw_allowed_respects_health_check_false() {
    let env = Env::default();
    let ctx = test_initiation(&env);
    env.mock_all_auths();
    initialise_lenders(&env, &ctx);
    let sym = XLM_SYMBOL;

    let sa_client =
        SmartAccountContractClient::new(&env, &ctx.smart_account_contract.clone().unwrap());
    sa_client.set_has_debt(&true);

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    let zero = risk.get_current_total_borrows(&ctx.smart_account_contract.clone().unwrap());
    assert_eq!(zero, U256::from_u32(&env, 0));

    let account_manager_client =
        AccountManagerContractClient::new(&env, &ctx.account_manager_contract);
    account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
    account_manager_client.set_iscollateral_allowed(&USDC_SYMBOL);

    let trader = Addr::generate(&env);
    let usdc_token = StellarAssetClient::new(&env, &ctx.usdc_address);
    usdc_token.mint(&trader, &LARGE_AMOUNT);

    let smart_acc = account_manager_client.create_account(&trader);
    account_manager_client.deposit_collateral_tokens(
        &smart_acc,
        &USDC_SYMBOL,
        &U256::from_u128(&env, 100 * WAD_U128),
    );

    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 5 * WAD_U128),
        &XLM_SYMBOL,
    );

    account_manager_client.borrow(
        &smart_acc,
        &U256::from_u128(&env, 10 * WAD_U128),
        &EURC_SYMBOL,
    );

    let risk = RiskEngineContractClient::new(&env, &ctx.risk_engine_contract);
    // Try withdrawing a tiny amount → health will drop below threshold → expect false (but not panic)
    let allowed = risk.is_withdraw_allowed(
        &EURC_SYMBOL,
        &U256::from_u128(&env, 69 * WAD_U128),
        &smart_acc,
    );
    assert!(
        !allowed,
        "withdraw should be disallowed when it breaks health"
    );
}

#[test]
fn extend_ttl_risk_sets_ttl_properly() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let reg = Address::generate(&env);
    let id = Address::generate(&env);

    env.register_at(&id, RiskEngineContract, (admin.clone(), reg.clone()));
    as_auth(&env, &id, || {
        let ttl = env.storage().persistent().get_ttl(&RiskEngineKey::Admin);
        println!("TTL IS {:?}", ttl);
        assert!(
            ttl >= 6_307_200,
            "TTL should be extended to at least 1 year"
        );
    });
}
