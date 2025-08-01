use soroban_sdk::{Address, Symbol, contracterror, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum AccountDataKey {
    UserAddresses,                          // List of all user addresses
    UserCollateralBalance(Address, Symbol), // Collateral balance for a specific user address, collateral asset symbol
    UserCollateralTokensList(Address),      // All collateral tokens symbols held by user address
    UserBorrowedDebt(Address, Symbol),      // User debt balance in a specific asset symbol
    UserBorrowedTokensList(Address),        // All borrowed tokens symbols held by user address
    TotalDebtInPool(Symbol),                // Total debt in pool for a specific asset symbol
    IsAccountInitialised(Address),          // Flag to check if account is initialized
    IsAccountActive(Address),               // Flag to check if account is active
    HasDebt(Address),                       // Flag to check if account has debt
    AccountCreatedTime(Address),            // Time when account was created
    AccountDeletedTime(Address),            // Time when account is deleted
    IsCollateralAllowed(Symbol),
    AssetCap,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum RiskEngineKey {
    RegistryContract,
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum RiskEngineError {
    RiskEngineNotInitialized = 1,
}
