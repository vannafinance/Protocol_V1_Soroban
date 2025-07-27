use soroban_sdk::{contract, contractimpl, log, Address, Env, Symbol, Vec, U256};

use crate::{
    errors::MarginAccountError,
    events::{AccountCreationEvent, AccountDeactivationEvent, AccountDeletionEvent},
    types::{AccountManagerKey, DataKey, LendingProtocols, MarginAccountDataKey},
};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
// const TLL_LEDGERS_MONTH: u32 = 518400;

pub mod lending_protocol_xlm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_xlm.wasm"
    );
}

pub mod lending_protocol_usdc {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_usdc.wasm"
    );
}

pub mod lending_protocol_eurc {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_eurc.wasm"
    );
}

#[contract]
pub struct AccountLogicContract;

#[contractimpl]
impl AccountLogicContract {
    pub fn init_accountmanager_contract(env: Env, admin: Address, asset_cap: U256) {
        if env.storage().persistent().has(&AccountManagerKey::Admin) {
            panic!("Admin already set for account manager");
        }
        env.storage()
            .persistent()
            .set(&AccountManagerKey::Admin, &admin);
        Self::extend_account_ttl(&env, AccountManagerKey::Admin);
        let user_addresses: Vec<Address> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&MarginAccountDataKey::UserAddresses, &user_addresses);
        Self::extend_ttl_margin_account(&env, MarginAccountDataKey::UserAddresses);

