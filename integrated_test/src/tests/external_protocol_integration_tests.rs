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

const AQUARIUS_XLM_USDC: &str = "AQ_XLM_USDC";

#[derive(Clone)]
#[contracttype]
enum MockAquariusKey {
    Admin,
    PoolIndex,
    PoolAddress,
    ShareTokenAddress,
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
        let self_address = env.current_contract_address();
        env.storage()
            .persistent()
            .set(&MockAquariusKey::PoolIndex, &pool_index);
        env.storage()
            .persistent()
            .set(&MockAquariusKey::PoolAddress, &self_address);
        env.storage()
            .persistent()
            .set(&MockAquariusKey::ShareTokenAddress, &self_address);
        (soroban_sdk::BytesN::from_array(&env, &pool_index.to_array()), self_address.clone())
    }

    pub fn get_pool(
        env: Env,
        tokens: Vec<Address>,
        pool_id: soroban_sdk::BytesN<32>,
    ) -> Address {
        env.storage()
            .persistent()
            .get(&MockAquariusKey::PoolAddress)
            .unwrap_or_else(|| panic!("Mock pool address not set"))
    }

    pub fn share_id(
        env: Env,
        tokens: Vec<Address>,
        pool_id: soroban_sdk::BytesN<32>,
    ) -> Address {
        env.storage()
            .persistent()
            .get(&MockAquariusKey::ShareTokenAddress)
            .unwrap_or_else(|| panic!("Mock share token address not set"))
    }

    pub fn deposit(
        env: Env,
        user: Address,
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
            .get(&MockAquariusKey::LPBalance(user.clone()))
            .unwrap_or(0u128);

        env.storage()
            .persistent()
            .set(&MockAquariusKey::LPBalance(user), &(current_lp + lp_tokens));

        (desired_amounts, lp_tokens)
    }

    pub fn withdraw(
        env: Env,
        user: Address,
        share_amount: u128,
        min_amounts: Vec<u128>,
    ) -> Vec<u128> {
        let current_lp = env
            .storage()
            .persistent()
            .get(&MockAquariusKey::LPBalance(user.clone()))
            .unwrap_or(0u128);

        if share_amount > current_lp {
            panic!("Insufficient LP tokens");
        }

        env.storage()
            .persistent()
            .set(
                &MockAquariusKey::LPBalance(user),
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

#[test]
fn test_aquarius_single_token_to_liquidity_flow() {
    let ctx = setup_aquarius();

    let account_manager_client =
        AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = account_manager_client.create_account(&ctx.user);

    // User starts with only XLM (realistic scenario)
    let initial_xlm_wad = 10000u128 * WAD_U128;

    // Step 1: User swaps half of their XLM to get USDC
    let xlm_to_swap_wad = initial_xlm_wad / 2; // Swap 50% of XLM
    let swap_call = build_aquarius_swap_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_to_swap_wad,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &swap_call);

    // Step 2: Now user has both XLM and USDC, add liquidity
    let remaining_xlm_wad = initial_xlm_wad - xlm_to_swap_wad;
    // Assuming 1:1 swap ratio for simplicity (in real scenario would query pool)
    let received_usdc_wad = xlm_to_swap_wad;

    let add_liquidity_call = build_aquarius_add_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        remaining_xlm_wad,
        received_usdc_wad,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &add_liquidity_call);

    // Verify LP tokens were minted and tracked
    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_tracking_symbol = Symbol::new(&ctx.env, AQUARIUS_XLM_USDC);
    let lp_balance = tracking_client.balance(&smart_account, &lp_tracking_symbol);

    assert!(lp_balance > 0, "LP tracking tokens should be minted");

    // Step 3: Remove liquidity to complete the flow
    let remove_call = build_aquarius_remove_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        lp_balance as u128,
        smart_account.clone(),
    );

    account_manager_client.execute(&smart_account, &remove_call);

    let final_lp_balance = tracking_client.balance(&smart_account, &lp_tracking_symbol);
    assert_eq!(final_lp_balance, 0, "All LP tokens should be burned");
}

