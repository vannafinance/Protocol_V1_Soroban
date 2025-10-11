fn main() {}

#[cfg(test)]
mod tests;

// #[cfg(test)]
// mod tests {

//     use account_manager_contract::account_manager::AccountManagerContractClient;
//     use account_manager_contract::account_manager::{self, AccountManagerContract};
//     use lending_protocol_xlm::liquidity_pool_xlm::{
//         self, LiquidityPoolXLM, LiquidityPoolXLMClient,
//     };
//     use oracle_contract::oracle_service::{OracleContract, OracleContractClient};
//     use registry_contract::registry::RegistryContract;
//     use registry_contract::registry::RegistryContractClient;
//     use risk_engine_contract::risk_engine::RiskEngineContract;
//     use sep_40_oracle::testutils::{self, Asset, MockPriceOracle, MockPriceOracleClient};
//     use sep_40_oracle::{Asset as MAsset, PriceData, PriceFeedClient, PriceFeedTrait};
//     use smart_account_contract::smart_account::SmartAccountContractClient;
//     use soroban_sdk::token;
//     use soroban_sdk::token::StellarAssetClient;
//     use soroban_sdk::{Address as Addr, String, Symbol, U256};
//     use soroban_sdk::{Env, Vec, testutils::Address};
//     use vxlm_token_contract::v_xlm::VXLMToken;
//     use vxlm_token_contract::v_xlm::VXLMTokenClient;

//     const SMART_ACCOUNT_WASM: &[u8] =
//         include_bytes!("../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm");

//     pub struct ContractAddresses {
//         admin: Addr,
//         liquidity_pool_xlm: Addr,
//         liquidity_pool_usdc: Addr,
//         liquidity_pool_eurc: Addr,
//         registry_contract: Addr,
//         rate_model_contract: Addr,
//         account_manager_contract: Addr,
//         oracle_contract: Addr,
//         risk_engine_contract: Addr,
//         smart_account_contract: Option<Addr>,
//         vxlm_token_contract: Addr,
//         xlm_address: Addr,
//         usdc_address: Addr,
//         eurc_address: Addr,
//     }

//     pub fn test_initiation(env: &Env) -> ContractAddresses {
//         let admin = Addr::generate(&env);
//         let liquidity_pool_xlm_addr = Addr::generate(&env);
//         let liquidity_pool_usdc_addr = Addr::generate(&env);
//         let liquidity_pool_eurc_addr = Addr::generate(&env);

//         let registry_contract_id = Addr::generate(&env);
//         let account_manager_id = Addr::generate(&env);
//         let rate_model_id = Addr::generate(&env);
//         let oracle_contract_id = Addr::generate(&env);
//         let risk_engine_contract_id = Addr::generate(&env);
//         let vxlm_token_contract_id = Addr::generate(&env);
//         let xlm_token = env.register_stellar_asset_contract_v2(admin.clone());
//         let usdc_token = env.register_stellar_asset_contract_v2(admin.clone());
//         let eurc_token = env.register_stellar_asset_contract_v2(admin.clone());

//         ContractAddresses {
//             admin: admin,
//             liquidity_pool_xlm: liquidity_pool_xlm_addr,
//             liquidity_pool_usdc: liquidity_pool_usdc_addr,
//             liquidity_pool_eurc: liquidity_pool_eurc_addr,
//             registry_contract: registry_contract_id,
//             rate_model_contract: rate_model_id,
//             account_manager_contract: account_manager_id,
//             oracle_contract: oracle_contract_id,
//             risk_engine_contract: risk_engine_contract_id,
//             smart_account_contract: None,
//             vxlm_token_contract: vxlm_token_contract_id,
//             xlm_address: xlm_token.address(),
//             usdc_address: usdc_token.address(),
//             eurc_address: eurc_token.address(),
//         }
//     }

//     fn liquidity_pool_lenders_initialise(env: &Env, contracts: &ContractAddresses) {
//         // let xlm_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());

//         env.register_at(
//             &contracts.registry_contract,
//             RegistryContract,
//             (contracts.admin.clone(),),
//         );

//         env.register_at(
//             &contracts.account_manager_contract,
//             AccountManagerContract,
//             (contracts.admin.clone(), contracts.registry_contract.clone()),
//         );

//         env.register_at(
//             &contracts.liquidity_pool_xlm,
//             LiquidityPoolXLM,
//             (
//                 contracts.admin.clone(),
//                 contracts.xlm_address.clone(),
//                 contracts.registry_contract.clone(),
//                 contracts.account_manager_contract.clone(),
//                 contracts.rate_model_contract.clone(),
//                 contracts.admin.clone(),
//             ),
//         );

