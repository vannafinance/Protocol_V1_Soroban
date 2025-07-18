use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Env, Symbol, Vec, U256,
};

use crate::types::{DataKey, MarginAccountDataKey};

#[contract]
pub struct AccountLogicContract;

#[contractimpl]
impl AccountLogicContract {
    pub fn initialise_account(env: Env, user_address: Address) {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        admin.require_auth();

        //Set account creation time
        env.storage().persistent().set(
            &MarginAccountDataKey::AccountCreatedTime(user_address.clone()),
            &env.ledger().timestamp(),
        );

        // Push users address to list of Margin account user addresses
        let mut user_addresses: Vec<Address> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserAddresses)
            .expect("Account contract not initiated");
        user_addresses.push_back(user_address);

        env.storage()
            .persistent()
            .set(&MarginAccountDataKey::UserAddresses, &user_addresses);
    }
}
