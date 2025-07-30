use soroban_sdk::{Address, String, Symbol, U256, Vec, contracterror, contracttype};

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

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum AccountManagerError {
    CollateralTokenNotFound = 1,
    BorrowedTokenNotFound = 2,
    MarginAccountNotFound = 3,
    IntegerConversionError = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountDeletionEvent {
    pub margin_account: Address,
    pub deletion_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderBorrowEvent {
    pub margin_account: Address,
    pub token_amount: U256,
    pub timestamp: u64,
    pub token_symbol: Symbol,
    pub token_value: U256,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderRepayEvent {
    pub margin_account: Address,
    pub token_amount: U256,
    pub timestamp: u64,
    pub token_symbol: Symbol,
    pub token_value: U256,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderLiquidateEvent {
    pub margin_account: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderSettleAccountEvent {
    pub margin_account: Address,
    pub timestamp: u64,
}
