#![cfg(test)]

use account_manager_contract::account_manager::{
    AccountManagerContract, AccountManagerContractClient,
    smart_account_contract::SmartAccExternalAction,
};
use account_manager_contract::types::ExternalProtocolCall;
use blend_contract_sdk::pool::{
    PoolConfig, Positions, Request, Reserve, ReserveConfig, ReserveData,
};
use registry_contract::registry::{RegistryContract, RegistryContractClient};
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{
    Address, Bytes, Env, Map, String, Symbol, U256, Vec, contract, contractimpl, contracttype,
    symbol_short, testutils::Address as _, token::StellarAssetClient,
};
use tracking_token_contract::tracking_token::{TrackingToken, TrackingTokenClient};

const SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

const WAD_U128: u128 = 10000_0000_00000_00000;
const SCALAR_12: i128 = 1_000_000_000_000;

const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");

const BLEND_XLM: &str = "BLEND_XLM";
const BLEND_USDC: &str = "BLEND_USDC";
const BLEND_EURC: &str = "BLEND_EURC";

#[derive(Clone)]
#[contracttype]
enum MockPoolKey {
    Admin,
    AssetIndex(Address),
    AssetDecimals(Address),
    SupplyPositions(Address),
}

#[contract]
pub struct MockBlendPool;

#[contractimpl]
impl MockBlendPool {
    pub fn init(env: Env, admin: Address, assets: Vec<Address>, decimals: Vec<u32>) {
        admin.require_auth();
        env.storage().persistent().set(&MockPoolKey::Admin, &admin);
        for (idx, asset) in assets.iter().enumerate() {
            let dec = decimals.get(idx as u32).unwrap_or(7);
            env.storage()
                .persistent()
                .set(&MockPoolKey::AssetIndex(asset.clone()), &(idx as u32));
            env.storage()
                .persistent()
                .set(&MockPoolKey::AssetDecimals(asset.clone()), &dec);
        }
    }

    pub fn get_config(env: Env) -> PoolConfig {
        let admin: Address = env.storage().persistent().get(&MockPoolKey::Admin).unwrap();
        PoolConfig {
            oracle: admin,
            min_collateral: 0,
            bstop_rate: 0,
            status: 0,
            max_positions: 4,
        }
    }

    pub fn get_reserve(env: Env, asset: Address) -> Reserve {
        let index: u32 = env
            .storage()
            .persistent()
            .get(&MockPoolKey::AssetIndex(asset.clone()))
            .unwrap_or(0);
        let decimals: u32 = env
            .storage()
            .persistent()
            .get(&MockPoolKey::AssetDecimals(asset.clone()))
            .unwrap_or(7);
        let config = ReserveConfig {
            index,
            decimals,
            c_factor: 0,
            l_factor: 0,
            util: 0,
            max_util: 100_0000000,
            r_base: 0,
            r_one: 0,
            r_two: 0,
            r_three: 0,
            reactivity: 0,
            supply_cap: i128::MAX,
            enabled: true,
        };
        let data = ReserveData {
            d_rate: SCALAR_12,
            b_rate: SCALAR_12,
            ir_mod: 0,
            b_supply: 0,
            d_supply: 0,
            backstop_credit: 0,
            last_time: env.ledger().timestamp(),
        };
        Reserve {
            asset,
            config,
            data,
            scalar: 10i128.pow(decimals),
        }
    }

    pub fn get_positions(env: Env, address: Address) -> Positions {
        let supply: Map<u32, i128> = env
            .storage()
            .persistent()
            .get(&MockPoolKey::SupplyPositions(address))
            .unwrap_or_else(|| Map::new(&env));
        Positions {
            liabilities: Map::new(&env),
            collateral: Map::new(&env),
            supply,
        }
    }

