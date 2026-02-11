use soroban_sdk::{Address, BytesN, Env, Symbol, Vec, contract, contractimpl, symbol_short};

use crate::types::{RegistryContractError, RegistryKey};

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const _TLL_LEDGERS_MONTH: u32 = 518400;

#[contract]
pub struct RegistryContract;
const ADMIN: Symbol = symbol_short!("admin");

#[contractimpl]
impl RegistryContract {
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().persistent().set(&ADMIN, &admin);
        env.storage()
            .persistent()
            .extend_ttl(&ADMIN, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    pub fn set_lendingpool_xlm(
        env: &Env,
        lendingpool_xlm: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
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
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::OracleContract, &oracle_contract_address);
        Self::extend_ttl_registry(env, RegistryKey::OracleContract);

        Ok(())
    }

    pub fn set_native_xlm_contract_address(
        env: &Env,
        xlm_contract_adddress: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage().persistent().set(
            &RegistryKey::NativeXlmContractAddress,
            &xlm_contract_adddress,
        );
        Self::extend_ttl_registry(env, RegistryKey::NativeXlmContractAddress);

        Ok(())
    }

    pub fn set_native_usdc_contract_address(
        env: &Env,
        usdc_contract_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::UsdcContractAddress, &usdc_contract_address);
        Self::extend_ttl_registry(env, RegistryKey::UsdcContractAddress);

        Ok(())
    }

    pub fn set_native_eurc_contract_address(
        env: &Env,
        eurc_contract_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::EurcContractAddress, &eurc_contract_address);
        Self::extend_ttl_registry(env, RegistryKey::EurcContractAddress);

        Ok(())
    }

    pub fn set_blend_pool_address(
        env: &Env,
        blend_pool_address: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&RegistryKey::BlendPoolContract, &blend_pool_address);
        Self::extend_ttl_registry(env, RegistryKey::BlendPoolContract);

        Ok(())
    }

    pub fn set_tracking_token_contract_addr(
        env: &Env,
        tracking_token_contract_addr: Address,
    ) -> Result<(), RegistryContractError> {
        let admin: Address = env.storage().persistent().get(&ADMIN).unwrap();
        admin.require_auth();

        env.storage().persistent().set(
            &RegistryKey::TrackingTokenContract,
            &tracking_token_contract_addr,
        );
        Self::extend_ttl_registry(env, RegistryKey::TrackingTokenContract);

        Ok(())
    }

    pub fn get_lendingpool_xlm(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::LendingPoolXlm)
            .unwrap_or_else(|| panic!("Failed to get lendingpool_xlm address"));
        Ok(res)
    }

    pub fn get_smart_account_hash(env: &Env) -> Result<BytesN<32>, RegistryContractError> {
        let res: BytesN<32> = env
            .storage()
            .persistent()
            .get(&RegistryKey::SmartAccountContractHash)
            .unwrap_or_else(|| panic!("Failed to get smart account hash"));
        Ok(res)
    }

    pub fn get_accountmanager_contract(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::AccountManagerContract)
            .unwrap_or_else(|| panic!("Failed to get account_manager address"));
        Ok(res)
    }

    pub fn get_lendingpool_eurc(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::LendingPoolEurc)
            .unwrap_or_else(|| panic!("Failed to get lendingpool_eurc address"));
        Ok(res)
    }

    pub fn get_lendingpool_usdc(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::LendingPoolUsdc)
            .unwrap_or_else(|| panic!("Failed to get lendingpool_usdc address"));
        Ok(res)
    }

    pub fn get_risk_engine_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::RiskEngineContract)
            .unwrap_or_else(|| panic!("Failed to get risk_enginer contract address"));
        Ok(res)
    }

    pub fn get_rate_model_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::RateModelContract)
            .unwrap_or_else(|| panic!("Failed to get rate_model contract address"));
        Ok(res)
    }

    pub fn get_oracle_contract_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::OracleContract)
            .unwrap_or_else(|| panic!("Failed to get oracle contract address"));
        Ok(res)
    }

    pub fn get_xlm_contract_adddress(env: &Env) -> Result<Address, RegistryContractError> {
        let token_contract_address: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::NativeXlmContractAddress)
            .expect("Failed to fetch native token contract address for XLM");

        Ok(token_contract_address)
    }

    pub fn get_usdc_contract_address(env: &Env) -> Result<Address, RegistryContractError> {
        let token_contract_address: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::UsdcContractAddress)
            .expect("Failed to fetch token contract address for USDC");

        Ok(token_contract_address)
    }

    pub fn get_blend_pool_address(env: &Env) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::BlendPoolContract)
            .unwrap_or_else(|| panic!("Failed to get blend_pool contract address"));
        Ok(res)
    }

    pub fn get_tracking_token_contract_addr(
        env: &Env,
    ) -> Result<Address, RegistryContractError> {
        let res: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::TrackingTokenContract)
            .unwrap_or_else(|| panic!("Failed to get tracking_token contract address"));
        Ok(res)
    }

    pub fn get_eurc_contract_address(env: &Env) -> Result<Address, RegistryContractError> {
        let token_contract_address: Address = env
            .storage()
            .persistent()
            .get(&RegistryKey::EurcContractAddress)
            .expect("Failed to fetch token contract address for EURC");

        Ok(token_contract_address)
    }

    pub fn get_admin(env: &Env) -> Result<Address, RegistryContractError> {
        env.storage()
            .persistent()
            .get(&ADMIN)
            .expect("Failed to fetch admin address")
    }
    pub fn add_account(
        env: &Env,
        trader: Address,
        smart_account: Address,
    ) -> Result<bool, RegistryContractError> {
        let acc_manager = Self::get_accountmanager_contract(env).unwrap();
        acc_manager.require_auth();

        let mut accounts_list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&RegistryKey::SmartAccountsList)
            .unwrap_or(Vec::new(env));

        if !accounts_list.contains(smart_account.clone()) {
            accounts_list.push_back(smart_account.clone());
            Self::set_smart_accounts_list(env, accounts_list);
        }

        env.storage().persistent().set(
            &RegistryKey::OwnerAddress(smart_account.clone()),
            &Some(trader),
        );
        Self::extend_ttl_registry(&env, RegistryKey::OwnerAddress(smart_account.clone()));

        Ok(true)
    }

    pub fn close_account(
        env: &Env,
        trader: Address,
        smart_account: Address,
    ) -> Result<bool, RegistryContractError> {
        let acc_manager = Self::get_accountmanager_contract(env).unwrap();
        acc_manager.require_auth();

        env.storage().persistent().set(
            &RegistryKey::OwnerAddress(smart_account.clone()),
            &None::<Address>,
        );
        Self::extend_ttl_registry(&env, RegistryKey::OwnerAddress(smart_account.clone()));

        Ok(true)
    }

    pub fn update_account(
        env: &Env,
        trader: Address,
        smart_account: Address,
    ) -> Result<bool, RegistryContractError> {
        let acc_manager = Self::get_accountmanager_contract(env).unwrap();
        acc_manager.require_auth();
        env.storage().persistent().set(
            &RegistryKey::OwnerAddress(smart_account.clone()),
            &Some(trader),
        );
        Self::extend_ttl_registry(&env, RegistryKey::OwnerAddress(smart_account.clone()));

        Ok(true)
    }

    fn set_smart_accounts_list(env: &Env, list: Vec<Address>) {
        env.storage()
            .persistent()
            .set(&RegistryKey::SmartAccountsList, &list);
        Self::extend_ttl_registry(&env, RegistryKey::SmartAccountsList);
    }

    fn extend_ttl_registry(env: &Env, key: RegistryKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }
}