// ============================================================================
// Full Cycle: Create Account → Deposit Collateral → Borrow → Open Aquarius Position
// ============================================================================

#[derive(Clone)]
#[contracttype]
enum MockLendingPoolKey {
    BorrowBalance(Address),
}

#[contract]
pub struct MockLendingPool;

#[contractimpl]
impl MockLendingPool {
    pub fn lend_to(env: Env, borrower: Address, amount_wad: U256) -> bool {
        let current: U256 = env
            .storage()
            .persistent()
            .get(&MockLendingPoolKey::BorrowBalance(borrower.clone()))
            .unwrap_or(U256::from_u128(&env, 0));
        env.storage()
            .persistent()
            .set(&MockLendingPoolKey::BorrowBalance(borrower), &current.add(&amount_wad));
        true
    }

    pub fn get_borrow_balance(env: Env, borrower: Address) -> U256 {
        env.storage()
            .persistent()
            .get(&MockLendingPoolKey::BorrowBalance(borrower))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    pub fn get_user_borrow_shares(env: Env, borrower: Address) -> U256 {
        env.storage()
            .persistent()
            .get(&MockLendingPoolKey::BorrowBalance(borrower))
            .unwrap_or(U256::from_u128(&env, 0))
    }

    pub fn collect_from(env: Env, amount_wad: U256, borrower: Address) -> bool {
        let current: U256 = env
            .storage()
            .persistent()
            .get(&MockLendingPoolKey::BorrowBalance(borrower.clone()))
            .unwrap_or(U256::from_u128(&env, 0));
        let remaining = if amount_wad >= current {
            U256::from_u128(&env, 0)
        } else {
            current.sub(&amount_wad)
        };
        env.storage()
            .persistent()
            .set(&MockLendingPoolKey::BorrowBalance(borrower), &remaining);
        remaining == U256::from_u128(&env, 0)
    }
}

#[contract]
pub struct MockRiskEngine;

#[contractimpl]
impl MockRiskEngine {
    pub fn is_borrow_allowed(_env: Env, _symbol: Symbol, _amount: U256, _account: Address) -> bool {
        true
    }

    pub fn is_withdraw_allowed(_env: Env, _symbol: Symbol, _amount: U256, _account: Address) -> bool {
        true
    }
}

struct FullCycleTestContext {
    env: Env,
    user: Address,
    account_manager: Address,
    aquarius_router: Address,
    tracking_token: Address,
    xlm: Address,
    usdc: Address,
}

fn setup_full_cycle() -> FullCycleTestContext {
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

    // Mint XLM to user so deposit_collateral_tokens can transfer it (200 XLM, 7 decimals)
    StellarAssetClient::new(&env, &xlm_token.address()).mint(&user, &2_000_000_000i128);

    // Register Aquarius router and init XLM-USDC pool
    let aquarius_router = env.register(MockAquariusRouter, ());
    let aquarius_router_client = MockAquariusRouterClient::new(&env, &aquarius_router);
    aquarius_router_client.init(&admin);
    let mut tokens = Vec::new(&env);
    tokens.push_back(xlm_token.address());
    tokens.push_back(usdc_token.address());
    let (pool_index, _) = aquarius_router_client.init_standard_pool(&admin, &tokens, &30u32);

    // Register mock USDC lending pool and mock risk engine
    let lending_pool_usdc = env.register(MockLendingPool, ());
    let risk_engine = env.register(MockRiskEngine, ());

    // Register tracking token and initialise AQ_XLM_USDC symbol
    let tracking_token = env.register(TrackingToken, ());
    let tracking_client = TrackingTokenClient::new(&env, &tracking_token);
    tracking_client.initialize(
        &account_manager,
        &Symbol::new(&env, AQUARIUS_XLM_USDC),
        &7u32,
        &String::from_str(&env, "Aquarius XLM-USDC LP"),
    );

    // Configure registry
    let registry_client = RegistryContractClient::new(&env, &registry);
    let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
    registry_client.set_smart_account_hash(&smart_hash);
    registry_client.set_native_xlm_contract_address(&xlm_token.address());
    registry_client.set_native_usdc_contract_address(&usdc_token.address());
    registry_client.set_lendingpool_usdc(&lending_pool_usdc);
    registry_client.set_risk_engine_address(&risk_engine);
    registry_client.set_aquarius_router_address(&aquarius_router);
    registry_client.set_aquarius_pool_index(&pool_index);
    registry_client.set_tracking_token_contract_addr(&tracking_token);
    registry_client.set_accountmanager_contract(&account_manager);

    // Allow XLM as collateral and set asset cap on account manager
    let am_client = AccountManagerContractClient::new(&env, &account_manager);
    am_client.set_max_asset_cap(&U256::from_u128(&env, 10));
    am_client.set_iscollateral_allowed(&XLM_SYMBOL);

    FullCycleTestContext {
        env,
        user,
        account_manager,
        aquarius_router,
        tracking_token,
        xlm: xlm_token.address(),
        usdc: usdc_token.address(),
    }
}

#[test]
fn test_full_cycle_deposit_collateral_borrow_open_aquarius_position() {
    let ctx = setup_full_cycle();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);

