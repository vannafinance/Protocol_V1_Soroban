use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum LendingError {
    NotInitialized = 1,
    Unauthorized = 2,
    InsufficientBalance = 3,
    InvalidLTV = 4,
    PriceNotFound = 5,
    StalePrice = 6,
    LoanNotFound = 7,
    NotUndercollateralized = 8,
    UserNotFound = 9,
    InvalidRole = 10,
    PoolNotInitialized = 11,
    LenderNotRegistered = 12,
}
