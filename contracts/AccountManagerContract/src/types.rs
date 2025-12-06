use soroban_sdk::{Address, Symbol, U256, Vec, contracterror, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum AccountManagerKey {
    UsersList,                  // List of all unique users
    SmartAccounts(Address),     // User's smart account addresses
    TraderAddress(Address),     // Traders address for respective margin account.
    InactiveAccountOf(Address), // List of inactive accounts for a trader
    // UserBorrowedDebt(Address, Symbol), // User debt balance in a specific asset symbol
    // UserBorrowedTokensList(Address),   // All borrowed tokens symbols held by user address
    // TotalDebtInPool(Symbol), // Total debt in pool for a specific asset symbol
    IsAccountInitialised(Address), // Flag to check if account is initialized
    AccountCreatedTime(Address),   // Time when account was created
    AccountClosedTime(Address),    // Time when account is deleted
    IsCollateralAllowed(Symbol),
    AssetCap,
    Admin,
    RegistryContract,
}

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum AccountManagerError {
    CollateralTokenNotFound = 1,
    BorrowedTokenNotFound = 2,
    MarginAccountNotFound = 3,
    IntegerConversionError = 4,
    UserDoesntHaveCollateralToken = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountCreationEvent {
    pub smart_account: Address,
    pub creation_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountDeletionEvent {
    pub smart_account: Address,
    pub deletion_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderBorrowEvent {
    pub smart_account: Address,
    pub token_amount: U256,
    pub timestamp: u64,
    pub token_symbol: Symbol,
    pub token_value: U256,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderRepayEvent {
    pub smart_account: Address,
    pub token_amount: U256,
    pub timestamp: u64,
    pub token_symbol: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderLiquidateEvent {
    pub smart_account: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderSettleAccountEvent {
    pub smart_account: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalAction {
    Deposit,
    Swap,
    Withdraw,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalProtocolCall {
    pub protocol_address: Address,
    pub type_action: ExternalAction,
    pub tokens_out: Vec<Symbol>,
    pub tokens_in: Vec<Symbol>,
    pub amount_out: U256,
    pub amount_in: U256,
    pub is_token_pair: bool,
    pub token_pair_ratio: u64,
    pub margin_account: Address,
}