    // Step 1: Create margin account
    let smart_account = am_client.create_account(&ctx.user);

    // Step 2: Deposit 100 XLM as collateral
    let collateral_wad = 100u128 * WAD_U128;
    am_client.deposit_collateral_tokens(
        &smart_account,
        &XLM_SYMBOL,
        &U256::from_u128(&ctx.env, collateral_wad),
    );

    // Verify collateral is recorded in the smart account (in WAD)
    let sa_client =
        account_manager_contract::account_manager::smart_account_contract::Client::new(
            &ctx.env,
            &smart_account,
        );
    assert_eq!(
        sa_client.get_collateral_token_balance(&XLM_SYMBOL),
        U256::from_u128(&ctx.env, collateral_wad),
        "Collateral balance must be recorded in WAD"
    );

    // Step 3: Borrow 50 USDC against the XLM collateral
    let borrow_wad = 50u128 * WAD_U128;
    am_client.borrow(
        &smart_account,
        &U256::from_u128(&ctx.env, borrow_wad),
        &USDC_SYMBOL,
    );

    // Verify the smart account now carries debt
    assert!(sa_client.has_debt(), "Smart account must have debt after borrowing");
    assert!(
        sa_client.get_all_borrowed_tokens().contains(USDC_SYMBOL),
        "USDC must appear in borrowed tokens list"
    );

    // Step 4: Open Aquarius position — add 40 XLM + 40 USDC liquidity
    let xlm_lp_wad = 40u128 * WAD_U128;
    let usdc_lp_wad = 40u128 * WAD_U128;
    let open_position_call = build_aquarius_add_liquidity_call(
        &ctx.env,
        ctx.aquarius_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_lp_wad,
        usdc_lp_wad,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &open_position_call);

    // Verify LP tracking tokens were minted
    // Mock LP formula: (scaled_xlm + scaled_usdc) / 2
    let xlm_scaled = scale_wad_to_token(xlm_lp_wad, 7);
    let usdc_scaled = scale_wad_to_token(usdc_lp_wad, 7);
    let expected_lp = (xlm_scaled + usdc_scaled) / 2;

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_symbol = Symbol::new(&ctx.env, AQUARIUS_XLM_USDC);
    let lp_balance = tracking_client.balance(&smart_account, &lp_symbol);

    assert!(lp_balance > 0, "LP tracking tokens must be minted after opening position");
    assert_eq!(lp_balance, expected_lp, "LP balance must match mock pool formula");

    // Verify the LP symbol is now tracked as collateral in the smart account
    assert!(
        sa_client.get_all_collateral_tokens().contains(lp_symbol),
        "Aquarius LP tracking symbol must be added to smart account collateral"
    );
}

// ============================================================================
// Soroswap Protocol Integration Tests
// ============================================================================

const SOROSWAP_XLM_USDC: &str = "SS_XLM_USDC";

// ---------------------------------------------------------------------------
// Mock Soroswap Router
// A self-contained mock that tracks LP balances internally.
// Uses mock_all_auths() so no actual token transfers occur.
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[contracttype]
enum MockSoroswapKey {
    Admin,
    PairAddress,
    LPBalance(Address),
}

