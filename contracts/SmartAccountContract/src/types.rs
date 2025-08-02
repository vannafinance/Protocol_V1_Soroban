use soroban_sdk::{Address, Symbol, contracterror, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum SmartAccountDataKey {
    UserAddresses,             // List of all user addresses
    CollateralBalance(Symbol), // Collateral balance for a specific user address, collateral asset symbol
    CollateralTokensList,      // All collateral tokens symbols held by user address
    BorrowedDebt(Symbol),      // User debt balance in a specific asset symbol
    BorrowedTokensList,        // All borrowed tokens symbols held by user address
    TotalDebtInPool(Symbol),   // Total debt in pool for a specific asset symbol
    IsAccountActive,           // Flag to check if account is active
    HasDebt,                   // Flag to check if account has debt
    AccountCreatedTime,        // Time when account was created
    AccountDeletedTime,        // Time when account is deleted
    IsCollateralAllowed(Symbol),
    AssetCap,
    AccountManager,
    RegistryContract,
    OwnerAddress,
}

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum SmartAccountError {
    CollateralTokenNotFound = 1,
    BorrowedTokenNotFound = 2,
    MarginAccountNotFound = 3,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartAccountDeactivationEvent {
    pub margin_account: Address,
    pub deactivate_time: u64,
}
