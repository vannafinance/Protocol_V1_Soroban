#![cfg(test)]

use std::u128;

use lending_protocol_xlm::liquidity_pool_xlm::LiquidityPoolXLM;
use registry_contract::registry::{RegistryContract, RegistryContractClient};
use risk_engine_contract::risk_engine::RiskEngineContract;
use smart_account_contract::smart_account::{SmartAccountContract, SmartAccountContractClient};
use soroban_sdk::{
    Address, Env, IntoVal, Symbol, U256,
    testutils::{Address as _, Events, MockAuth, MockAuthInvoke},
    token::{self, StellarAssetClient},
};

const SMART_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

// -----------------------------------------------------------------------------
// Test Utilities
// -----------------------------------------------------------------------------
pub struct ContractAddresses {
    admin: Address,
    liquidity_pool_xlm: Address,
    liquidity_pool_usdc: Address,
    liquidity_pool_eurc: Address,
    registry_contract: Address,
    rate_model_contract: Address,
    account_manager_contract: Address,
    oracle_contract: Address,
    risk_engine_contract: Address,
    smart_account_contract: Option<Address>,
    vxlm_token_contract: Address,
    xlm_address: Address,
    usdc_address: Address,
    eurc_address: Address,
    mock_oracle_address: Address,
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
    let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());
    let eurc_token = env.register_stellar_asset_contract_v2(admin.clone());

    let mut contracts = ContractAddresses {
        admin: admin.clone(),
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
    // set oracle contract to something simple (we can reuse a mock)
    // let price_feed_addr = oracle_price_feed_setup(&env, &mut contracts);
    // // register oracle contract that points to price feed
    // env.register_at(
    //     &contracts.oracle_contract,
    //     OracleContract,
    //     (contracts.admin.clone(), price_feed_addr),
    // );

    // register smart account and set smart account hash in registry
    let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
    // let smart_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

    registry_client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &registry_client.address,
                fn_name: "set_native_xlm_contract_address",
                args: (&contracts.xlm_address,).into_val(env),
                sub_invokes: &[],
            },
        }])
        .set_native_xlm_contract_address(&contracts.xlm_address);

    registry_client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &registry_client.address,
                fn_name: "set_native_usdc_contract_address",
                args: (&contracts.usdc_address,).into_val(env),
                sub_invokes: &[],
            },
        }])
        .set_native_usdc_contract_address(&contracts.usdc_address);

    registry_client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &registry_client.address,
                fn_name: "set_native_eurc_contract_address",
                args: (&contracts.eurc_address,).into_val(env),
                sub_invokes: &[],
            },
        }])
        .set_native_eurc_contract_address(&contracts.eurc_address);

    registry_client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &registry_client.address,
                fn_name: "set_lendingpool_xlm",
                args: (&contracts.liquidity_pool_xlm,).into_val(env),
                sub_invokes: &[],
            },
        }])
        .set_lendingpool_xlm(&contracts.liquidity_pool_xlm);

    // registry_client.set_smart_account_hash(&smart_hash);
    // registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
    // registry_client.set_native_eurc_contract_address(&contracts.eurc_address);
    // registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
    // registry_client.set_oracle_contract_address(&contracts.oracle_contract);
    // registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
    // registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
    // registry_client.set_rate_model_address(&contracts.rate_model_contract);

    contracts
}

fn new_smart_account(
    env: &Env,
    account_manager: &Address,
    registry: &Address,
    user: &Address,
) -> SmartAccountContractClient<'static> {
    let id = env.register(SmartAccountContract, (account_manager, registry, user));
    let client = SmartAccountContractClient::new(env, &id);
    client
}

fn as_auth<T>(env: &Env, who: &Address, f: impl FnOnce() -> T) -> T {
    env.as_contract(who, f)
}

// fn expect_panic<F: FnOnce() -> R + std::panic::UnwindSafe, R>(f: F) {
//     assert!(std::panic::catch_unwind(f).is_err());
// }

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn activation_and_auth_flow_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let user = Address::generate(&env);
    let sa = new_smart_account(
        &env,
        &cc.account_manager_contract,
        &cc.registry_contract,
        &user,
    );

    assert!(!sa.is_account_active());
    sa.activate_account();

    as_auth(&env, &cc.account_manager_contract, || sa.activate_account());
    assert!(sa.is_account_active());

    as_auth(&env, &cc.account_manager_contract, || {
        sa.deactivate_account()
    });
    assert!(!sa.is_account_active());

    let events = env.events().all();
    assert!(
        events
            .iter()
            .any(|e| format!("{:?}", e).contains("Smart_Account_Activated"))
    );
    assert!(
        events
            .iter()
            .any(|e| format!("{:?}", e).contains("Smart_Account_Deactivated"))
    );
}

