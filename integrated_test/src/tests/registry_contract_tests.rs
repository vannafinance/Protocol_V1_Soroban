#![cfg(test)]

use soroban_sdk::{
    Address, BytesN, Env, IntoVal, Symbol, Vec, testutils::{Address as _, BytesN as _, MockAuth, MockAuthInvoke}
};

use registry_contract::registry::{RegistryContract, RegistryContractClient};
use registry_contract::types::{RegistryContractError, RegistryKey};
use soroban_sdk::symbol_short;
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");

fn setup() -> (Env, Address, RegistryContractClient<'static>) {
    let env = Env::default();

    let admin = Address::generate(&env);
    let contract_id = env.register(RegistryContract, (admin.clone(),));
    let client = RegistryContractClient::new(&env, &contract_id);

    // initialize
    (env, admin, client)
}

#[test]
fn test_constructor_and_admin_storage() {
    let (env, admin, client) = setup();

    let stored_admin: Address = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn test_set_and_get_lendingpool_xlm_failure() {
    let (env, admin, client) = setup();
    let addr = Address::generate(&env);
    println!("Admin is {:?}", admin);

    // simulate unauthorized call
    let bad_actor = Address::generate(&env);
    println!("Bad_actor is {:?}", bad_actor);

    client
        .mock_auths(&[MockAuth {
            address: &bad_actor,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "hello",
                args: (&addr,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_lendingpool_xlm(&addr);
}

#[test]
fn test_set_and_get_lendingpool_xlm_success() {
    let (env, admin, client) = setup();
    let addr = Address::generate(&env);
    println!("Admin is {:?}", admin);

    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_lendingpool_xlm",
                args: (&addr,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_lendingpool_xlm(&addr);
    assert!(client.get_lendingpool_xlm().eq(&addr));
}

#[test]
fn test_set_and_get_smart_account_hash() {
    let (env, admin, client) = setup();
    let hash = BytesN::random(&env);

    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_smart_account_hash",
                args: (&hash,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_smart_account_hash(&hash);

    assert_eq!(client.get_smart_account_hash(), hash);
}

#[test]
fn test_all_setters_and_getters() {
    let (env, _, client) = setup();
    let addr1 = Address::generate(&env);
    let addr2 = Address::generate(&env);
    let addr3 = Address::generate(&env);

    env.mock_all_auths();

    client.set_accountmanager_contract(&addr1);
    assert_eq!(client.get_accountmanager_contract(), addr1);

    client.set_lendingpool_eurc(&addr2);
    assert_eq!(client.get_lendingpool_eurc(), addr2);

    client.set_lendingpool_usdc(&addr3);
    assert_eq!(client.get_lendingpool_usdc(), addr3);

    client.set_risk_engine_address(&addr1);
    assert_eq!(client.get_risk_engine_address(), addr1);

    client.set_rate_model_address(&addr2);
    assert_eq!(client.get_rate_model_address(), addr2);

    client.set_oracle_contract_address(&addr3);
    assert_eq!(client.get_oracle_contract_address(), addr3);

    client.set_native_xlm_contract_address(&addr1);
    assert_eq!(client.get_xlm_contract_adddress(), addr1);

    client.set_native_usdc_contract_address(&addr2);
    assert_eq!(client.get_usdc_contract_address(), addr2);

    client.set_native_eurc_contract_address(&addr3);
    assert_eq!(client.get_eurc_contract_address(), addr3);
}

#[test]
fn test_account_lifecycle_add_update_close_with_auth() {
    let (env, admin, client) = setup();
    let trader = Address::generate(&env);
    let smart_account = Address::generate(&env);
    let acc_manager = Address::generate(&env);

    // Set account manager first
    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_accountmanager_contract",
                args: (&acc_manager,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_accountmanager_contract(&acc_manager);

    // ✅ Authenticated call from AccountManager to add_account
    let res = client
        .mock_auths(&[MockAuth {
            address: &acc_manager,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "add_account",
                args: (&trader, &smart_account).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .add_account(&trader, &smart_account);

    assert!(res);

    // Verify owner set
    let owner_key = RegistryKey::OwnerAddress(smart_account.clone());
    env.as_contract(&client.address, || {
        let owner: Option<Address> = env.storage().persistent().get(&owner_key).unwrap();
        assert_eq!(owner, Some(trader.clone()));
    });

    // ✅ Update smart account owner  (account manager authenticated)
    let new_trader = Address::generate(&env);
    let res2 = client
        .mock_auths(&[MockAuth {
            address: &acc_manager,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "update_account",
                args: (&new_trader, &smart_account).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_account(&new_trader, &smart_account);
    assert!(res2);

    env.as_contract(&client.address, || {
        let updated: Option<Address> = env.storage().persistent().get(&owner_key).unwrap();
        assert_eq!(updated, Some(new_trader.clone()));
    });

    // ✅ Close account (account manager authenticated)
    let res3 = client
        .mock_auths(&[MockAuth {
            address: &acc_manager,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "close_account",
                args: (&new_trader, &smart_account).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .close_account(&new_trader, &smart_account);
    assert!(res3);

    env.as_contract(&client.address, || {
        let closed: Option<Address> = env.storage().persistent().get(&owner_key).unwrap();
        assert_eq!(closed, None);
    });
}

fn test_account_lifecycle_fails_without_acc_manager_auth()
-> (RegistryContractClient<'static>, Address, Address) {
    let (env, admin, client) = setup();
    let trader = Address::generate(&env);
    let smart_account = Address::generate(&env);
    let acc_manager = Address::generate(&env);

    // Set account manager first
    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_accountmanager_contract",
                args: (&acc_manager,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_accountmanager_contract(&acc_manager);
    (client, trader, smart_account)
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn add_account_without_auth() {
    let (client, trader, smart_account) = test_account_lifecycle_fails_without_acc_manager_auth();
    // ❌ Unauthorized add_account
    client.add_account(&trader, &smart_account);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn update_account_without_auth() {
    let (client, trader, smart_account) = test_account_lifecycle_fails_without_acc_manager_auth();
    // ❌ Unauthorized update_account
    client.update_account(&trader, &smart_account);
}

#[test]
#[should_panic(expected = "Unauthorized function call for address")]
fn close_account_without_auth() {
    let (client, trader, smart_account) = test_account_lifecycle_fails_without_acc_manager_auth();
    // ❌ Unauthorized close_account
    client.close_account(&trader, &smart_account);
}

#[test]
fn test_duplicate_add_account_does_not_duplicate_list() {
    let (env, admin, client) = setup();
    let trader = Address::generate(&env);
    let smart_account = Address::generate(&env);
    let acc_manager = Address::generate(&env);

    // Set account manager first
    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_accountmanager_contract",
                args: (&acc_manager,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_accountmanager_contract(&acc_manager);

    // ✅ Authenticated call from AccountManager to add_account
    let res = client
        .mock_auths(&[MockAuth {
            address: &acc_manager,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "add_account",
                args: (&trader, &smart_account).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .add_account(&trader, &smart_account);
    assert!(res);

    // ✅ Authenticated call from AccountManager to add_account duplicate
    let res2 = client
        .mock_auths(&[MockAuth {
            address: &acc_manager,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "add_account",
                args: (&trader, &smart_account).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .add_account(&trader, &smart_account);
    assert!(res2);

    env.as_contract(&client.address, || {
        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&RegistryKey::SmartAccountsList)
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap(), smart_account);
    });
}

#[test]
#[should_panic(expected = "Failed to get lendingpool_xlm address")]
fn test_get_unset_address_panics() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(RegistryContract, (admin,));
    let client = RegistryContractClient::new(&env, &contract_id);
    let _ = client.get_lendingpool_xlm();
}
