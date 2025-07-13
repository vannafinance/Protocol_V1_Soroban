// In a separate file 'deployer.rs'
#[contract]
pub struct LiquidityDeployer;

#[contractimpl]
impl LiquidityDeployer {
    pub fn deploy_liquidity_pool(
        env: Env,
        admin: Address,
        liquidity_pool_wasm_hash: BytesN<32>,
    ) -> Address {
        // Verify deployer is the intended admin
        admin.require_auth();

        // Create the liquidity pool contract
        let contract_address = env
            .deployer()
            .with_current_contract(liquidity_pool_wasm_hash)
            .deploy();

        // Initialize the admin
        let client = LiquidityPoolXLMClient::new(&env, &contract_address);
        client.private_deploy(&admin);

        contract_address
    }
}