    pub fn submit(
        env: Env,
        from: Address,
        _spender: Address,
        _to: Address,
        requests: Vec<Request>,
    ) -> Positions {
        let mut supply: Map<u32, i128> = env
            .storage()
            .persistent()
            .get(&MockPoolKey::SupplyPositions(from.clone()))
            .unwrap_or_else(|| Map::new(&env));

        for req in requests.iter() {
            let index: u32 = env
                .storage()
                .persistent()
                .get(&MockPoolKey::AssetIndex(req.address.clone()))
                .unwrap_or(0);
            let current = supply.get(index).unwrap_or(0);
            if req.request_type == 0 {
                supply.set(index, current + req.amount);
            } else if req.request_type == 1 {
                let burn = if req.amount > current {
                    current
                } else {
                    req.amount
                };
                let new_bal = current - burn;
                if new_bal == 0 {
                    supply.remove(index);
                } else {
                    supply.set(index, new_bal);
                }
            } else {
                panic!("Unsupported request type in mock pool");
            }
        }

        env.storage()
            .persistent()
            .set(&MockPoolKey::SupplyPositions(from.clone()), &supply);

        Positions {
            liabilities: Map::new(&env),
            collateral: Map::new(&env),
            supply,
        }
    }
}

struct TestContext {
    env: Env,
    admin: Address,
    user: Address,
    registry: Address,
    account_manager: Address,
    blend_pool: Address,
    tracking_token: Address,
    xlm: Address,
    usdc: Address,
    eurc: Address,
}

fn scale_wad_to_token(amount_wad: u128, decimals: u32) -> i128 {
    ((amount_wad * 10u128.pow(decimals)) / WAD_U128) as i128
}

fn build_external_call(
    env: &Env,
    protocol: Address,
    action: SmartAccExternalAction,
    token_symbol: Symbol,
    amount_wad: u128,
    smart_account: Address,
) -> Bytes {
    let mut tokens_out = Vec::new(env);
    tokens_out.push_back(token_symbol);

    let mut amount_out = Vec::new(env);
    amount_out.push_back(U256::from_u128(env, amount_wad));

    let call = ExternalProtocolCall {
        protocol_address: protocol,
        type_action: action,
        tokens_out,
        tokens_in: Vec::new(env),
        amount_out,
        amount_in: Vec::new(env),
        is_token_pair: false,
        token_pair_ratio: 0,
        margin_account: smart_account,
        fee_fraction: 0,
        min_liquidity_out: U256::from_u128(env, 0),
    };

    call.to_xdr(env)
}

fn setup() -> TestContext {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let registry = Address::generate(&env);
    let account_manager = Address::generate(&env);

    env.register_at(&registry, RegistryContract, (admin.clone(),));
    env.register_at(
        &account_manager,
        AccountManagerContract,
        (admin.clone(), registry.clone()),
    );

    let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());
    let eurc_token = env.register_stellar_asset_contract_v2(admin.clone());

    let blend_pool = env.register(MockBlendPool, ());
    let mut assets = Vec::new(&env);
    assets.push_back(xlm_token.address());
    assets.push_back(usdc_token.address());
    assets.push_back(eurc_token.address());

    let mut decimals = Vec::new(&env);
    decimals.push_back(StellarAssetClient::new(&env, &xlm_token.address()).decimals());
    decimals.push_back(StellarAssetClient::new(&env, &usdc_token.address()).decimals());
    decimals.push_back(StellarAssetClient::new(&env, &eurc_token.address()).decimals());

    let blend_pool_client = MockBlendPoolClient::new(&env, &blend_pool);
    blend_pool_client.init(&admin, &assets, &decimals);

    let tracking_token = env.register(TrackingToken, ());
    let tracking_client = TrackingTokenClient::new(&env, &tracking_token);
    tracking_client.initialize(
        &account_manager,
        &Symbol::new(&env, BLEND_XLM),
        &StellarAssetClient::new(&env, &xlm_token.address()).decimals(),
        &String::from_str(&env, "BLEND XLM"),
    );
    tracking_client.initialize(
        &account_manager,
        &Symbol::new(&env, BLEND_USDC),
        &StellarAssetClient::new(&env, &usdc_token.address()).decimals(),
        &String::from_str(&env, "BLEND USDC"),
    );
    tracking_client.initialize(
        &account_manager,
        &Symbol::new(&env, BLEND_EURC),
        &StellarAssetClient::new(&env, &eurc_token.address()).decimals(),
        &String::from_str(&env, "BLEND EURC"),
    );

    let registry_client = RegistryContractClient::new(&env, &registry);
    let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
    registry_client.set_smart_account_hash(&smart_hash);
    registry_client.set_native_xlm_contract_address(&xlm_token.address());
    registry_client.set_native_usdc_contract_address(&usdc_token.address());
    registry_client.set_native_eurc_contract_address(&eurc_token.address());
    registry_client.set_blend_pool_address(&blend_pool);
    registry_client.set_tracking_token_contract_addr(&tracking_token);
    registry_client.set_accountmanager_contract(&account_manager);

    TestContext {
        env,
        admin,
        user,
        registry,
        account_manager,
        blend_pool,
        tracking_token,
        xlm: xlm_token.address(),
        usdc: usdc_token.address(),
        eurc: eurc_token.address(),
    }
}

