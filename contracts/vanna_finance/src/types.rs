use core::ops::Add;

use soroban_sdk::{contracttype, Address, String, Symbol, Vec};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum DataKey {
    //// WORK IN PROGRESS.. Data structures may change for better optimisation
    /// Sample
    Admin,
    PriceData(String),   // Asset pair price data
    Oracle,              // Authorized oracle address
    User(Address),       // User account data
    Loan(u32),           // Loan details (global loan ID)
    LoanCounter,         // Global loan counter
    Governance,          // Governance parameters
    StdReferenceAddress, // Oracle reference address
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum AccountManagerKey {
    Admin,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum PoolDataKey {
    Deployed,                       // Whether the pool has been deployed
    LenderBalance(Address, Symbol), // Lender balance for a specific user address, asset symbol
    Lenders(Symbol),                // List of all lenders for particular asset symbol
    Pool(Symbol),                   // Liquidity pool balance for each asset symbol
    PoolAddress(Symbol),            // Pool Address for each token
    TotalBorrowShares,              // Total borrow shares of all users
    UserBorrowShares(Address),      // Borrow shares of a user
    LastUpdatedTime,                // Last time the pool data was updated
    Borrows,                        // Total borrowed asset value
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum TokenDataKey {
    TotalTokensMinted(Symbol),
    TotalTokensBurnt(Symbol),
    CurrentVTokenBalance(Symbol),
    VTokenBalance(Address, Symbol),
    VTokenValue(Symbol),
    VTokenClientAddress(Symbol),
    UsdcClientAddress,
    EurcClientAddress,
    NativeXLMClientAddress,
    TokenIssuerAddress,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum MarginAccountDataKey {
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
pub enum LendingProtocols {
    XlmAddress,
    EurcAddress,
    UsdcAddress,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum BorrowDataKey {
    IsBorrowAllowed(Address, Symbol), // Flag to check if borrow is allowed for a user for a specific asset symbol
    BorrowLimit(Address, Symbol),     // Borrow limit for a user for a specific asset symbol
    IsWithDrawAllowed(Address, Symbol), // Flag to check if withdraw is allowed for a user for a specific asset symbol
    WithdrawLimit(Address, Symbol),     // Withdraw limit for a user for a specific asset symbol
    IsAccountHealthy(Address),          // Flag to check if account is healthy
    LastUpdatedTime(Symbol),            // Last updated time for token symbol
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum ContractDetails {
    RegistryContract,
    RateModel,
    AccountManager,
    Treasury,
}