#[contract]
pub struct MockSoroswapRouter;

#[contractimpl]
impl MockSoroswapRouter {
    /// Initialise the mock router with an admin and a dummy pair address.
    pub fn init(env: Env, admin: Address, pair_address: Address) {
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&MockSoroswapKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&MockSoroswapKey::PairAddress, &pair_address);
    }

    /// Add liquidity: LP = (amount_a + amount_b) / 2 for simplicity.
    pub fn add_liquidity(
        env: Env,
        _token_a: Address,
        _token_b: Address,
        amount_a_desired: i128,
        amount_b_desired: i128,
        _amount_a_min: i128,
        _amount_b_min: i128,
        to: Address,
        _deadline: u64,
    ) -> (i128, i128, i128) {
        let lp = (amount_a_desired + amount_b_desired) / 2;
        let current: i128 = env
            .storage()
            .persistent()
            .get(&MockSoroswapKey::LPBalance(to.clone()))
            .unwrap_or(0i128);
        env.storage()
            .persistent()
            .set(&MockSoroswapKey::LPBalance(to), &(current + lp));
        (amount_a_desired, amount_b_desired, lp)
    }

    /// Remove liquidity: burns LP tokens and returns equal token amounts.
    pub fn remove_liquidity(
        env: Env,
        _token_a: Address,
        _token_b: Address,
        liquidity: i128,
        _amount_a_min: i128,
        _amount_b_min: i128,
        to: Address,
        _deadline: u64,
    ) -> (i128, i128) {
        let current: i128 = env
            .storage()
            .persistent()
            .get(&MockSoroswapKey::LPBalance(to.clone()))
            .unwrap_or(0i128);
        if liquidity > current {
            panic!("Insufficient LP tokens in mock");
        }
        env.storage()
            .persistent()
            .set(&MockSoroswapKey::LPBalance(to), &(current - liquidity));
        (liquidity / 2, liquidity / 2)
    }

    /// Swap: mock 0.3% fee, returns [amount_in, amount_out].
    pub fn swap_exact_tokens_for_tokens(
        env: Env,
        amount_in: i128,
        _amount_out_min: i128,
        _path: Vec<Address>,
        _to: Address,
        _deadline: u64,
    ) -> Vec<i128> {
        let amount_out = (amount_in * 997) / 1000;
        soroban_sdk::vec![&env, amount_in, amount_out]
    }

    /// Returns the stored admin address as a stand-in for the factory.
    pub fn get_factory(env: Env) -> Address {
        env.storage()
            .persistent()
            .get(&MockSoroswapKey::Admin)
            .unwrap()
    }

    /// Returns the stored pair address deterministically.
    pub fn router_pair_for(env: Env, _token_a: Address, _token_b: Address) -> Address {
        env.storage()
            .persistent()
            .get(&MockSoroswapKey::PairAddress)
            .unwrap_or_else(|| panic!("Mock pair address not set"))
    }

    /// Helper for tests: query a user's internal LP balance.
    pub fn get_lp_balance(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&MockSoroswapKey::LPBalance(user))
            .unwrap_or(0i128)
    }
}

// ---------------------------------------------------------------------------
// Soroswap test helpers
// ---------------------------------------------------------------------------