//         env.register_at(&contracts.vxlm_token_contract, VXLMToken, ());

//         let vxlm_token_contract_client = VXLMTokenClient::new(&env, &contracts.vxlm_token_contract);
//         vxlm_token_contract_client.initialize(
//             &contracts.liquidity_pool_xlm,
//             &6_u32,
//             &String::from_str(&env, "VXLM TOKEN"),
//             &String::from_str(&env, "VXLM"),
//         );

//         let lp_xlm_client = LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);

//         let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
//         registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
//         let lender_addr1 = Addr::generate(&env);
//         let lender_addr2 = Addr::generate(&env);

//         let lender_addr3 = Addr::generate(&env);

//         let lender_addr4 = Addr::generate(&env);

//         let stellar_asset = StellarAssetClient::new(&env, &contracts.xlm_address);

//         stellar_asset.mint(&lender_addr1, &1000000000i128);
//         stellar_asset.mint(&lender_addr2, &1000000000i128);
//         stellar_asset.mint(&lender_addr3, &1000000000i128);
//         stellar_asset.mint(&lender_addr4, &1000000000i128);

//         let amount1 = U256::from_u32(&env, 400);
//         let amount2 = U256::from_u32(&env, 500);
//         let amount3 = U256::from_u32(&env, 600);
//         let amount4 = U256::from_u32(&env, 700);

//         let x = lp_xlm_client.initialize_pool_xlm(&contracts.vxlm_token_contract);
//         // println!("response from : {:?} ", x);

//         lp_xlm_client.deposit_xlm(&lender_addr1, &amount1);
//         lp_xlm_client.deposit_xlm(&lender_addr2, &amount2);
//         lp_xlm_client.deposit_xlm(&lender_addr3, &amount3);
//         lp_xlm_client.deposit_xlm(&lender_addr4, &amount4);

//         let xlm_token_client = token::TokenClient::new(&env, &contracts.xlm_address);
//         println!(
//             "Balance after depositing: {:?}",
//             xlm_token_client.balance(&lender_addr1)
//         );
//         assert!(xlm_token_client.balance(&lender_addr1) == 999999600);
//         assert!(xlm_token_client.balance(&lender_addr2) == 999999500);
//         assert!(xlm_token_client.balance(&lender_addr3) == 999999400);
//         assert!(xlm_token_client.balance(&lender_addr4) == 999999300);
//     }

//     fn oracle_price_feed_setup(env: &Env, contracts: &ContractAddresses) -> Addr {
//         let price_feed_add = Addr::generate(&env);
//         let usdc_symbol = Symbol::new(&env, "USDC");
//         let xlm_symbol = Symbol::new(&env, "XLM");
//         let eurc_symbol = Symbol::new(&env, "EURC");
//         let sol_symbol = Symbol::new(&env, "SOL");

//         let xlm_address = Addr::generate(&env);

//         let wasm_hash = env
//             .deployer()
//             .upload_contract_wasm(testutils::MockPriceOracleWASM);

//         let price_feed_addr = env
//             .deployer()
//             .with_address(
//                 price_feed_add,
//                 AccountManagerContract::generate_predictable_salt(
//                     &env,
//                     contracts.admin.clone(),
//                     contracts.account_manager_contract.clone(),
//                 ),
//             )
//             .deploy_v2(wasm_hash, ());

//         println!("Price feed contract deployed! at {:?}", price_feed_addr);

//         let price_feed_client = MockPriceOracleClient::new(&env, &price_feed_addr);
//         let assets = Vec::from_array(
//             &env,
//             [
//                 Asset::Other(xlm_symbol),
//                 Asset::Other(usdc_symbol.clone()),
//                 Asset::Other(eurc_symbol),
//             ],
//         );
//         price_feed_client.set_data(
//             &contracts.admin,
//             &testutils::Asset::Other(usdc_symbol.clone()),
//             &assets,
//             &7,
//             &3,
//         );
//         price_feed_client.set_price(
//             &Vec::from_array(&env, [4000000, 9990000, 12262415]),
//             &env.ledger().timestamp(),
//         );
//         price_feed_addr
//     }

//     #[test]
//     fn all_integrated_tests_start() {
//         let env = Env::default();
//         env.mock_all_auths();

//         let contracts = test_initiation(&env);

//         let xlm_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());
//         // let vxlm_token =
//         //     env.register_stellar_asset_contract_v2(contracts.liquidity_pool_xlm.clone());

//         println!("vxlm token address is {:?}", contracts.vxlm_token_contract);

//         env.register_at(
//             &contracts.registry_contract,
//             RegistryContract,
//             (contracts.admin.clone(),),
//         );

