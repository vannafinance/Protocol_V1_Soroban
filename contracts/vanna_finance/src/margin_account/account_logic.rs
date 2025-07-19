use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, Symbol, Vec};

use crate::{
    errors::MarginAccountError,
    types::{DataKey, MarginAccountDataKey},
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const TLL_LEDGERS_MONTH: u32 = 518400;

#[contract]
pub struct AccountLogicContract;

#[contractimpl]
impl AccountLogicContract {
    pub fn initialise_account_contract(env: Env, admin: Address) {
        env.storage().persistent().set(&DataKey::Admin, &admin);
        Self::extend_ttl(&env, DataKey::Admin);
    }

    pub fn initialise_account(env: Env, user_address: Address) {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        admin.require_auth();

        //Set account creation time
        let key_a = MarginAccountDataKey::AccountCreatedTime(user_address.clone());
        env.storage()
            .persistent()
            .set(&key_a, &env.ledger().timestamp());
        Self::extend_ttl_margin_account(&env, key_a);

        // Push users address to list of Margin account user addresses
        let mut user_addresses: Vec<Address> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserAddresses)
            .expect("Account contract not initiated");
        user_addresses.push_back(user_address.clone());

        env.storage()
            .persistent()
            .set(&MarginAccountDataKey::UserAddresses, &user_addresses);
        Self::extend_ttl_margin_account(&env, MarginAccountDataKey::UserAddresses);

        let key_b = MarginAccountDataKey::IsAccountInitialised(user_address.clone());
        env.storage().persistent().set(&key_b, &true);
        Self::extend_ttl_margin_account(&env, key_b);

        let key_c = MarginAccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key_c, &true);
        Self::extend_ttl_margin_account(&env, key_c);

        let key_d = MarginAccountDataKey::HasDebt(user_address.clone());
        env.storage().persistent().set(&key_d, &false);
        Self::extend_ttl_margin_account(&env, key_d);
    }

    pub fn deactivate_account(env: Env, user_address: Address) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let key = MarginAccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_margin_account(&env, key);
        Ok(())
    }

    pub fn activate_account(env: Env, user_address: Address) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let key = MarginAccountDataKey::IsAccountActive(user_address.clone());

        env.storage().persistent().set(&key, &true);
        Self::extend_ttl_margin_account(&env, key);

        Ok(())
    }

    pub fn add_collateral_token(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        collateral_tokens_list.push_back(token_symbol);

        let key_c = MarginAccountDataKey::UserCollateralTokensList(user_address.clone());
        env.storage()
            .persistent()
            .set(&key_c, &collateral_tokens_list);
        Self::extend_ttl_margin_account(&env, key_c);
        Ok(())
    }

    pub fn remove_collateral_token(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        let index = collateral_tokens_list
            .first_index_of(token_symbol)
            .unwrap_or_else(|| panic!("Collateral token doesn't exist in the list"));

        collateral_tokens_list.remove(index);
        Ok(())
    }

    pub fn get_all_collateral_tokens(
        env: Env,
        user_address: Address,
    ) -> Result<Vec<Symbol>, MarginAccountError> {
        user_address.require_auth();
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        Ok(collateral_tokens_list)
    }

    pub fn add_borrowed_token(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        borrowed_tokens_list.push_back(token_symbol);

        let key_a = MarginAccountDataKey::UserBorrowedTokensList(user_address.clone());
        env.storage()
            .persistent()
            .set(&key_a, &borrowed_tokens_list);
        Self::extend_ttl_margin_account(&env, key_a);
        Ok(())
    }

    pub fn remove_borrowed_token(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        let index = borrowed_tokens_list
            .first_index_of(token_symbol)
            .unwrap_or_else(|| panic!("Borrowed token doesn't exist in the list"));

        borrowed_tokens_list.remove(index);
        Ok(())
    }

    pub fn get_all_borrowed_tokens(
        env: Env,
        user_address: Address,
    ) -> Result<Vec<Symbol>, MarginAccountError> {
        user_address.require_auth();
        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        Ok(borrowed_tokens_list)
    }

    pub fn has_debt(env: Env, user_address: Address) -> bool {
        let has_debt = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::HasDebt(user_address))
            .unwrap();
        has_debt
    }

    pub fn set_has_debt(
        env: Env,
        user_address: Address,
        has_debt: bool,
    ) -> Result<(), MarginAccountError> {
        env.storage().persistent().set(
            &MarginAccountDataKey::HasDebt(user_address.clone()),
            &has_debt,
        );
        Self::extend_ttl_margin_account(&env, MarginAccountDataKey::HasDebt(user_address));
        Ok(())
    }

    fn extend_ttl_margin_account(env: &Env, key: MarginAccountDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn extend_ttl(env: &Env, key: DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
