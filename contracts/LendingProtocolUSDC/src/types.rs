use soroban_sdk::{Address, String, Symbol, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum PoolDataKey {
    Admin,
    Initialised,                  // Whether the pool has been Initialised
    Lenders(Symbol),              // List of all lenders for particular asset symbol
    PoolAddress(Symbol),          // Pool Address for each token
    TotalBorrowSharesWAD,         // Total borrow shares of all users
    UserBorrowSharesWAD(Address), // Borrow shares of a user
    LastUpdatedTime,              // Last time the pool data was updated
    BorrowsWAD,                   // Total borrowed asset value
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum TokenDataKey {
    TotalTokensMintedWAD(Symbol),
    TotalTokensBurntWAD(Symbol),
    CurrentVTokenBalanceWAD(Symbol),
    VTokenBalance(Address, Symbol),
    VTokenValue(Symbol),
    VTokenContractAddress(Symbol),
    NativeUSDCAddress,
    TokenIssuerAddress,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum ContractDetails {
    RegistryContract,
    RateModel,
    AccountManager,
    Treasury,
    OriginationFee,
}
