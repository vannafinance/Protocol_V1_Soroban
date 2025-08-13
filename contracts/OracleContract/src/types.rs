use soroban_sdk::{Address, String, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum OracleDataKey {
    Admin,
    PriceData(String),   // Asset pair price data
    Oracle,              // Authorized oracle address
    User(Address),       // User account data
    Governance,          // Governance parameters
    StdReferenceAddress, // Oracle reference address
    ReflectorAddress,
}
