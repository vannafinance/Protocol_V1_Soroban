use soroban_sdk::{contracterror, contracttype};

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum RegistryContractError {
    CollateralTokenNotFound = 1,
    BorrowedTokenNotFound = 2,
    MarginAccountNotFound = 3,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum RegistryKey {
    LendingPoolXlm,
    LendingPoolUsdc,
    LendingPoolEurc,
    RateModelContract,
    OracleContract,
    RiskEngineContract,
    SmartAccountContractHash,
    AccountManagerContract,
    NativeXlmContractAddress,
    UsdcContractAddress,
    EurcContractAddress,
}
