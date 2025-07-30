use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug)]
pub enum RegistryContractError {
    CollateralTokenNotFound = 1,
    BorrowedTokenNotFound = 2,
    MarginAccountNotFound = 3,
}