#[test]
fn activation_and_auth_flow_success() {
    let env = Env::default();
    // let (pool_xlm, _, _, _, _, _, registry) = setup(&env);
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    assert!(!sa.is_account_active());
    as_auth(&env, &manager, || sa.activate_account());
    assert!(sa.is_account_active());

    as_auth(&env, &manager, || {
        sa.deactivate_account();
    });
    assert!(!sa.is_account_active());

    // Todo check events emission
    // as_auth(&env, &sa.address, || {
    //     let events = env.events().all();
    //     println!("Events are : {:?}", events);
    //     assert!(
    //         events
    //             .iter()
    //             .any(|e| format!("{:?}", e).contains("Smart_Account_Activated"))
    //     );
    //     assert!(
    //         events
    //             .iter()
    //             .any(|e| format!("{:?}", e).contains("Smart_Account_Deactivated"))
    //     );
    // });
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn add_collateral_token_requires_manager_auth_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    sa.add_collateral_token(&Symbol::new(&env, "XLM"));
}

#[test]
fn add_collateral_token_requires_manager_auth_success() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    as_auth(&env, &manager, || {
        sa.add_collateral_token(&Symbol::new(&env, "XLM"));
    });

    let toks = sa.get_all_collateral_tokens();
    assert!(toks.contains(&Symbol::new(&env, "XLM")));
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn collateral_balance_transfer_and_cleanup_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    as_auth(&env, &manager, || {
        sa.add_collateral_token(&Symbol::new(&env, "XLM"));
    });
    sa.set_collateral_token_balance(&Symbol::new(&env, "XLM"), &U256::from_u128(&env, 5000));
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn remove_collateral_balance_and_cleanup_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    as_auth(&env, &manager, || {
        sa.add_collateral_token(&Symbol::new(&env, "XLM"));
        sa.set_collateral_token_balance(&Symbol::new(&env, "XLM"), &U256::from_u128(&env, 5000));
    });

    sa.remove_collateral_token_balance(&user.clone(), &Symbol::new(&env, "XLM"), &1000);
}

#[test]
fn collateral_balance_transfer_success() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    as_auth(&env, &manager, || {
        sa.add_collateral_token(&Symbol::new(&env, "XLM"));
        sa.set_collateral_token_balance(&Symbol::new(&env, "XLM"), &U256::from_u128(&env, 5000));
    });

    // Allowing mock auth for minting and then removing it
    env.mock_all_auths();
    let stellar_asset_xlm = StellarAssetClient::new(&env, &cc.xlm_address);
    stellar_asset_xlm.mint(&sa.address, &5000);
    env.set_auths(&[]);

    as_auth(&env, &manager, || {
        sa.remove_collateral_token_balance(&user.clone(), &Symbol::new(&env, "XLM"), &1000);
    });

    let bal = sa.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
    let xlm = token::Client::new(&env, &cc.xlm_address);
    assert_eq!(xlm.balance(&sa.address), 4000);
    assert_eq!(bal, U256::from_u128(&env, 4000));
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn set_debt_flag_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    sa.set_has_debt(&true, &Symbol::new(&env, "XLM"));
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn add_borrowed_token_auth_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    // sa.set_has_debt(&true, &Symbol::new(&env, "XLM"));
    as_auth(&env, &cc.liquidity_pool_xlm, || {
        sa.set_has_debt(&true, &Symbol::new(&env, "XLM"))
    });
    assert!(sa.has_debt());

    sa.add_borrowed_token(&Symbol::new(&env, "XLM"));
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn remove_borrowed_token_auth_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    // sa.set_has_debt(&true, &Symbol::new(&env, "XLM"));
    as_auth(&env, &cc.liquidity_pool_xlm, || {
        sa.set_has_debt(&true, &Symbol::new(&env, "XLM"))
    });
    assert!(sa.has_debt());

    as_auth(&env, &cc.liquidity_pool_xlm, || {
        sa.add_borrowed_token(&Symbol::new(&env, "XLM"));
    });
    let list = sa.get_all_borrowed_tokens();
    assert!(list.contains(&Symbol::new(&env, "XLM")));

    sa.remove_borrowed_token(&Symbol::new(&env, "XLM"));
}

#[test]
fn borrowed_token_auth_and_debt_flag_success() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    as_auth(&env, &cc.liquidity_pool_xlm, || {
        sa.set_has_debt(&true, &Symbol::new(&env, "XLM"))
    });
    assert!(sa.has_debt());

    as_auth(&env, &cc.liquidity_pool_xlm, || {
        sa.add_borrowed_token(&Symbol::new(&env, "XLM"))
    });
    let list = sa.get_all_borrowed_tokens();
    assert!(list.contains(&Symbol::new(&env, "XLM")));

    as_auth(&env, &cc.liquidity_pool_xlm, || {
        sa.remove_borrowed_token(&Symbol::new(&env, "XLM"))
    });
    assert!(!sa.has_debt());
}