#[test]
fn execute_deposit_and_withdraw_usdc_mints_and_burns_tracking_tokens() {
    let ctx = setup();

    let account_manager_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = account_manager_client.create_account(&ctx.user);

    let deposit_wad = 100u128 * WAD_U128;
    let call_bytes = build_external_call(
        &ctx.env,
        ctx.blend_pool.clone(),
        SmartAccExternalAction::Deposit,
        USDC_SYMBOL,
        deposit_wad,
        smart_account.clone(),
    );
    account_manager_client.execute(&smart_account, &call_bytes);

    let usdc_decimals = StellarAssetClient::new(&ctx.env, &ctx.usdc).decimals();
    let expected_minted = scale_wad_to_token(deposit_wad, usdc_decimals);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let tracking_symbol = Symbol::new(&ctx.env, BLEND_USDC);
    assert_eq!(
        tracking_client.balance(&smart_account, &tracking_symbol),
        expected_minted
    );

    let pool_client = blend_contract_sdk::pool::Client::new(&ctx.env, &ctx.blend_pool);
    let reserve = pool_client.get_reserve(&ctx.usdc);
    let positions = pool_client.get_positions(&smart_account);
    assert_eq!(
        positions.supply.get(reserve.config.index).unwrap_or(0),
        expected_minted
    );

    let withdraw_wad = 40u128 * WAD_U128;
    let withdraw_bytes = build_external_call(
        &ctx.env,
        ctx.blend_pool.clone(),
        SmartAccExternalAction::Withdraw,
        USDC_SYMBOL,
        withdraw_wad,
        smart_account.clone(),
    );
    account_manager_client.execute(&smart_account, &withdraw_bytes);

    let expected_burn = scale_wad_to_token(withdraw_wad, usdc_decimals);
    assert_eq!(
        tracking_client.balance(&smart_account, &tracking_symbol),
        expected_minted - expected_burn
    );
}

#[test]
fn execute_deposit_and_withdraw_xlm_tracks_supply_position() {
    let ctx = setup();

    let account_manager_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = account_manager_client.create_account(&ctx.user);

    let deposit_wad = 25u128 * WAD_U128;
    let call_bytes = build_external_call(
        &ctx.env,
        ctx.blend_pool.clone(),
        SmartAccExternalAction::Deposit,
        XLM_SYMBOL,
        deposit_wad,
        smart_account.clone(),
    );
    account_manager_client.execute(&smart_account, &call_bytes);

    let xlm_decimals = StellarAssetClient::new(&ctx.env, &ctx.xlm).decimals();
    let expected_minted = scale_wad_to_token(deposit_wad, xlm_decimals);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let tracking_symbol = Symbol::new(&ctx.env, BLEND_XLM);
    assert_eq!(
        tracking_client.balance(&smart_account, &tracking_symbol),
        expected_minted
    );

    let pool_client = blend_contract_sdk::pool::Client::new(&ctx.env, &ctx.blend_pool);
    let reserve = pool_client.get_reserve(&ctx.xlm);
    let positions = pool_client.get_positions(&smart_account);
    assert_eq!(
        positions.supply.get(reserve.config.index).unwrap_or(0),
        expected_minted
    );

    let withdraw_wad = 10u128 * WAD_U128;
    let withdraw_bytes = build_external_call(
        &ctx.env,
        ctx.blend_pool.clone(),
        SmartAccExternalAction::Withdraw,
        XLM_SYMBOL,
        withdraw_wad,
        smart_account.clone(),
    );
    account_manager_client.execute(&smart_account, &withdraw_bytes);

    let expected_burn = scale_wad_to_token(withdraw_wad, xlm_decimals);
    assert_eq!(
        tracking_client.balance(&smart_account, &tracking_symbol),
        expected_minted - expected_burn
    );
}

