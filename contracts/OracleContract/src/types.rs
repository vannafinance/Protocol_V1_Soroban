use soroban_sdk::{Address, String, contracttype};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum DataKey {
    //// WORK IN PROGRESS.. Data structures may change for better optimisation
    /// Sample
    Admin,
    PriceData(String),   // Asset pair price data
    Oracle,              // Authorized oracle address
    User(Address),       // User account data
    Governance,          // Governance parameters
    StdReferenceAddress, // Oracle reference address
}