#[test]
#[should_panic(expected = "Non existent lending pool, Auth failed!!")]
fn security_check_unsupported_symbol_failure() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    sa.set_has_debt(&true, &Symbol::new(&env, "FAKE"));
}

/// ✅ Test 1: Happy path — sweeps all collateral tokens to a given address.
#[test]
fn sweep_to_transfers_all_collateral_balances() {
    let env = Env::default();
    let cc = test_initiation(&env);

    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);
    let sa_addr = sa.address.clone();

    // Allowing mock auth for minting and then removing it
    env.mock_all_auths();
    let stellar_asset_xlm = StellarAssetClient::new(&env, &cc.xlm_address);
    let stellar_asset_usdc = StellarAssetClient::new(&env, &cc.usdc_address);
    let stellar_asset_eurc = StellarAssetClient::new(&env, &cc.eurc_address);
    stellar_asset_xlm.mint(&sa_addr, &1000);
    stellar_asset_usdc.mint(&sa_addr, &2000);
    stellar_asset_eurc.mint(&sa_addr, &3000);
    env.set_auths(&[]);

    // Set collateral tokens and balances (authorized)
    as_auth(&env, &manager, || {
        sa.add_collateral_token(&Symbol::new(&env, "XLM"));
        sa.add_collateral_token(&Symbol::new(&env, "USDC"));
        sa.add_collateral_token(&Symbol::new(&env, "EURC"));

        sa.set_collateral_token_balance(&Symbol::new(&env, "XLM"), &U256::from_u128(&env, 1000));
        sa.set_collateral_token_balance(&Symbol::new(&env, "USDC"), &U256::from_u128(&env, 2000));
        sa.set_collateral_token_balance(&Symbol::new(&env, "EURC"), &U256::from_u128(&env, 3000));
    });

    // Perform sweep
    as_auth(&env, &manager, || {
        sa.sweep_to(&recipient.clone());
    });

    let xlm = token::Client::new(&env, &cc.xlm_address);
    let usdc = token::Client::new(&env, &cc.usdc_address);

    let eurc = token::Client::new(&env, &cc.eurc_address);

    // Recipient should receive all token balances
    assert_eq!(xlm.balance(&recipient), 1000);
    assert_eq!(usdc.balance(&recipient), 2000);
    assert_eq!(eurc.balance(&recipient), 3000);

    // Smart account balances should now be 0
    assert_eq!(xlm.balance(&sa_addr), 0);
    assert_eq!(usdc.balance(&sa_addr), 0);
    assert_eq!(eurc.balance(&sa_addr), 0);

    // Collateral balances in storage should be cleared to 0
    assert_eq!(
        sa.get_collateral_token_balance(&Symbol::new(&env, "XLM")),
        U256::from_u128(&env, 0)
    );
}

/// 🚫 Test 2: Unauthorized caller — only account manager can call sweep_to.
#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn sweep_to_unauthorized_fails() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    sa.sweep_to(&recipient.clone());
}

/// 🧩 Test 3: sweep_to with no collateral tokens — should panic.
#[test]
// #[should_panic(expected = "No collateral tokens exist for this smart account")]
fn sweep_to_with_empty_collateral_list_is_safe() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    // No collateral tokens set
    as_auth(&env, &manager, || {
        sa.sweep_to(&recipient.clone());
    });
    // Nothing to assert; No collateral tokens to sweep to
}

/// 💥 Test 4: Conversion error — if collateral balance U256→u128 conversion fails
#[test]
#[should_panic(expected = "\"failing with contract error\", 4")]
fn sweep_to_integer_conversion_error_panics() {
    let env = Env::default();
    let cc = test_initiation(&env);
    let manager = cc.account_manager_contract.clone();
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);
    let sa = new_smart_account(&env, &manager, &cc.registry_contract, &user);

    env.mock_all_auths();
    let stellar_asset_xlm = StellarAssetClient::new(&env, &cc.xlm_address);
    stellar_asset_xlm.mint(&sa.address, &1000);
    env.set_auths(&[]);

    // Set an unreasonably large U256 value to simulate overflow
    as_auth(&env, &manager, || {
        sa.add_collateral_token(&Symbol::new(&env, "XLM"));
        let huge = U256::from_u128(&env, u128::MAX);
        sa.set_collateral_token_balance(
            &Symbol::new(&env, "XLM"),
            &huge.add(&U256::from_u128(&env, 10)),
        );
    });

    // When calling sweep_to, should panic with IntegerConversionError
    as_auth(&env, &manager, || {
        sa.sweep_to(&recipient.clone());
    });
}
