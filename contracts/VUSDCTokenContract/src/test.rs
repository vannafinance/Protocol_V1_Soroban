// Tests
#[cfg(test)]
mod tests {
    use crate::v_usdc::{VUSDCToken, VUSDCTokenClient};

    use soroban_sdk::IntoVal;
    use soroban_sdk::{Address, Env, testutils::Address as _};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let contract_id = env.register(VUSDCToken, ());
        let client = VUSDCTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);

        client.initialize(
            &admin,
            &7u32,
            &"Vault XLM".into_val(&env),
            &"vXLM".into_val(&env),
        );

        assert_eq!(client.admin(), admin);
        assert_eq!(client.decimals(), 7u32);
        assert_eq!(client.name(), "Vault XLM".into_val(&env));
        assert_eq!(client.symbol(), "vXLM".into_val(&env));
        assert_eq!(client.total_supply(), 0i128);
    }

    #[test]
    fn test_mint_and_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VUSDCToken, ());
        let client = VUSDCTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        client.initialize(
            &admin,
            &7u32,
            &"Vault XLM".into_val(&env),
            &"vXLM".into_val(&env),
        );

        // Mint tokens
        client.mint(&user, &1000i128);

        assert_eq!(client.balance(&user), 1000i128);
        assert_eq!(client.total_supply(), 1000i128);
    }

    #[test]
    fn test_transfer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VUSDCToken, ());
        let client = VUSDCTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        client.initialize(
            &admin,
            &7u32,
            &"Vault XLM".into_val(&env),
            &"vXLM".into_val(&env),
        );

        // Mint and transfer
        client.mint(&user1, &1000i128);
        client.transfer(&user1, &user2, &300i128);

        assert_eq!(client.balance(&user1), 700i128);
        assert_eq!(client.balance(&user2), 300i128);
    }

    #[test]
    fn test_burn() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VUSDCToken, ());
        let client = VUSDCTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        client.initialize(
            &admin,
            &7u32,
            &"Vault XLM".into_val(&env),
            &"vXLM".into_val(&env),
        );

        // Mint and burn
        client.mint(&user, &1000i128);
        client.burn(&user, &400i128);

        assert_eq!(client.balance(&user), 600i128);
        assert_eq!(client.total_supply(), 600i128);
    }
}