        Self::set_max_asset_cap(&env, asset_cap);
    }

    pub fn init_lending_protocols(
        env: Env,
        lending_protocol_xlm: Address,
        lending_protocol_usdc: Address,
        lending_protocol_eurc: Address,
    ) {
        let admin = Self::get_account_contract_admin(&env);
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&LendingProtocols::XlmAddress, &lending_protocol_xlm);
        env.storage()
            .persistent()
            .set(&LendingProtocols::UsdcAddress, &lending_protocol_usdc);

        env.storage()
            .persistent()
            .set(&LendingProtocols::EurcAddress, &lending_protocol_eurc);
        Self::extend_lendingttl(&env, LendingProtocols::XlmAddress);
        Self::extend_lendingttl(&env, LendingProtocols::UsdcAddress);
        Self::extend_lendingttl(&env, LendingProtocols::EurcAddress);
    }

    pub fn initialise_account(env: Env, user_address: Address) {
        let admin: Address = Self::get_account_contract_admin(&env);
        admin.require_auth();

        //Set account creation time
        let key_a = MarginAccountDataKey::AccountCreatedTime(user_address.clone());
        env.storage()
            .persistent()
            .set(&key_a, &env.ledger().timestamp());
        Self::extend_ttl_margin_account(&env, key_a);

        // Push users address to list of Margin account user addresses
        let key_h = MarginAccountDataKey::UserAddresses;
        let mut user_addresses: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key_h)
            .expect("Account contract not initiated");
        user_addresses.push_back(user_address.clone());

        env.storage().persistent().set(&key_h, &user_addresses);
        Self::extend_ttl_margin_account(&env, key_h);

        let key_b = MarginAccountDataKey::IsAccountInitialised(user_address.clone());
        env.storage().persistent().set(&key_b, &true);
        Self::extend_ttl_margin_account(&env, key_b);

        let key_c = MarginAccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key_c, &true);
        Self::extend_ttl_margin_account(&env, key_c);

        let key_d = MarginAccountDataKey::HasDebt(user_address.clone());
        env.storage().persistent().set(&key_d, &false);
        Self::extend_ttl_margin_account(&env, key_d);
        log!(&env, "Reach aaaauuuuuxd");

        env.events().publish(
            (Symbol::new(&env, "Account_Creation"), user_address.clone()),
            AccountCreationEvent {
                margin_account: user_address,
                creation_time: env.ledger().timestamp(),
            },
        );
        log!(&env, "Reach qqqqq");
    }

    pub fn deactivate_account(env: Env, user_address: Address) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let key = MarginAccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_margin_account(&env, key);
        env.events().publish(
            (
                Symbol::new(&env, "Account_Deactivated"),
                user_address.clone(),
            ),
            AccountDeactivationEvent {
                margin_account: user_address,
                deactivate_time: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn activate_account(env: Env, user_address: Address) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let key = MarginAccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key, &true);
        Self::extend_ttl_margin_account(&env, key);

        Ok(())
    }

    pub fn add_collateral_token_balance(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();

        if !Self::get_iscollateral_allowed(&env, token_symbol.clone()) {
            panic!("This token is not allowed as collateral");
        }

        let key_c = MarginAccountDataKey::UserCollateralTokensList(user_address.clone());
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_c)
            .unwrap_or_else(|| Vec::new(&env));

        if U256::from_u32(&env, collateral_tokens_list.len()) >= Self::get_max_asset_cap(&env) {
            panic!("Max asset cap crossed!");
        };

        if !collateral_tokens_list.contains(token_symbol.clone()) {
            collateral_tokens_list.push_back(token_symbol.clone());
        }

        env.storage()
            .persistent()
            .set(&key_c, &collateral_tokens_list);
        Self::extend_ttl_margin_account(&env, key_c);

        let key_a =
            MarginAccountDataKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let new_balance = token_balance.add(&token_amount);
        env.storage().persistent().set(&key_a, &new_balance);
        Self::extend_ttl_margin_account(&env, key_a);

        Ok(())
    }

    pub fn remove_collateral_token_balance(
        env: Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();

        let key_a = MarginAccountDataKey::UserCollateralTokensList(user_address.clone());
        let mut collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| Vec::new(&env));
        let index = collateral_tokens_list
            .first_index_of(token_symbol.clone())
            .unwrap_or_else(|| panic!("Collateral token doesn't exist in the list"));

        let key_b =
            MarginAccountDataKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        if token_amount > token_balance {
            panic!("Insufficient Collateral balance for user in this token to deduct",);
        }
        let new_balance = token_balance.sub(&token_amount);
        env.storage().persistent().set(&key_b, &new_balance);
        Self::extend_ttl_margin_account(&env, key_b);

        if token_amount == token_balance {
            collateral_tokens_list.remove(index);
            env.storage()
                .persistent()
                .set(&key_a, &collateral_tokens_list);

            Self::extend_ttl_margin_account(&env, key_a);
        }

        Ok(())
    }

    pub fn get_collateral_token_balance(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
    ) -> Result<U256, MarginAccountError> {
        let key_a =
            MarginAccountDataKey::UserCollateralBalance(user_address.clone(), token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        Ok(token_balance)
    }

    pub fn get_all_collateral_tokens(
        env: &Env,
        user_address: Address,
    ) -> Result<Vec<Symbol>, MarginAccountError> {
        user_address.require_auth();
        let collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        Ok(collateral_tokens_list)
    }

    pub fn add_borrowed_token_balance(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let key_a = MarginAccountDataKey::UserBorrowedTokensList(user_address.clone());

        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| Vec::new(&env));
        if !borrowed_tokens_list.contains(&token_symbol.clone()) {
            borrowed_tokens_list.push_back(token_symbol.clone());
        }

        env.storage()
            .persistent()
            .set(&key_a, &borrowed_tokens_list);
        Self::extend_ttl_margin_account(&env, key_a);

        let key_b =
            MarginAccountDataKey::UserBorrowedDebt(user_address.clone(), token_symbol.clone());
        let token_debt = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        let new_debt = token_debt.add(&token_amount);
        env.storage().persistent().set(&key_b, &new_debt);
        Self::extend_ttl_margin_account(&env, key_b);

        let total_debt = Self::get_total_debt_in_pool(&env, token_symbol.clone());
        let res = total_debt.add(&token_amount);
        env.storage().persistent().set(
            &MarginAccountDataKey::TotalDebtInPool(token_symbol.clone()),
            &res,
        );
        Self::extend_ttl_margin_account(
            &env,
            MarginAccountDataKey::TotalDebtInPool(token_symbol.clone()),
        );

        Self::set_has_debt(&env, user_address, true).unwrap();

        Ok(())
    }

    pub fn remove_borrowed_token_balance(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
        token_amount: U256,
    ) -> Result<(), MarginAccountError> {
        user_address.require_auth();
        let key_a = MarginAccountDataKey::UserBorrowedTokensList(user_address.clone());
        let mut borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| Vec::new(&env));
        let index = borrowed_tokens_list
            .first_index_of(token_symbol.clone())
            .unwrap_or_else(|| panic!("Borrowed token doesn't exist in the list"));

        let key_b =
            MarginAccountDataKey::UserBorrowedDebt(user_address.clone(), token_symbol.clone());
        let token_debt = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        if token_amount > token_debt {
            panic!("Cannot remove debt more than what it exists for this token",);
        } else if token_amount == token_debt {
            env.storage()
                .persistent()
                .set(&key_b, &U256::from_u128(&env, 0));
            Self::extend_ttl_margin_account(&env, key_b);

            borrowed_tokens_list.remove(index).unwrap();
            env.storage()
                .persistent()
                .set(&key_a, &borrowed_tokens_list);
            Self::extend_ttl_margin_account(&env, key_a);

            // Set total_debt for pool
            let total_debt = Self::get_total_debt_in_pool(&env, token_symbol.clone());
            log!(&env, "Reached x");

            let res = total_debt.sub(&token_amount);
            log!(&env, "Reached y");

            env.storage().persistent().set(
                &MarginAccountDataKey::TotalDebtInPool(token_symbol.clone()),
                &res,
            );

            if borrowed_tokens_list.len() == 0 {
                Self::set_has_debt(&env, user_address, false).unwrap();
            }
        } else {
            let new_debt = token_debt.sub(&token_amount);
            env.storage().persistent().set(&key_b, &new_debt);
            Self::extend_ttl_margin_account(&env, key_b);

            // Set total_debt for pool
            let total_debt = Self::get_total_debt_in_pool(&env, token_symbol.clone());
            let res = total_debt.sub(&token_amount);
            env.storage().persistent().set(
                &MarginAccountDataKey::TotalDebtInPool(token_symbol.clone()),
                &res,
            );
        }

        Self::extend_ttl_margin_account(
            &env,
            MarginAccountDataKey::TotalDebtInPool(token_symbol.clone()),
        );

        Ok(())
    }

    pub fn get_borrowed_token_debt(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
    ) -> Result<U256, MarginAccountError> {
        let key_b =
            MarginAccountDataKey::UserBorrowedDebt(user_address.clone(), token_symbol.clone());
        let token_debt = env
            .storage()
            .persistent()
            .get(&key_b)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        Ok(token_debt)
    }

    pub fn get_all_borrowed_tokens(
        env: &Env,
        user_address: Address,
    ) -> Result<Vec<Symbol>, MarginAccountError> {
        user_address.require_auth();
        let borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        Ok(borrowed_tokens_list)
    }

    pub fn has_debt(env: &Env, user_address: Address) -> bool {
        let has_debt = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::HasDebt(user_address))
            .unwrap_or_else(|| false);
        has_debt
    }

    pub fn set_has_debt(
        env: &Env,
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

    pub fn get_total_debt_in_pool(env: &Env, token_symbol: Symbol) -> U256 {
        let key_x = MarginAccountDataKey::TotalDebtInPool(token_symbol.clone());

        let res = env
            .storage()
            .persistent()
            .get(&key_x)
            .unwrap_or_else(|| U256::from_u128(&env, 0));

        res
    }

    // !!! todo CCI
    // pub fn get_total_liquidity_in_pool(env: &Env, token_symbol: Symbol) -> U256 {
    //     env.storage()
    //         .persistent()
    //         .get(&PoolDataKey::Pool(token_symbol))
    //         .unwrap_or(U256::from_u128(&env, 0))
    // }

    pub fn delete_account(env: &Env, user_address: Address) -> Result<(), MarginAccountError> {
        user_address.require_auth();

        if Self::has_debt(env, user_address.clone()) {
            panic!("Cannot delete account with debt, please repay debt first");
        }

        // Set account deletion time
        env.storage().persistent().set(
            &MarginAccountDataKey::AccountDeletedTime(user_address.clone()),
            &env.ledger().timestamp(),
        );

        // remove user's address from list of Margin account user addresses
        let key_d = MarginAccountDataKey::UserAddresses;
        let mut user_addresses: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key_d)
            .expect("Account contract not initiated");
        let index = user_addresses
            .first_index_of(user_address.clone())
            .unwrap_or_else(|| panic!("User account not found in list"));
        user_addresses.remove(index);
        env.storage().persistent().set(&key_d, &user_addresses);
        Self::extend_ttl_margin_account(&env, key_d);

        let borrowed_tokens_symbols: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        // Remove balance for each borrowed token
        for symbol in borrowed_tokens_symbols {
            env.storage()
                .persistent()
                .remove(&MarginAccountDataKey::UserBorrowedDebt(
                    user_address.clone(),
                    symbol,
                ));
        }
        // Then remove all borrowed tokens from the list
        env.storage()
            .persistent()
            .remove(&MarginAccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ));

        let collateral_tokens: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&MarginAccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));

        // Remove balance for each collateral token
        for symbolx in collateral_tokens {
            env.storage()
                .persistent()
                .remove(&MarginAccountDataKey::UserCollateralBalance(
                    user_address.clone(),
                    symbolx,
                ));
        }

        // Then remove all collateral tokens from the list
        env.storage()
            .persistent()
            .remove(&MarginAccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ));

        let key_c = MarginAccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key_c, &false);
        Self::extend_ttl_margin_account(&env, key_c);

        let key_d = MarginAccountDataKey::HasDebt(user_address.clone());
        env.storage().persistent().set(&key_d, &false);
        Self::extend_ttl_margin_account(&env, key_d);

        env.events().publish(
            (Symbol::new(&env, "Account_Deleted"), user_address.clone()),
            AccountDeletionEvent {
                margin_account: user_address,
                deletion_time: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn set_iscollateral_allowed(env: &Env, token_symbol: Symbol, allowed: bool) {
        let admin: Address = Self::get_account_contract_admin(env);
        admin.require_auth();
        let key = MarginAccountDataKey::IsCollateralAllowed(token_symbol);
        env.storage().persistent().set(&key, &allowed);
        Self::extend_ttl_margin_account(env, key);
    }

    pub fn get_iscollateral_allowed(env: &Env, token_symbol: Symbol) -> bool {
        let key = MarginAccountDataKey::IsCollateralAllowed(token_symbol);
        env.storage().persistent().get(&key).unwrap_or(false)
    }

    fn get_account_contract_admin(env: &Env) -> Address {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&AccountManagerKey::Admin)
            .unwrap_or_else(|| panic!("Admin key not set"));
        admin
    }

    pub fn set_max_asset_cap(env: &Env, cap: U256) {
        let key = MarginAccountDataKey::AssetCap;
        env.storage().persistent().set(&key, &cap);
        Self::extend_ttl_margin_account(env, key);
    }

    pub fn get_max_asset_cap(env: &Env) -> U256 {
        let key = MarginAccountDataKey::AssetCap;
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("Asset cap not set"))
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

    fn extend_account_ttl(env: &Env, key: AccountManagerKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn extend_lendingttl(env: &Env, key: LendingProtocols) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
