use soroban_sdk::{Address, String, Symbol, Vec, contracttype};

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
pub enum ContractDetails {
    RegistryContract,
    RateModel,
    AccountManager,
    Treasury,
}
