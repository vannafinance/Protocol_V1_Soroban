use soroban_sdk::{Address, BytesN, Env, Symbol, contract, contractimpl, symbol_short};

use crate::types::{RegistryContractError, RegistryKey};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;

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
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::LendingPoolXlm, &lendingpool_xlm);
        Self::extend_ttl_registry(env, RegistryKey::LendingPoolXlm);
        Ok(())
    }

    pub fn set_smart_account_hash(
        env: &Env,
        smart_account_hash: BytesN<32>,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::SmartAccountContractHash, &smart_account_hash);

        Self::extend_ttl_registry(env, RegistryKey::SmartAccountContractHash);

        Ok(())
    }

    pub fn set_accountmanager_contract(
        env: &Env,
        account_manager_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage().persistent().set(
            &RegistryKey::AccountManagerContract,
            &account_manager_address,
        );
        Self::extend_ttl_registry(env, RegistryKey::AccountManagerContract);

        Ok(())
    }

    pub fn set_lendingpool_eurc(
        env: &Env,
        lendingpool_eurc_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::LendingPoolEurc, &lendingpool_eurc_address);
        Self::extend_ttl_registry(env, RegistryKey::LendingPoolEurc);

        Ok(())
    }

    pub fn set_lendingpool_usdc(
        env: &Env,
        lendingpool_usdc_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::LendingPoolUsdc, &lendingpool_usdc_address);
        Self::extend_ttl_registry(env, RegistryKey::LendingPoolUsdc);

        Ok(())
    }

    pub fn set_risk_engine_address(
        env: &Env,
        risk_engine_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::RiskEngineContract, &risk_engine_address);
        Self::extend_ttl_registry(env, RegistryKey::RiskEngineContract);

        Ok(())
    }

    pub fn set_rate_model_address(
        env: &Env,
        rate_model_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::RateModelContract, &rate_model_address);
        Self::extend_ttl_registry(env, RegistryKey::RateModelContract);

        Ok(())
    }

    pub fn set_oracle_contract_address(
        env: &Env,
        oracle_contract_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::OracleContract, &oracle_contract_address);
        Self::extend_ttl_registry(env, RegistryKey::OracleContract);

        Ok(())
    }

    pub fn set_xlm_token_contract_adddress(
        env: &Env,
        xlm_contract_adddress: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage().persistent().set(
            &RegistryKey::NativeXlmContractAddress,
            &xlm_contract_adddress,
        );
        Self::extend_ttl_registry(env, RegistryKey::NativeXlmContractAddress);

        Ok(())
    }

    pub fn set_usdc_contract_address(
        env: &Env,
        usdc_contract_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::UsdcContractAddress, &usdc_contract_address);
        Self::extend_ttl_registry(env, RegistryKey::UsdcContractAddress);

        Ok(())
    }

    pub fn set_eurc_contract_address(
        env: &Env,
        eurc_contract_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::EurcContractAddress, &eurc_contract_address);
        Self::extend_ttl_registry(env, RegistryKey::EurcContractAddress);

        Ok(())
    }

    pub fn get_lendingpool_xlm(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::LendingPoolXlm)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_smart_account_hash(env: &Env) -> Result<BytesN<32>, RegistryContractError> {
        let res: BytesN<32> = env
            .storage()
            .persistent()
            .get(&RegistryKey::SmartAccountContractHash)
            .unwrap_or_else(|| panic!("Failed to get Hash"));
        Ok(res)
    }

    pub fn get_accountmanager_contract(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::AccountManagerContract)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_lendingpool_eurc(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::LendingPoolEurc)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_lendingpool_usdc(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::LendingPoolUsdc)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_risk_engine_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::RiskEngineContract)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_rate_model_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::RateModelContract)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_oracle_contract_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::OracleContract)
            .unwrap_or_else(|| panic!("Failed to get address"));
        Ok(res)
    }

    pub fn get_xlm_token_contract_adddress(env: &Env) -> Result<Address, RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let token_contract_address: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::NativeXlmContractAddress)
            .expect("Failed to fetch token contract address for XLM");

        Ok(token_contract_address)
    }

    pub fn get_usdc_contract_address(env: &Env) -> Result<Address, RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let token_contract_address: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::UsdcContractAddress)
            .expect("Failed to fetch token contract address for USDC");

        Ok(token_contract_address)
    }

    pub fn get_eurc_contract_address(env: &Env) -> Result<Address, RegistryContractError> {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();

        let token_contract_address: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::EurcContractAddress)
            .expect("Failed to fetch token contract address for EURC");

        Ok(token_contract_address)
    }

    fn extend_ttl_registry(env: &Env, key: RegistryKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
