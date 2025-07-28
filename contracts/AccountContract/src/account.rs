use core::panic;

use soroban_sdk::{Address, Env, Symbol, Vec, contract};

use crate::types::{AccountDataKey, AccountDeactivationEvent, AccountError};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;

#[contract]
pub struct AccountContract;

impl AccountContract {
    pub fn deactivate_account(env: Env, user_address: Address) -> Result<(), AccountError> {
        user_address.require_auth();
        let key = AccountDataKey::IsAccountActive(user_address.clone());
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

    pub fn activate_account(env: Env, user_address: Address) -> Result<(), AccountError> {
        user_address.require_auth();
        let key = AccountDataKey::IsAccountActive(user_address.clone());
        env.storage().persistent().set(&key, &true);
        Self::extend_ttl_margin_account(&env, key);

        Ok(())
    }

    pub fn has_debt(env: &Env, user_address: Address) -> bool {
        let has_debt = env
            .storage()
            .persistent()
            .get(&AccountDataKey::HasDebt(user_address))
            .unwrap_or_else(|| false);
        has_debt
    }

    pub fn set_has_debt(
        env: &Env,
        user_address: Address,
        has_debt: bool,
    ) -> Result<(), AccountError> {
        env.storage()
            .persistent()
            .set(&AccountDataKey::HasDebt(user_address.clone()), &has_debt);
        Self::extend_ttl_margin_account(&env, AccountDataKey::HasDebt(user_address));
        Ok(())
    }

    pub fn get_all_borrowed_tokens(
        env: &Env,
        user_address: Address,
    ) -> Result<Vec<Symbol>, AccountError> {
        user_address.require_auth();
        let borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&AccountDataKey::UserBorrowedTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        Ok(borrowed_tokens_list)
    }

    pub fn get_all_collateral_tokens(
        env: &Env,
        user_address: Address,
    ) -> Result<Vec<Symbol>, AccountError> {
        user_address.require_auth();
        let collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&AccountDataKey::UserCollateralTokensList(
                user_address.clone(),
            ))
            .unwrap_or_else(|| Vec::new(&env));
        Ok(collateral_tokens_list)
    }

    fn extend_ttl_margin_account(env: &Env, key: AccountDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