//         env.register_at(
//             &contracts.account_manager_contract,
//             AccountManagerContract,
//             (contracts.admin.clone(), contracts.registry_contract.clone()),
//         );

//         env.register_at(
//             &contracts.liquidity_pool_xlm,
//             LiquidityPoolXLM,
//             (
//                 contracts.admin.clone(),
//                 xlm_token.address(),
//                 // vxlm_token.address(),
//                 contracts.registry_contract.clone(),
//                 contracts.account_manager_contract.clone(),
//                 contracts.rate_model_contract,
//                 contracts.admin.clone(),
//             ),
//         );

//         let vxlm_token_contract_address =
//             env.register_at(&contracts.vxlm_token_contract, VXLMToken, ());

//         let vxlm_token_contract_client = VXLMTokenClient::new(&env, &vxlm_token_contract_address);
//         vxlm_token_contract_client.initialize(
//             &contracts.liquidity_pool_xlm,
//             &6_u32,
//             &String::from_str(&env, "VXLM TOKEN"),
//             &String::from_str(&env, "VXLM"),
//         );

//         let lp_xlm_client =
//             liquidity_pool_xlm::LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);

//         let lender_addr = Addr::generate(&env);
//         let stellar_asset = StellarAssetClient::new(&env, &xlm_token.address());

//         let registry_client = RegistryContractClient::new(&env, &contracts.registry_contract);
//         registry_client.set_native_xlm_contract_adddress(&xlm_token.address());

//         stellar_asset.mint(&lender_addr, &1000000000i128);

//         let amount = U256::from_u32(&env, 400);
//         let amountx = U256::from_u32(&env, 40);

//         let x = lp_xlm_client.initialize_pool_xlm(&vxlm_token_contract_address);
//         // println!("response from : {:?} ", x);

//         lp_xlm_client.deposit_xlm(&lender_addr, &amount);

//         // println!(
//         //     " VXLM balance after depositing : {:?}",
//         //     vxlm_token_contract_client.balance(&lender_addr)
//         // );

//         lp_xlm_client.redeem_vxlm(&lender_addr, &amountx);

//         // println!(
//         //     " VXLM balance after redeeming : {:?}",
//         //     vxlm_token_contract_client.balance(&lender_addr)
//         // );

//         let registry_contract_client =
//             RegistryContractClient::new(&env, &&contracts.registry_contract.clone());

//         const SMART_ACCOUNT_WASM: &[u8] = include_bytes!(
//             "../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm"
//         );

//         let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

//         registry_contract_client.set_smart_account_hash(&smart_account_hash);

//         let account_manager_client =
//             account_manager_contract::account_manager::AccountManagerContractClient::new(
//                 &env,
//                 &contracts.account_manager_contract,
//             );

//         let trader_address = Addr::generate(&env);

//         let add = account_manager_client.create_account(&trader_address);
//         println!("Created margin account address is  : {:?}", add);

//         account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));
//         stellar_asset.mint(&trader_address, &10000i128);
//     }

//     #[test]
//     fn account_manager_flows() {
//         let env = Env::default();
//         env.mock_all_auths();

//         let contracts = test_initiation(&env);
//         let reflector_address = Addr::generate(&env);
//         let usdc_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());
//         let xlm_token = env.register_stellar_asset_contract_v2(contracts.admin.clone());

//         env.register_at(
//             &contracts.registry_contract,
//             RegistryContract,
//             (contracts.admin.clone(),),
//         );

//         env.register_at(
//             &contracts.account_manager_contract,
//             AccountManagerContract,
//             (contracts.admin.clone(), contracts.registry_contract.clone()),
//         );

//         env.register_at(
//             &contracts.risk_engine_contract,
//             RiskEngineContract,
//             (contracts.admin.clone(), contracts.registry_contract.clone()),
//         );

//         env.register_at(
//             &contracts.oracle_contract,
//             OracleContract,
//             (contracts.admin.clone(), reflector_address),
//         );

//         let account_manager_client =
//             account_manager_contract::account_manager::AccountManagerContractClient::new(
//                 &env,
//                 &contracts.account_manager_contract,
//             );

//         const SMART_ACCOUNT_WASM: &[u8] = include_bytes!(
//             "../../target/wasm32v1-none/release-with-logs/smart_account_contract.wasm"
//         );

//         let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

//         let registry_contract_client =
//             RegistryContractClient::new(&env, &contracts.registry_contract.clone());
//         registry_contract_client.set_smart_account_hash(&smart_account_hash);
//         registry_contract_client.set_native_usdc_contract_address(&usdc_token.address());
//         registry_contract_client.set_native_xlm_contract_adddress(&xlm_token.address());
//         registry_contract_client.set_risk_engine_address(&contracts.risk_engine_contract);
//         registry_contract_client.set_oracle_contract_address(&contracts.oracle_contract);