fn build_soroswap_add_liquidity_call(
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

fn build_soroswap_remove_liquidity_call(
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

fn build_soroswap_swap_call(
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

// ---------------------------------------------------------------------------
// Soroswap test context and setup
// ---------------------------------------------------------------------------

struct SoroswapTestContext {
    env: Env,
    admin: Address,
    user: Address,
    account_manager: Address,
    soroswap_router: Address,
    tracking_token: Address,
    xlm: Address,
    usdc: Address,
}

fn setup_soroswap() -> SoroswapTestContext {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let registry = Address::generate(&env);
    let account_manager = Address::generate(&env);
    // Dummy pair address returned by router_pair_for
    let pair_address = Address::generate(&env);

    env.register_at(&registry, RegistryContract, (admin.clone(),));
    env.register_at(
        &account_manager,
        AccountManagerContract,
        (admin.clone(), registry.clone()),
    );

    let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());

    let soroswap_router = env.register(MockSoroswapRouter, ());
    let soroswap_router_client = MockSoroswapRouterClient::new(&env, &soroswap_router);
    soroswap_router_client.init(&admin, &pair_address);

    let tracking_token = env.register(TrackingToken, ());
    let tracking_client = TrackingTokenClient::new(&env, &tracking_token);
    tracking_client.initialize(
        &account_manager,
        &Symbol::new(&env, SOROSWAP_XLM_USDC),
        &7u32,
        &String::from_str(&env, "Soroswap XLM-USDC LP"),
    );

    let registry_client = RegistryContractClient::new(&env, &registry);
    let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);
    registry_client.set_smart_account_hash(&smart_hash);
    registry_client.set_native_xlm_contract_address(&xlm_token.address());
    registry_client.set_native_usdc_contract_address(&usdc_token.address());
    registry_client.set_soroswap_router_address(&soroswap_router);
    registry_client.set_tracking_token_contract_addr(&tracking_token);
    registry_client.set_accountmanager_contract(&account_manager);

    SoroswapTestContext {
        env,
        admin,
        user,
        account_manager,
        soroswap_router,
        tracking_token,
        xlm: xlm_token.address(),
        usdc: usdc_token.address(),
    }
}

// ---------------------------------------------------------------------------
// Soroswap tests
// ---------------------------------------------------------------------------

#[test]
fn test_soroswap_add_liquidity_mints_lp_tracking_tokens() {
    let ctx = setup_soroswap();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = am_client.create_account(&ctx.user);

    // Add 1000 XLM + 1000 USDC liquidity
    let xlm_wad = 1000u128 * WAD_U128;
    let usdc_wad = 1000u128 * WAD_U128;

    let call_bytes = build_soroswap_add_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_wad,
        usdc_wad,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &call_bytes);

    // Mock LP = (scaled_xlm + scaled_usdc) / 2 = (1000*10^7 + 1000*10^7) / 2 = 10^10
    let xlm_scaled = scale_wad_to_token(xlm_wad, 7);
    let usdc_scaled = scale_wad_to_token(usdc_wad, 7);
    let expected_lp = (xlm_scaled + usdc_scaled) / 2;

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_symbol = Symbol::new(&ctx.env, SOROSWAP_XLM_USDC);
    let lp_balance = tracking_client.balance(&smart_account, &lp_symbol);

    assert!(lp_balance > 0, "LP tracking tokens must be minted");
    assert_eq!(lp_balance, expected_lp, "LP balance must match mock formula");
}

#[test]
fn test_soroswap_remove_liquidity_burns_lp_tracking_tokens() {
    let ctx = setup_soroswap();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = am_client.create_account(&ctx.user);

    // First add liquidity
    let xlm_wad = 2000u128 * WAD_U128;
    let usdc_wad = 2000u128 * WAD_U128;

    let add_call = build_soroswap_add_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        xlm_wad,
        usdc_wad,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &add_call);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_symbol = Symbol::new(&ctx.env, SOROSWAP_XLM_USDC);
    let initial_lp = tracking_client.balance(&smart_account, &lp_symbol);
    assert!(initial_lp > 0);

    // Remove half the LP tokens
    let remove_amount = (initial_lp / 2) as u128;
    let remove_call = build_soroswap_remove_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        remove_amount,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &remove_call);

    let final_lp = tracking_client.balance(&smart_account, &lp_symbol);
    assert_eq!(
        final_lp,
        initial_lp - remove_amount as i128,
        "LP tracking tokens must be burned correctly"
    );
}

