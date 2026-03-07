use crate::account_manager::smart_account_contract::SmartAccExternalAction;
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
pub struct ExternalProtocolCall {
    pub protocol_address: Address,              // Protocol address (Blend pool or Aquarius router)
    pub type_action: SmartAccExternalAction,    // Deposit/Withdraw/Swap/AddLiquidity/RemoveLiquidity
    pub tokens_out: Vec<Symbol>,                // Output tokens
    pub tokens_in: Vec<Symbol>,                 // Input tokens (for swaps/liquidity)
    pub amount_out: Vec<U256>,                  // Amounts in WAD
    pub amount_in: Vec<U256>,                   // Input amounts
    pub is_token_pair: bool,                    // For liquidity pool operations
    pub token_pair_ratio: u64,                  // Ratio for token pairs (not used currently)
    pub margin_account: Address,                // Smart account address
    pub fee_fraction: u32,                      // Fee for Aquarius pools (e.g., 30 = 0.3%)
    pub min_liquidity_out: U256,                // Minimum LP tokens to receive (slippage protection)
}