//         let stellar_asset_usdc = StellarAssetClient::new(&env, &usdc_token.address());
//         let stellar_asset_xlm = StellarAssetClient::new(&env, &xlm_token.address());

//         let trader_address = Addr::generate(&env);
//         let trader_address2 = Addr::generate(&env);

//         println!("TRader address 1 {:?}", trader_address);
//         println!("Trader address 2 {:?}", trader_address2);

//         let margin_acc1 = account_manager_client.create_account(&trader_address);

//         let margin_acc2 = account_manager_client.create_account(&trader_address2);

//         println!("CReated margin account addres is {:?}", margin_acc1);
//         println!("CReated margin account addres is {:?}", margin_acc2);
//         account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));

//         assert!(
//             account_manager_client
//                 .get_max_asset_cap()
//                 .eq(&U256::from_u32(&env, 10))
//         );

//         stellar_asset_xlm.mint(&trader_address, &10000i128);
//         stellar_asset_xlm.mint(&trader_address2, &10000i128);

//         println!(
//             "Account manager address is {:?}",
//             &contracts.account_manager_contract
//         );

//         let margin_client1 = smart_account_contract::smart_account::SmartAccountContractClient::new(
//             &env,
//             &margin_acc1,
//         );

//         let margin_client2 = smart_account_contract::smart_account::SmartAccountContractClient::new(
//             &env,
//             &margin_acc2,
//         );

//         account_manager_client.set_iscollateral_allowed(&Symbol::new(&env, "XLM"));

//         let collateral_balx =
//             margin_client1.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
//         println!("COllateral balance before XLM is {:?}", collateral_balx);

//         account_manager_client.deposit_collateral_tokens(
//             &margin_acc1,
//             &Symbol::new(&env, "XLM"),
//             &U256::from_u128(&env, 100),
//         );

//         account_manager_client.deposit_collateral_tokens(
//             &margin_acc2,
//             &Symbol::new(&env, "XLM"),
//             &U256::from_u128(&env, 999),
//         );

//         let collateral_bal = margin_client1.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
//         println!("COllateral balance after XLM is {:?}", collateral_bal);

//         let collateral_bal = margin_client2.get_collateral_token_balance(&Symbol::new(&env, "XLM"));
//         println!(
//             "COllateral balance of Trader 2 after XLM is {:?}",
//             collateral_bal
//         );

//         account_manager_client.withdraw_collateral_balance(
//             &margin_acc1,
//             &Symbol::new(&env, "XLM"),
//             &U256::from_u128(&env, 80),
//         );

//         // account_manager_client.borrow(
//         //     &trader_address,
//         //     &U256::from_u32(&env, 1000),
//         //     &Symbol::new(&env, "USDC"),
//         // );

//         // account_manager_client.approve();
//     }

//     #[test]
//     fn test_oracle_price() {
//         let env = Env::default();
//         env.mock_all_auths();
//         let contracts = test_initiation(&env);

//         let price_feed_add = Addr::generate(&env);
//         let usdc_symbol = Symbol::new(&env, "USDC");
//         let xlm_symbol = Symbol::new(&env, "XLM");
//         let eurc_symbol = Symbol::new(&env, "EURC");
//         let sol_symbol = Symbol::new(&env, "SOL");

//         let wasm_hash = env
//             .deployer()
//             .upload_contract_wasm(testutils::MockPriceOracleWASM);

//         let price_feed_addr = env
//             .deployer()
//             .with_address(
//                 price_feed_add,
//                 AccountManagerContract::generate_predictable_salt(
//                     &env,
//                     contracts.admin.clone(),
//                     contracts.account_manager_contract.clone(),
//                 ),
//             )
//             .deploy_v2(wasm_hash, ());

//         let price_feed_client = MockPriceOracleClient::new(&env, &price_feed_addr);
//         let assets = Vec::from_array(
//             &env,
//             [
//                 Asset::Other(xlm_symbol),
//                 Asset::Other(usdc_symbol.clone()),
//                 Asset::Other(eurc_symbol),
//             ],
//         );
//         price_feed_client.set_data(
//             &contracts.admin,
//             &testutils::Asset::Other(usdc_symbol.clone()),
//             &assets,
//             &2,
//             &6,
//         );
//         price_feed_client.set_price(
//             &Vec::from_array(&env, [1000000, 28437629, 3000000]),
//             &9988229,
//         );
//         let recent = price_feed_client.lastprice(&testutils::Asset::Other(usdc_symbol.clone()));
//         println!("Recent price {:?}", recent.unwrap().price);