#[test]
fn test_soroswap_swap_does_not_change_lp_tracking() {
    let ctx = setup_soroswap();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = am_client.create_account(&ctx.user);

    // Add liquidity first
    let add_call = build_soroswap_add_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        500u128 * WAD_U128,
        500u128 * WAD_U128,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &add_call);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_symbol = Symbol::new(&ctx.env, SOROSWAP_XLM_USDC);
    let lp_before_swap = tracking_client.balance(&smart_account, &lp_symbol);

    // Execute a swap
    let swap_call = build_soroswap_swap_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        100u128 * WAD_U128,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &swap_call);

    let lp_after_swap = tracking_client.balance(&smart_account, &lp_symbol);
    assert_eq!(
        lp_after_swap, lp_before_swap,
        "Swap must not change LP tracking token balance"
    );
}

#[test]
fn test_soroswap_full_flow_add_swap_remove() {
    let ctx = setup_soroswap();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = am_client.create_account(&ctx.user);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_symbol = Symbol::new(&ctx.env, SOROSWAP_XLM_USDC);

    // Step 1: Add liquidity with 3000 XLM + 3000 USDC
    let add_call = build_soroswap_add_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        3000u128 * WAD_U128,
        3000u128 * WAD_U128,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &add_call);
    let lp_after_add = tracking_client.balance(&smart_account, &lp_symbol);
    assert!(lp_after_add > 0, "LP tokens must be minted after adding liquidity");

    // Step 2: Swap — LP balance must remain unchanged
    let swap_call = build_soroswap_swap_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        500u128 * WAD_U128,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &swap_call);
    assert_eq!(
        tracking_client.balance(&smart_account, &lp_symbol),
        lp_after_add,
        "Swap must not affect LP balance"
    );

    // Step 3: Remove all LP — balance must drop to zero
    let remove_call = build_soroswap_remove_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        lp_after_add as u128,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &remove_call);
    assert_eq!(
        tracking_client.balance(&smart_account, &lp_symbol),
        0,
        "All LP tokens must be burned after full removal"
    );
}

#[test]
fn test_soroswap_lp_symbol_added_to_smart_account_collateral() {
    let ctx = setup_soroswap();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = am_client.create_account(&ctx.user);

    let add_call = build_soroswap_add_liquidity_call(
        &ctx.env,
        ctx.soroswap_router.clone(),
        XLM_SYMBOL,
        USDC_SYMBOL,
        1000u128 * WAD_U128,
        1000u128 * WAD_U128,
        smart_account.clone(),
    );
    am_client.execute(&smart_account, &add_call);

    let sa_client =
        account_manager_contract::account_manager::smart_account_contract::Client::new(
            &ctx.env,
            &smart_account,
        );
    let lp_symbol = Symbol::new(&ctx.env, SOROSWAP_XLM_USDC);
    assert!(
        sa_client.get_all_collateral_tokens().contains(lp_symbol),
        "Soroswap LP tracking symbol must be added to smart account collateral"
    );
}

#[test]
fn test_soroswap_add_liquidity_multiple_times_accumulates_lp() {
    let ctx = setup_soroswap();
    let am_client = AccountManagerContractClient::new(&ctx.env, &ctx.account_manager);
    let smart_account = am_client.create_account(&ctx.user);

    let tracking_client = TrackingTokenClient::new(&ctx.env, &ctx.tracking_token);
    let lp_symbol = Symbol::new(&ctx.env, SOROSWAP_XLM_USDC);

    // First deposit
    am_client.execute(
        &smart_account,
        &build_soroswap_add_liquidity_call(
            &ctx.env,
            ctx.soroswap_router.clone(),
            XLM_SYMBOL,
            USDC_SYMBOL,
            1000u128 * WAD_U128,
            1000u128 * WAD_U128,
            smart_account.clone(),
        ),
    );
    let lp_after_first = tracking_client.balance(&smart_account, &lp_symbol);

    // Second deposit
    am_client.execute(
        &smart_account,
        &build_soroswap_add_liquidity_call(
            &ctx.env,
            ctx.soroswap_router.clone(),
            XLM_SYMBOL,
            USDC_SYMBOL,
            500u128 * WAD_U128,
            500u128 * WAD_U128,
            smart_account.clone(),
        ),
    );
    let lp_after_second = tracking_client.balance(&smart_account, &lp_symbol);

    assert!(
        lp_after_second > lp_after_first,
        "LP tracking balance must increase after second deposit"
    );
}
