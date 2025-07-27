use soroban_sdk::{Address, Symbol, U256, contracttype};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingDepositEvent {
    pub lender: Address,
    pub amount: U256,   // USDC amount (with 6 decimals)
    pub timestamp: u64, // Ledger timestamp
    pub asset_symbol: Symbol, // USDC symbol
                        // pub pool_id: BytesN<32>,  // Unique pool identifier
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingWithdrawEvent {
    pub lender: Address,
    pub vtoken_amount: U256,
    pub timestamp: u64, // Ledger timestamp
    pub asset_symbol: Symbol, // asset symbol
                        // pub pool_id: BytesN<32>,  // Unique pool identifier
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingTokenMintEvent {
    pub lender: Address,
    pub token_amount: U256,
    pub timestamp: u64,
    pub token_symbol: Symbol,
    pub token_value: U256,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingTokenBurnEvent {
    pub lender: Address,
    pub token_amount: U256,
    pub timestamp: u64,
    pub token_symbol: Symbol,
    pub token_value: U256,
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
pub struct TraderAddCollateralEvent {
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