//         // Check if oracle test mode is fetching the same data added into the price feed
//         let oracle_address = env.register_at(
//             &contracts.oracle_contract,
//             OracleContract,
//             (contracts.admin.clone(), price_feed_addr),
//         );

//         let oracle_client = OracleContractClient::new(&env, &oracle_address);

//         let (price, decimals) = oracle_client.get_price_latest(&Symbol::new(&env, "USDC"));
//         println!("Oracle price : {:?}", price);
//     }

//     #[test]
//     fn test_trader_borrow_logic() {
//         let env = Env::default();
//         env.mock_all_auths();
//         let contracts = test_initiation(&env);

//         liquidity_pool_lenders_initialise(&env, &contracts);

//         let price_feed_addr = oracle_price_feed_setup(&env, &contracts);

//         env.register_at(
//             &contracts.risk_engine_contract,
//             RiskEngineContract,
//             (contracts.admin.clone(), contracts.registry_contract.clone()),
//         );

//         env.register_at(
//             &contracts.oracle_contract,
//             OracleContract,
//             (contracts.admin.clone(), price_feed_addr),
//         );

//         let account_manager_client =
//             AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

//         let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

//         let registry_client =
//             RegistryContractClient::new(&env, &contracts.registry_contract.clone());
//         registry_client.set_smart_account_hash(&smart_account_hash);
//         registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
//         registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
//         registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
//         registry_client.set_oracle_contract_address(&contracts.oracle_contract);
//         registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
//         registry_client.set_rate_model_address(&contracts.rate_model_contract);

//         let stellar_asset_usdc = StellarAssetClient::new(&env, &contracts.usdc_address);
//         let stellar_asset_xlm = StellarAssetClient::new(&env, &contracts.xlm_address);

//         let trader_address = Addr::generate(&env);
//         println!("Trader address 1 {:?}", trader_address);

//         let margin_acc1 = account_manager_client.create_account(&trader_address);
//         println!("Created margin account addres is {:?}", margin_acc1);
//         account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));

//         assert!(
//             account_manager_client
//                 .get_max_asset_cap()
//                 .eq(&U256::from_u32(&env, 10))
//         );

//         stellar_asset_usdc.mint(&trader_address, &10000i128);
//         let usdc_symbol = Symbol::new(&env, "USDC");
//         let xlm_symbol = Symbol::new(&env, "XLM");

//         let margin_client1 = SmartAccountContractClient::new(&env, &margin_acc1);
//         account_manager_client.set_iscollateral_allowed(&usdc_symbol);

//         let collateral_balx = margin_client1.get_collateral_token_balance(&usdc_symbol);
//         assert!(collateral_balx.eq(&U256::from_u128(&env, 0)));

//         account_manager_client.deposit_collateral_tokens(
//             &margin_acc1,
//             &usdc_symbol,
//             &U256::from_u128(&env, 100),
//         );

//         let collateral_bal = margin_client1.get_collateral_token_balance(&usdc_symbol);
//         assert!(collateral_bal.eq(&U256::from_u128(&env, 100)));

//         account_manager_client.withdraw_collateral_balance(
//             &margin_acc1,
//             &usdc_symbol,
//             &U256::from_u128(&env, 10),
//         );

//         let collateral_balxy = margin_client1.get_collateral_token_balance(&usdc_symbol);
//         assert!(collateral_balxy.eq(&U256::from_u128(&env, 90)));

//         account_manager_client.borrow(&margin_acc1, &U256::from_u128(&env, 10), &xlm_symbol);
//         let borrowd_xlm = margin_client1.get_borrowed_token_debt(&xlm_symbol);
//         assert!(borrowd_xlm.eq(&U256::from_u128(&env, 10)));

//         let xlm = token::Client::new(&env, &contracts.xlm_address);
//         println!("Balance before repay{:?}", xlm.balance(&margin_acc1));

//         account_manager_client.repay(&U256::from_u128(&env, 8), &xlm_symbol, &margin_acc1);
//         assert!(xlm.balance(&margin_acc1).eq(&2));
//     }

//     #[test]
//     // #[should_panic(expected = "assertion failed")]
//     fn test_trader_borrow_failures() {
//         let env = Env::default();
//         env.mock_all_auths();
//         let contracts = test_initiation(&env);

//         liquidity_pool_lenders_initialise(&env, &contracts);

//         let price_feed_addr = oracle_price_feed_setup(&env, &contracts);

//         env.register_at(
//             &contracts.risk_engine_contract,
//             RiskEngineContract,
//             (contracts.admin.clone(), contracts.registry_contract.clone()),
//         );

