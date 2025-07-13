use soroban_sdk::{contracttype, Address, String, Symbol};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum DataKey {
    //// WORK IN PROGRESS.. Data structures may change for better optimisation
    /// Sample
    Admin,
    PriceData(String), // Asset pair price data
    Oracle,            // Authorized oracle address
    User(Address),     // User account data
    Loan(u32),         // Loan details (global loan ID)
    LoanCounter,       // Global loan counter
    Governance,        // Governance parameters
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum PoolDataKey {
    Deployed,                       // Whether the pool has been deployed
    LenderBalance(Address, Symbol), // Lender balance for a specific user address, asset symbol
    Lenders(Symbol),                // List of all lenders for particular asset symbol
    Pool(Symbol),                   // Liquidity pool for each asset symbol
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum TokenDataKey {
    TotalTokensMinted(Symbol),
    TotalTokensBurnt(Symbol),
    CurrentTokenBalance(Symbol),
    TokenBalance(Address, Symbol),
    TokenValue(Symbol),
}
