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
pub struct VUSDCToken;

#[contractimpl]
impl VUSDCToken {
    /// Initialize the token with admin, decimals, name, and symbol
    pub fn initialize(
        env: Env,
        admin: Address,
        decimal: u32,
        name: String,
        symbol: String,
    ) -> Result<(), TokenError> {
        // Check if already initialized
        if env.storage().instance().has(&DataKey::TokenInfo) {
            return Err(TokenError::AlreadyInitialized);
        }

        // Store token info
        let token_info = TokenInfo {
            decimals: decimal,
            name: name.clone(),
            symbol: symbol.clone(),
        };

        env.storage()
            .instance()
            .set(&DataKey::TokenInfo, &token_info);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TotalSupply, &0i128);

        // Extend TTL for instance storage
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit initialization event
        env.events().publish(
            (Symbol::new(&env, "initialize"),),
            (admin.clone(), decimal, name, symbol),
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
    pub fn decimals(env: Env) -> u32 {
        let token_info: TokenInfo = env.storage().instance().get(&DataKey::TokenInfo).unwrap();
        token_info.decimals
    }

    /// Get token name
    pub fn name(env: Env) -> String {
        let token_info: TokenInfo = env.storage().instance().get(&DataKey::TokenInfo).unwrap();
        token_info.name
    }

    /// Get token symbol
    pub fn symbol(env: Env) -> String {
        let token_info: TokenInfo = env.storage().instance().get(&DataKey::TokenInfo).unwrap();
        token_info.symbol
    }

    /// Get balance of an address
    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    /// Get total supply
    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    /// Transfer tokens from one address to another
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;
        from.require_auth();

        // Check if from address is frozen
        if env
            .storage()
            .persistent()
            .has(&DataKey::Frozen(from.clone()))
        {
            return Err(TokenError::Frozen);
        }

        Self::transfer_internal(&env, from.clone(), to.clone(), amount)?;

        // Emit transfer event
        env.events().publish(
            (Symbol::new(&env, "transfer"),),
            TransferEvent {
                from: Some(from),
                to: Some(to),
                amount,
            },
        );

        Ok(())
    }

    /// Transfer from (with allowance)
    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;
        spender.require_auth();

        // Check if from address is frozen
        if env
            .storage()
            .persistent()
            .has(&DataKey::Frozen(from.clone()))
        {
            return Err(TokenError::Frozen);
        }

        let allowance_key = DataKey::Allowance(AllowanceDataKey {
            from: from.clone(),
            spender: spender.clone(),
        });

        let allowance: i128 = env.storage().persistent().get(&allowance_key).unwrap_or(0);

        if allowance < amount {
            return Err(TokenError::AllowanceError);
        }

        // Update allowance
        env.storage()
            .persistent()
            .set(&allowance_key, &(allowance - amount));

        if allowance - amount > 0 {
            env.storage()
                .persistent()
                .extend_ttl(&allowance_key, 100, 1000000);
        }

        Self::transfer_internal(&env, from.clone(), to.clone(), amount)?;

        // Emit transfer event
        env.events().publish(
            (Symbol::new(&env, "transfer"),),
            TransferEvent {
                from: Some(from),
                to: Some(to),
                amount,
            },
        );