//         env.register_at(
//             &contracts.oracle_contract,
//             OracleContract,
//             (contracts.admin.clone(), price_feed_addr),
//         );

//         let account_manager_client =
//             AccountManagerContractClient::new(&env, &contracts.account_manager_contract);

//         let smart_account_hash = env.deployer().upload_contract_wasm(SMART_ACCOUNT_WASM);

//         let registry_client =
//             RegistryContractClient::new(&env, &contracts.registry_contract.clone());
//         registry_client.set_smart_account_hash(&smart_account_hash);
//         registry_client.set_native_usdc_contract_address(&contracts.usdc_address);
//         registry_client.set_native_xlm_contract_adddress(&contracts.xlm_address);
//         registry_client.set_risk_engine_address(&contracts.risk_engine_contract);
//         registry_client.set_oracle_contract_address(&contracts.oracle_contract);
//         registry_client.set_lendingpool_xlm(&contracts.liquidity_pool_xlm);
//         registry_client.set_rate_model_address(&contracts.rate_model_contract);

//         let stellar_asset_usdc = StellarAssetClient::new(&env, &contracts.usdc_address);
//         let stellar_asset_xlm = StellarAssetClient::new(&env, &contracts.xlm_address);

//         let trader_address = Addr::generate(&env);
//         let margin_acc1 = account_manager_client.create_account(&trader_address);

//         account_manager_client.set_max_asset_cap(&U256::from_u32(&env, 10));

//         stellar_asset_usdc.mint(&trader_address, &10000i128);
//         let usdc_symbol = Symbol::new(&env, "USDC");
//         let xlm_symbol = Symbol::new(&env, "XLM");

//         let margin_client1 = SmartAccountContractClient::new(&env, &margin_acc1);
//         account_manager_client.set_iscollateral_allowed(&usdc_symbol);

//         margin_client1.get_collateral_token_balance(&usdc_symbol);

//         account_manager_client.deposit_collateral_tokens(
//             &margin_acc1,
//             &usdc_symbol,
//             &U256::from_u128(&env, 100),
//         );

//         margin_client1.get_collateral_token_balance(&usdc_symbol);

//         account_manager_client.withdraw_collateral_balance(
//             &margin_acc1,
//             &usdc_symbol,
//             &U256::from_u128(&env, 90),
//         );

//         let lp_xlm_client = LiquidityPoolXLMClient::new(&env, &contracts.liquidity_pool_xlm);
//         let pool_borrows = lp_xlm_client.get_borrows();
//         println!("XLM Pool borrows before {:?} ", pool_borrows);

//         account_manager_client.borrow(&margin_acc1, &U256::from_u128(&env, 10), &xlm_symbol);
//         let borrowd_xlm = margin_client1.get_borrowed_token_debt(&xlm_symbol);
//         let borrows_after = lp_xlm_client.get_borrows();

//         println!("XLM Pool borrows after {:?} ", borrows_after);

//         assert!(borrowd_xlm.eq(&U256::from_u128(&env, 10)));
//     }

//     //! =======================
// //! Vanna Protocol – Account Manager Comprehensive Test Suite
// //! =======================
// //! This file includes integration + edge case tests for AccountManagerContract
// //! Dependencies: registry_contract, smart_account_contract, risk_engine_contract, liquidity pools, oracle, etc.

// #![cfg(test)]

// use account_manager_contract::account_manager::{
//     AccountManagerContract, AccountManagerContractClient,
// };
// use lending_protocol_xlm::liquidity_pool_xlm::{LiquidityPoolXLM, LiquidityPoolXLMClient};
// use oracle_contract::oracle_service::{OracleContract, OracleContractClient};
// use registry_contract::registry::{RegistryContract, RegistryContractClient};
// use risk_engine_contract::risk_engine::RiskEngineContract;
// use sep_40_oracle::testutils::{Asset, MockPriceOracle, MockPriceOracleClient};
// use sep_40_oracle::{PriceFeedClient, PriceFeedTrait};
// use smart_account_contract::smart_account::SmartAccountContractClient;
// use soroban_sdk::{
//     testutils::Address as _, token, token::StellarAssetClient, Address, Env, String, Symbol, U256,
//     Vec,
// };
// use vxlm_token_contract::v_xlm::{VXLMToken, VXLMTokenClient};

// /// Struct to hold all deployed contract addresses during integration testing
// pub struct ContractAddresses {
//     pub admin: Address,
//     pub registry_contract: Address,
//     pub account_manager_contract: Address,
//     pub risk_engine_contract: Address,
//     pub oracle_contract: Address,
//     pub liquidity_pool_xlm: Address,
//     pub rate_model_contract: Address,
//     pub vxlm_token_contract: Address,
//     pub xlm_address: Address,
//     pub usdc_address: Address,
// }