// ============================================================================
// Aquarius Protocol Integration Tests
// ============================================================================

const AQUARIUS_XLM_USDC: &str = "AQ_XLM_U";

#[derive(Clone)]
#[contracttype]
enum MockAquariusKey {
    Admin,
    PoolIndex,
    LPBalance(Address),
    TokenBalance(Address, Address), // (user, token)
}

#[contract]
pub struct MockAquariusRouter;

#[contractimpl]
impl MockAquariusRouter {
    pub fn init(env: Env, admin: Address) {
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&MockAquariusKey::Admin, &admin);
    }

    pub fn init_standard_pool(
        env: Env,
        sender: Address,
        tokens: Vec<Address>,
        fee_fraction: u32,
    ) -> (soroban_sdk::BytesN<32>, Address) {
        let pool_index = env.crypto().sha256(&tokens.to_xdr(&env));
        env.storage()
            .persistent()
            .set(&MockAquariusKey::PoolIndex, &pool_index);
        (soroban_sdk::BytesN::from_array(&env, &pool_index.to_array()), sender.clone())
    }

    pub fn deposit(
        env: Env,
        sender: Address,
        tokens: Vec<Address>,
        pool_id: soroban_sdk::BytesN<32>,
        desired_amounts: Vec<u128>,
        min_shares: u128,
    ) -> (Vec<u128>, u128) {
        // Simple mock: LP tokens = average of deposited amounts
        let amount0 = desired_amounts.get(0).unwrap();
        let amount1 = desired_amounts.get(1).unwrap();
        let lp_tokens = (amount0 + amount1) / 2;

        let current_lp = env
            .storage()
            .persistent()
            .get(&MockAquariusKey::LPBalance(sender.clone()))
            .unwrap_or(0u128);

        env.storage()
            .persistent()
            .set(&MockAquariusKey::LPBalance(sender), &(current_lp + lp_tokens));

        (desired_amounts, lp_tokens)
    }

    pub fn withdraw(
        env: Env,
        sender: Address,
        tokens: Vec<Address>,
        pool_id: soroban_sdk::BytesN<32>,
        share_amount: u128,
        min_amounts: Vec<u128>,
    ) -> Vec<u128> {
        let current_lp = env
            .storage()
            .persistent()
            .get(&MockAquariusKey::LPBalance(sender.clone()))
            .unwrap_or(0u128);

        if share_amount > current_lp {
            panic!("Insufficient LP tokens");
        }

        env.storage()
            .persistent()
            .set(
                &MockAquariusKey::LPBalance(sender),
                &(current_lp - share_amount),
            );

        // Return proportional amounts
        soroban_sdk::vec![&env, share_amount, share_amount]
    }

    pub fn swap(
        env: Env,
        sender: Address,
        tokens: Vec<Address>,
        token_in: Address,
        token_out: Address,
        pool_id: soroban_sdk::BytesN<32>,
        amount_in: u128,
        min_amount_out: u128,
    ) -> u128 {
        // Simple mock: 1:1 swap with 0.3% fee
        let amount_out = (amount_in * 997) / 1000;
        amount_out
    }

    pub fn get_lp_balance(env: Env, sender: Address) -> u128 {
        env.storage()
            .persistent()
            .get(&MockAquariusKey::LPBalance(sender))
            .unwrap_or(0u128)
    }
}

