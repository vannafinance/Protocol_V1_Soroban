use soroban_sdk::{Address, Env, String, Symbol, contract, contractimpl};

use crate::types::{
    AllowanceDataKey, ApprovalEvent, BurnEvent, DataKey, MintEvent, TokenError, TokenInfo,
    TransferEvent,
};

fn check_nonnegative_amount(amount: i128) -> Result<(), TokenError> {
    if amount < 0 {
        Err(TokenError::NegativeAmount)
    } else {
        Ok(())
    }
}

#[contract]
pub struct TrackingToken;

#[contractimpl]
impl TrackingToken {
    /// Initialize the token with admin, decimals, name, and symbol
    pub fn initialize(
        env: Env,
        admin: Address,
        token_symbol: Symbol,
        decimal: u32,
        name: String,
    ) -> Result<(), TokenError> {
        // Check if already initialized
        if env
            .storage()
            .instance()
            .has(&DataKey::TokenInfo(token_symbol.clone()))
        {
            return Err(TokenError::AlreadyInitialized);
        }

        // Store token info
        let token_info = TokenInfo {
            decimals: decimal,
            name: name.clone(),
            symbol: token_symbol.clone(),
        };

        env.storage()
            .instance()
            .set(&DataKey::TokenInfo(token_symbol.clone()), &token_info);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply(token_symbol.clone()), &0i128);

        // Extend TTL for instance storage
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit initialization event
        env.events().publish(
            (Symbol::new(&env, "initialize"),),
            (admin.clone(), decimal, name, token_symbol),
        );

        Ok(())
    }

    /// Get token admin
    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    /// Set new admin (only current admin can call this)
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), TokenError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        env.storage().instance().extend_ttl(100, 1000000);

        env.events()
            .publish((Symbol::new(&env, "set_admin"),), (admin, new_admin));

        Ok(())
    }

    /// Get token decimals
    pub fn decimals(env: Env, token_symbol: Symbol) -> u32 {
        let token_info: TokenInfo = env
            .storage()
            .instance()
            .get(&DataKey::TokenInfo(token_symbol))
            .unwrap();
        token_info.decimals
    }

    /// Get token name
    pub fn name(env: Env, token_symbol: Symbol) -> String {
        let token_info: TokenInfo = env
            .storage()
            .instance()
            .get(&DataKey::TokenInfo(token_symbol))
            .unwrap();
        token_info.name
    }

    /// Get token symbol
    pub fn symbol(env: Env, token_symbol: Symbol) -> Symbol {
        let token_info: TokenInfo = env
            .storage()
            .instance()
            .get(&DataKey::TokenInfo(token_symbol))
            .unwrap();
        token_info.symbol
    }

    /// Get balance of an address
    pub fn balance(env: Env, id: Address, token_symbol: Symbol) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id, token_symbol))
            .unwrap_or(0)
    }

    /// Get total supply
    pub fn total_supply(env: Env, token_symbol: Symbol) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply(token_symbol))
            .unwrap_or(0)
    }

    /// Mint tokens (only admin can call)
    pub fn mint(
        env: Env,
        token_symbol: Symbol,
        to: Address,
        amount: i128,
    ) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        let balance_key = DataKey::Balance(to.clone(), token_symbol.clone());
        let current_balance: i128 = env.storage().persistent().get(&balance_key).unwrap_or(0);

        let new_balance = current_balance
            .checked_add(amount)
            .ok_or(TokenError::OverflowError)?;

        env.storage().persistent().set(&balance_key, &new_balance);
        env.storage()
            .persistent()
            .extend_ttl(&balance_key, 100, 1000000);

        // Update total supply
        let current_supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply(token_symbol.clone()))
            .unwrap_or(0);

        let new_supply = current_supply
            .checked_add(amount)
            .ok_or(TokenError::OverflowError)?;

        env.storage()
            .instance()
            .set(&DataKey::TotalSupply(token_symbol.clone()), &new_supply);
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit mint event
        env.events().publish(
            (Symbol::new(&env, "mint"),),
            MintEvent {
                admin: admin.clone(),
                token_symbol,
                to: to.clone(),
                amount,
            },
        );

        // Emit transfer event (from None)
        env.events().publish(
            (Symbol::new(&env, "transfer"),),
            TransferEvent {
                from: None,
                to: Some(to),
                amount,
            },
        );

        Ok(())
    }

    /// Burn tokens (only admin can call)
    pub fn burn(
        env: Env,
        token_symbol: Symbol,
        from: Address,
        amount: i128,
    ) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        let balance_key = DataKey::Balance(from.clone(), token_symbol.clone());
        let current_balance = env.storage().persistent().get(&balance_key).unwrap_or(0);

        if current_balance < amount {
            return Err(TokenError::BalanceError);
        }

        let new_balance = current_balance - amount;

        if new_balance > 0 {
            env.storage().persistent().set(&balance_key, &new_balance);
            env.storage()
                .persistent()
                .extend_ttl(&balance_key, 100, 1000000);
        } else {
            env.storage().persistent().remove(&balance_key);
        }

        // Update total supply
        let current_supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply(token_symbol.clone()))
            .unwrap_or(0);

        let new_supply = current_supply - amount;
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply(token_symbol.clone()), &new_supply);
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit burn event
        env.events().publish(
            (Symbol::new(&env, "burn"),),
            BurnEvent {
                token_symbol: token_symbol.clone(),
                from: from.clone(),
                amount,
            },
        );

        // Emit transfer event (to None)
        env.events().publish(
            (Symbol::new(&env, "transfer"),),
            TransferEvent {
                from: Some(from),
                to: None,
                amount,
            },
        );

        Ok(())
    }

    /// Burn tokens from own balance
    pub fn burn_from(
        env: Env,
        token_symbol: Symbol,
        from: Address,
        amount: i128,
    ) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;
        from.require_auth();

        let balance_key = DataKey::Balance(from.clone(), token_symbol.clone());
        let current_balance = env.storage().persistent().get(&balance_key).unwrap_or(0);

        if current_balance < amount {
            return Err(TokenError::BalanceError);
        }

        let new_balance = current_balance - amount;

        if new_balance > 0 {
            env.storage().persistent().set(&balance_key, &new_balance);
            env.storage()
                .persistent()
                .extend_ttl(&balance_key, 100, 1000000);
        } else {
            env.storage().persistent().remove(&balance_key);
        }

        // Update total supply
        let current_supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply(token_symbol.clone()))
            .unwrap_or(0);

        let new_supply = current_supply - amount;
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply(token_symbol.clone()), &new_supply);
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit transfer event (to None)
        env.events().publish(
            (Symbol::new(&env, "burn"),),
            BurnEvent {
                token_symbol: token_symbol.clone(),
                from: from.clone(),
                amount,
            },
        );

        Ok(())
    }

    /// Freeze/unfreeze an address (only admin can call)
    pub fn set_frozen(env: Env, id: Address, freeze: bool) -> Result<(), TokenError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        if freeze {
            env.storage()
                .persistent()
                .set(&DataKey::Frozen(id.clone()), &());
            env.storage()
                .persistent()
                .extend_ttl(&DataKey::Frozen(id.clone()), 100, 1000000);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::Frozen(id.clone()));
        }

        env.events()
            .publish((Symbol::new(&env, "set_frozen"),), (id, freeze));

        Ok(())
    }

    /// Check if address is frozen
    pub fn frozen(env: Env, id: Address) -> bool {
        env.storage().persistent().has(&DataKey::Frozen(id))
    }
}
