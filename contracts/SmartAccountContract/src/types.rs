use soroban_sdk::{Address, Symbol, contracterror, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum SmartAccountDataKey {
    // UserAddresses,                // List of all user addresses
    CollateralBalanceWAD(Symbol), // Collateral balance for a specific user address, collateral asset symbol
    CollateralTokensList,         // All collateral tokens symbols held by user address
    // // BorrowedDebtWAD(Symbol),      // User debt balance in a specific asset symbol
    BorrowedTokensList, // All borrowed tokens symbols held by user address
    IsAccountActive,    // Flag to check if account is active
    HasDebt,            // Flag to check if account has debt
    // AccountCreatedTime,           // Time when account was created
    // AccountDeletedTime,           // Time when account is deleted
    AccountManager,
    RegistryContract,
    OwnerAddress,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum SmartAccExternalAction {
    Deposit,
    Swap,
    Withdraw,
}

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum SmartAccountError {
    CollateralTokenNotFound = 1,
    BorrowedTokenNotFound = 2,
    MarginAccountNotFound = 3,
    IntegerConversionError = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartAccountDeactivationEvent {
    pub margin_account: Address,
    pub deactivate_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartAccountActivationEvent {
    pub margin_account: Address,
    pub activated_time: u64,
}