// /// ===============================
// /// Test Setup Helpers
// /// ===============================
// fn setup_env() -> (Env, ContractAddresses) {
//     let env = Env::default();
//     env.mock_all_auths();

//     let admin = Address::generate(&env);
//     let registry = Address::generate(&env);
//     let acc_mgr = Address::generate(&env);
//     let risk = Address::generate(&env);
//     let oracle = Address::generate(&env);
//     let lp = Address::generate(&env);
//     let rate_model = Address::generate(&env);
//     let vxlm = Address::generate(&env);

//     let xlm = env.register_stellar_asset_contract_v2(admin.clone());
//     let usdc = env.register_stellar_asset_contract_v2(admin.clone());

//     (
//         env.clone(),
//         ContractAddresses {
//             admin,
//             registry_contract: registry,
//             account_manager_contract: acc_mgr,
//             risk_engine_contract: risk,
//             oracle_contract: oracle,
//             liquidity_pool_xlm: lp,
//             rate_model_contract: rate_model,
//             vxlm_token_contract: vxlm,
//             xlm_address: xlm.address(),
//             usdc_address: usdc.address(),
//         },
//     )
// }

// /// Deploy & initialize all base contracts
// fn init_registry_and_manager(env: &Env, c: &ContractAddresses) {
//     env.register_at(&c.registry_contract, RegistryContract, (c.admin.clone(),));
//     env.register_at(
//         &c.account_manager_contract,
//         AccountManagerContract,
//         (c.admin.clone(), c.registry_contract.clone()),
//     );
//     let registry_client = RegistryContractClient::new(&env, &c.registry_contract);
//     registry_client.set_native_xlm_contract_adddress(&c.xlm_address);
//     registry_client.set_native_usdc_contract_address(&c.usdc_address);
// }

// /// ===============================
// /// Core Functional Tests
// /// ===============================
// #[test]
// fn test_account_creation_and_deletion_flow() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);

//     let account_manager = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let trader = Address::generate(&env);

//     // Create account
//     let smart_acc = account_manager.create_account(&trader);
//     assert!(smart_acc != trader);

//     // Duplicate account creation should not redeploy
//     let smart_acc2 = account_manager.create_account(&trader);
//     assert_eq!(smart_acc, smart_acc2);

//     // Delete account (mock smart account as debt-free)
//     let smart_client = SmartAccountContractClient::new(&env, &smart_acc);
//     // NOTE: smart_account_contract mocked to not have debt for now
//     let result = account_manager.delete_account(&smart_acc);
//     assert!(result.is_ok());
// }

// #[test]
// #[should_panic(expected = "Failed to get registry contract key!")]
// fn test_account_creation_without_registry_should_fail() {
//     let (env, c) = setup_env();
//     // no registry set
//     env.register_at(
//         &c.account_manager_contract,
//         AccountManagerContract,
//         (c.admin.clone(), c.registry_contract.clone()),
//     );
//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let trader = Address::generate(&env);
//     am_client.create_account(&trader);
// }

// #[test]
// fn test_deposit_and_withdraw_collateral_success() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);

//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let registry = RegistryContractClient::new(&env, &c.registry_contract);

//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);
//     am_client.set_max_asset_cap(&U256::from_u32(&env, 10));
//     am_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));

//     // Mint some tokens to trader
//     let usdc_token = StellarAssetClient::new(&env, &c.usdc_address);
//     usdc_token.mint(&trader, &10000i128);

//     // Deposit
//     am_client
//         .deposit_collateral_tokens(&smart_acc, &Symbol::new(&env, "USDC"), &U256::from_u128(&env, 100))
//         .unwrap();

//     let margin_client = SmartAccountContractClient::new(&env, &smart_acc);
//     let bal = margin_client.get_collateral_token_balance(&Symbol::new(&env, "USDC"));
//     assert_eq!(bal, U256::from_u128(&env, 100));

//     // Withdraw
//     am_client
//         .withdraw_collateral_balance(&smart_acc, &Symbol::new(&env, "USDC"), &U256::from_u128(&env, 50))
//         .unwrap();

//     let bal_after = margin_client.get_collateral_token_balance(&Symbol::new(&env, "USDC"));
//     assert_eq!(bal_after, U256::from_u128(&env, 50));
// }

// #[test]
// #[should_panic(expected = "Collateral not allowed for this token symbol")]
// fn test_deposit_disallowed_token_should_panic() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);

//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);
//     am_client.set_max_asset_cap(&U256::from_u32(&env, 10));

