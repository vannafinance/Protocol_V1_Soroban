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
