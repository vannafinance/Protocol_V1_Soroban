use soroban_sdk::{contracttype, Address, Symbol, U256};

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
    pub amount: U256,
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