        Ok(())
    }

    /// Approve spender to spend tokens
    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
    ) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;
        from.require_auth();

        let allowance_key = DataKey::Allowance(AllowanceDataKey {
            from: from.clone(),
            spender: spender.clone(),
        });

        env.storage().persistent().set(&allowance_key, &amount);

        if amount > 0 {
            env.storage()
                .persistent()
                .extend_ttl(&allowance_key, 100, 1000000);
        }

        // Emit approval event
        env.events().publish(
            (Symbol::new(&env, "approve"),),
            ApprovalEvent {
                from: from.clone(),
                to: spender.clone(),
                amount,
            },
        );

        Ok(())
    }

    /// Get allowance
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        let allowance_key = DataKey::Allowance(AllowanceDataKey { from, spender });
        env.storage().persistent().get(&allowance_key).unwrap_or(0)
    }

    /// Mint tokens (only admin can call)
    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        let balance_key = DataKey::Balance(to.clone());
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
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);

        let new_supply = current_supply
            .checked_add(amount)
            .ok_or(TokenError::OverflowError)?;

        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &new_supply);
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit mint event
        env.events().publish(
            (Symbol::new(&env, "mint"),),
            MintEvent {
                admin: admin.clone(),
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
    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        let balance_key = DataKey::Balance(from.clone());
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
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);

        let new_supply = current_supply - amount;
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &new_supply);
        env.storage().instance().extend_ttl(100, 1000000);

        // Emit burn event
        env.events().publish(
            (Symbol::new(&env, "burn"),),
            BurnEvent {
                admin: admin.clone(),
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
    pub fn burn_from(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        check_nonnegative_amount(amount)?;
        from.require_auth();

        let balance_key = DataKey::Balance(from.clone());
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
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);

        let new_supply = current_supply - amount;
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &new_supply);
        env.storage().instance().extend_ttl(100, 1000000);

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

    /// Authorize an address (only admin can call)
    pub fn set_authorized(env: Env, id: Address, authorize: bool) -> Result<(), TokenError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(TokenError::NotInitialized)?;

        admin.require_auth();

        if authorize {
            env.storage()
                .persistent()
                .set(&DataKey::Authorized(id.clone()), &());
            env.storage()
                .persistent()
                .extend_ttl(&DataKey::Authorized(id.clone()), 100, 1000000);
        } else {
            env.storage()
                .persistent()
                .remove(&DataKey::Authorized(id.clone()));
        }

        env.events()
            .publish((Symbol::new(&env, "set_authorized"),), (id, authorize));

        Ok(())
    }

    /// Check if address is authorized
    pub fn authorized(env: Env, id: Address) -> bool {
        env.storage().persistent().has(&DataKey::Authorized(id))
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

    // Internal transfer function
    fn transfer_internal(
        env: &Env,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), TokenError> {
        if amount == 0 {
            return Ok(());
        }

        let from_key = DataKey::Balance(from.clone());
        let to_key = DataKey::Balance(to.clone());

        let from_balance = env.storage().persistent().get(&from_key).unwrap_or(0);

        if from_balance < amount {
            return Err(TokenError::BalanceError);
        }

        let to_balance: i128 = env.storage().persistent().get(&to_key).unwrap_or(0);

        let new_from_balance: i128 = from_balance - amount;
        let new_to_balance = to_balance
            .checked_add(amount)
            .ok_or(TokenError::OverflowError)?;

        // Update balances
        if new_from_balance > 0 {
            env.storage().persistent().set(&from_key, &new_from_balance);
            env.storage()
                .persistent()
                .extend_ttl(&from_key, 100, 1000000);
        } else {
            env.storage().persistent().remove(&from_key);
        }

        env.storage().persistent().set(&to_key, &new_to_balance);
        env.storage().persistent().extend_ttl(&to_key, 100, 1000000);

        Ok(())
    }
}

// // Implement the Soroban token trait for compatibility
// impl token::TokenInterface for VUSDCToken {
//     fn allowance(env: Env, from: Address, spender: Address) -> i128 {
//         Self::allowance(env, from, spender)
//     }

//     fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
//         from.require_auth();
//         Self::approve(env, from, spender, amount).unwrap();
//         // Note: This implementation ignores expiration_ledger for simplicity
//         // You can enhance it to support expiration
//     }

//     fn balance(env: Env, id: Address) -> i128 {
//         Self::balance(env, id)
//     }

//     fn transfer(env: Env, from: Address, to: Address, amount: i128) {
//         Self::transfer(env, from, to, amount).unwrap();
//     }

//     fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
//         Self::transfer_from(env, spender, from, to, amount).unwrap();
//     }

//     fn burn(env: Env, from: Address, amount: i128) {
//         Self::burn_from(env, from, amount).unwrap();
//     }

//     fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
//         spender.require_auth();
//         // This would need allowance check in full implementation
//         Self::burn(env, from, amount).unwrap();
//     }

//     fn decimals(env: Env) -> u32 {
//         Self::decimals(env)
//     }

//     fn name(env: Env) -> String {
//         Self::name(env)
//     }

//     fn symbol(env: Env) -> String {
//         Self::symbol(env)
//     }
// }