fn build_aquarius_add_liquidity_call(
    env: &Env,
    router: Address,
    token0: Symbol,
    token1: Symbol,
    amount0_wad: u128,
    amount1_wad: u128,
    smart_account: Address,
) -> Bytes {
    let mut tokens_out = Vec::new(env);
    tokens_out.push_back(token0);
    tokens_out.push_back(token1);

    let mut amount_out = Vec::new(env);
    amount_out.push_back(U256::from_u128(env, amount0_wad));
    amount_out.push_back(U256::from_u128(env, amount1_wad));

    let call = ExternalProtocolCall {
        protocol_address: router,
        type_action: SmartAccExternalAction::AddLiquidity,
        tokens_out,
        tokens_in: Vec::new(env),
        amount_out,
        amount_in: Vec::new(env),
        is_token_pair: true,
        token_pair_ratio: 0,
        margin_account: smart_account,
        fee_fraction: 30u32,
        min_liquidity_out: U256::from_u128(env, 0),
    };

    call.to_xdr(env)
}

fn build_aquarius_remove_liquidity_call(
    env: &Env,
    router: Address,
    token0: Symbol,
    token1: Symbol,
    lp_amount: u128,
    smart_account: Address,
) -> Bytes {
    let mut tokens_out = Vec::new(env);
    tokens_out.push_back(token0);
    tokens_out.push_back(token1);

    let mut amount_out = Vec::new(env);
    amount_out.push_back(U256::from_u128(env, lp_amount));

    let call = ExternalProtocolCall {
        protocol_address: router,
        type_action: SmartAccExternalAction::RemoveLiquidity,
        tokens_out,
        tokens_in: Vec::new(env),
        amount_out,
        amount_in: Vec::new(env),
        is_token_pair: true,
        token_pair_ratio: 0,
        margin_account: smart_account,
        fee_fraction: 30u32,
        min_liquidity_out: U256::from_u128(env, 0),
    };

    call.to_xdr(env)
}

fn build_aquarius_swap_call(
    env: &Env,
    router: Address,
    token_in: Symbol,
    token_out: Symbol,
    amount_in_wad: u128,
    smart_account: Address,
) -> Bytes {
    let mut tokens_out = Vec::new(env);
    tokens_out.push_back(token_in);
    tokens_out.push_back(token_out);

    let mut amount_out = Vec::new(env);
    amount_out.push_back(U256::from_u128(env, amount_in_wad));

    let call = ExternalProtocolCall {
        protocol_address: router,
        type_action: SmartAccExternalAction::Swap,
        tokens_out,
        tokens_in: Vec::new(env),
        amount_out,
        amount_in: Vec::new(env),
        is_token_pair: false,
        token_pair_ratio: 0,
        margin_account: smart_account,
        fee_fraction: 30u32,
        min_liquidity_out: U256::from_u128(env, 0),
    };

    call.to_xdr(env)
}

struct AquariusTestContext {
    env: Env,
    admin: Address,
    user: Address,
    registry: Address,
    account_manager: Address,
    aquarius_router: Address,
    tracking_token: Address,
    xlm: Address,
    usdc: Address,
}

fn setup_aquarius() -> AquariusTestContext {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let registry = Address::generate(&env);
    let account_manager = Address::generate(&env);

    env.register_at(&registry, RegistryContract, (admin.clone(),));
    env.register_at(
        &account_manager,
        AccountManagerContract,
        (admin.clone(), registry.clone()),
    );

    let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());

    let aquarius_router = env.register(MockAquariusRouter, ());
    let aquarius_router_client = MockAquariusRouterClient::new(&env, &aquarius_router);
    aquarius_router_client.init(&admin);

    // Init pool
    let mut tokens = Vec::new(&env);
    tokens.push_back(xlm_token.address());
    tokens.push_back(usdc_token.address());
    let (pool_index, _) = aquarius_router_client.init_standard_pool(&admin, &tokens, &30u32);

    let tracking_token = env.register(TrackingToken, ());
    let tracking_client = TrackingTokenClient::new(&env, &tracking_token);

    tracking_client.initialize(
        &account_manager,
        &Symbol::new(&env, AQUARIUS_XLM_USDC),
        &7u32,
        &String::from_str(&env, "Aquarius XLM-USDC LP"),
    );

    let registry_client = RegistryContractClient::new(&env, &registry);
    let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
    registry_client.set_smart_account_hash(&smart_hash);
    registry_client.set_native_xlm_contract_address(&xlm_token.address());
    registry_client.set_native_usdc_contract_address(&usdc_token.address());
    registry_client.set_aquarius_router_address(&aquarius_router);
    registry_client.set_aquarius_pool_index(&pool_index);
    registry_client.set_tracking_token_contract_addr(&tracking_token);
    registry_client.set_accountmanager_contract(&account_manager);

    AquariusTestContext {
        env,
        admin,
        user,
        registry,
        account_manager,
        aquarius_router,
        tracking_token,
        xlm: xlm_token.address(),
        usdc: usdc_token.address(),
    }
}

