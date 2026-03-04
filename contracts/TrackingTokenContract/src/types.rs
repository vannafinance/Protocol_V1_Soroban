use soroban_sdk::{Address, String, Symbol, contracterror, contracttype};

// Contract data keys
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    TokenInfo(Symbol),
    Balance(Address, Symbol),
    Allowance(AllowanceDataKey),
    TotalSupply(Symbol),
    Authorized(Address),
    Frozen(Address),
}

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct TokenInfo {
    pub decimals: u32,
    pub name: String,
    pub symbol: Symbol,
}

// Events
#[derive(Clone)]
#[contracttype]
pub struct TransferEvent {
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct ApprovalEvent {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct MintEvent {
    pub admin: Address,
    pub token_symbol: Symbol,
    pub to: Address,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct BurnEvent {
    // pub admin: Address,
    pub token_symbol: Symbol,
    pub from: Address,
    pub amount: i128,
}

// Custom errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TokenError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NegativeAmount = 3,
    AllowanceError = 4,
    BalanceError = 5,
    OverflowError = 6,
    Unauthorized = 7,
    NotAuthorized = 8,
    Frozen = 9,
}
