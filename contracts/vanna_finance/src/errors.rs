use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum LendingError {
    NotInitialized = 1,
    Unauthorized = 2,
    InsufficientBalance = 3,
    InvalidLTV = 4,
    // This data structure is Work in progress
    // Shall be modified further !!!!!!
    PriceNotFound = 5,
    StalePrice = 6,
    NotUndercollateralized = 8,
    UserNotFound = 9,
    InvalidRole = 10,
    PoolNotInitialized = 11,
    LenderNotRegistered = 12,
    InsufficientPoolBalance = 13,
}

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum LendingTokenError {
    TokenBalanceNotInitialised = 1,
    Unauthorized = 2,
    InsufficientTokenBalance = 3,
}
