use soroban_sdk::Address;
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
    UsersList,         // List of all unique trader addresses
    SmartAccountsList, // List of of all smart accounts
    //SmartAccountAddress(Address), // Traders's smart account address
    OwnerAddress(Address), // Traders address for respective margin account
    BlendPoolContract,     // Blend Pool Contract Address
    SoroswapContract,
    AquariusContract,
}
