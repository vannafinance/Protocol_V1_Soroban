use soroban_sdk::{Address, Env, Symbol, contract, contractimpl, symbol_short};

use crate::types::RegistryContractError;

#[contract]
pub struct RegistryContract;
const ADMIN: Symbol = symbol_short!("admin");

#[contractimpl]
impl RegistryContract {
    pub fn __constructor(env: Env, admin: Address) -> Result<(), RegistryContractError> {
        env.storage().instance().set(&ADMIN, &admin);
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        Ok(())
    }
}