#[test]
fn test_aquarius_add_liquidity_mints_lp_tracking_tokens() {
    let ctx = setup_aquarius();

    let account_manager_client =
        AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = account_manager_client.create_account(&ctx.user);

    // Add liquidity: 1000 XLM + 1000 USDC
    let xlm_amount_wad = 1000u128 * WAD_U128;
    let usdc_amount_wad = 1000u128 * WAD_U128;

    let call_bytes = build_aquarius_add_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_amount_wad,
        usdc_amount_wad,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &call_bytes);

    // Verify LP tracking tokens were minted
    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_tracking_symbol = Symbol::new(&ctx.env, AQUARIUS_XLM_USDC);
    let lp_balance = tracking_client.balance(&smart_account, &lp_tracking_symbol);

    assert!(lp_balance > 0);
    // Mock returns average of token amounts: (1000*10^7 + 1000*10^7) / 2 = 10^10
    assert_eq!(lp_balance, 10_000_000_000); 
}

#[test]
fn test_aquarius_remove_liquidity_burns_lp_tracking_tokens() {
    let ctx = setup_aquarius();

    let account_manager_client =
        AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = account_manager_client.create_account(&ctx.user);

    // Add liquidity first
    let xlm_amount_wad = 2000u128 * WAD_U128;
    let usdc_amount_wad = 2000u128 * WAD_U128;

    let add_call = build_aquarius_add_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_amount_wad,
        usdc_amount_wad,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &add_call);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_tracking_symbol = Symbol::new(&ctx.env, AQUARIUS_XLM_USDC);
    let initial_lp = tracking_client.balance(&smart_account, &lp_tracking_symbol);

    // Remove half the liquidity
    let remove_amount = initial_lp / 2;
    let remove_call = build_aquarius_remove_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        remove_amount as u128,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &remove_call);

    // Verify LP tracking tokens were burned
    let final_lp = tracking_client.balance(&smart_account, &lp_tracking_symbol);
    assert_eq!(final_lp, initial_lp - remove_amount);
}

#[test]
fn test_aquarius_full_flow_add_swap_remove() {
    let ctx = setup_aquarius();

    let account_manager_client =
        AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = account_manager_client.create_account(&ctx.user);

    // Step 1: Add liquidity
    let xlm_amount_wad = 5000u128 * WAD_U128;
    let usdc_amount_wad = 5000u128 * WAD_U128;

    let add_call = build_aquarius_add_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_amount_wad,
        usdc_amount_wad,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &add_call);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_tracking_symbol = Symbol::new(&ctx.env, AQUARIUS_XLM_USDC);
    let lp_after_add = tracking_client.balance(&smart_account, &lp_tracking_symbol);
    assert!(lp_after_add > 0);

    // Step 2: Execute a swap (shouldn't affect LP tracking)
    let swap_call = build_aquarius_swap_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        1000u128 * WAD_U128,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &swap_call);

    let lp_after_swap = tracking_client.balance(&smart_account, &lp_tracking_symbol);
    assert_eq!(lp_after_swap, lp_after_add); // LP balance unchanged after swap

    // Step 3: Remove all liquidity
    let remove_call = build_aquarius_remove_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        lp_after_add as u128,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &remove_call);

    let lp_final = tracking_client.balance(&smart_account, &lp_tracking_symbol);
    assert_eq!(lp_final, 0);
}