//     let xlm_token = StellarAssetClient::new(&env, &c.xlm_address);
//     xlm_token.mint(&trader, &1000i128);

//     // Not calling set_iscollateral_allowed() here
//     am_client
//         .deposit_collateral_tokens(&smart_acc, &Symbol::new(&env, "XLM"), &U256::from_u128(&env, 100))
//         .unwrap();
// }

// #[test]
// #[should_panic(expected = "User doesn't have collateral in this token")]
// fn test_withdraw_without_collateral_should_fail() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);
//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);
//     am_client.withdraw_collateral_balance(&smart_acc, &Symbol::new(&env, "USDC"), &U256::from_u128(&env, 10)).unwrap();
// }

// /// ===============================
// /// Borrow & Repay Tests
// /// ===============================
// #[test]
// fn test_successful_borrow_and_repay_flow() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);

//     env.register_at(
//         &c.risk_engine_contract,
//         RiskEngineContract,
//         (c.admin.clone(), c.registry_contract.clone()),
//     );

//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let registry = RegistryContractClient::new(&env, &c.registry_contract);
//     registry.set_risk_engine_address(&c.risk_engine_contract);

//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);
//     am_client.set_max_asset_cap(&U256::from_u32(&env, 10));
//     am_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));

//     let usdc_token = StellarAssetClient::new(&env, &c.usdc_address);
//     usdc_token.mint(&trader, &10000i128);

//     // Deposit collateral first
//     am_client.deposit_collateral_tokens(&smart_acc, &Symbol::new(&env, "USDC"), &U256::from_u128(&env, 100)).unwrap();

//     // Borrow (mocked)
//     am_client.borrow(&smart_acc, &U256::from_u128(&env, 20), &Symbol::new(&env, "XLM")).unwrap();

//     // Repay
//     am_client.repay(&U256::from_u128(&env, 10), &Symbol::new(&env, "XLM"), &smart_acc).unwrap();
// }

// #[test]
// #[should_panic(expected = "Borrowing is not allowed for this user")]
// fn test_borrow_disallowed_by_risk_engine() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);

//     env.register_at(
//         &c.risk_engine_contract,
//         RiskEngineContract,
//         (c.admin.clone(), c.registry_contract.clone()),
//     );

//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let registry = RegistryContractClient::new(&env, &c.registry_contract);
//     registry.set_risk_engine_address(&c.risk_engine_contract);

//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);

//     am_client.borrow(&smart_acc, &U256::from_u128(&env, 10), &Symbol::new(&env, "USDC")).unwrap();
// }

// /// ===============================
// /// Liquidation & Settlement Tests
// /// ===============================
// #[test]
// #[should_panic(expected = "Cannot liquidate when account is healthy!!")]
// fn test_liquidation_on_healthy_account_should_fail() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);
//     env.register_at(
//         &c.risk_engine_contract,
//         RiskEngineContract,
//         (c.admin.clone(), c.registry_contract.clone()),
//     );
//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);
//     am_client.liquidate(&smart_acc).unwrap();
// }

// #[test]
// fn test_settle_account_flow() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);

//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     let trader = Address::generate(&env);
//     let smart_acc = am_client.create_account(&trader);
//     // Should not panic
//     let result = am_client.settle_account(&smart_acc);
//     assert!(result.is_ok());
// }

// /// ===============================
// /// Admin Logic & Parameter Management
// /// ===============================
// #[test]
// fn test_set_and_get_asset_cap() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);
//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     am_client.set_max_asset_cap(&U256::from_u32(&env, 15));
//     let cap = am_client.get_max_asset_cap();
//     assert_eq!(cap, U256::from_u32(&env, 15));
// }

// #[test]
// fn test_allow_and_check_collateral_token() {
//     let (env, c) = setup_env();
//     init_registry_and_manager(&env, &c);
//     let am_client = AccountManagerContractClient::new(&env, &c.account_manager_contract);
//     am_client.set_iscollateral_allowed(&Symbol::new(&env, "USDC"));
//     let allowed = am_client.get_iscollateral_allowed(&Symbol::new(&env, "USDC"));
//     assert!(allowed);
// }

// #[test]
// fn test_salt_generation_is_deterministic() {
//     let (env, c) = setup_env();
//     let trader = Address::generate(&env);
//     let salt1 =
//         AccountManagerContract::generate_predictable_salt(&env, trader.clone(), c.account_manager_contract.clone());
//     let salt2 =
//         AccountManagerContract::generate_predictable_salt(&env, trader.clone(), c.account_manager_contract.clone());
//     assert_eq!(salt1, salt2);
// }

// }
