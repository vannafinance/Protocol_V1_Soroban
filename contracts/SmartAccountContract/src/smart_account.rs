use core::panic;

use soroban_sdk::{
    Address, Env, Symbol, U256, Vec, contract, contractimpl, log, panic_with_error, symbol_short,
    token,
};

use crate::types::{
    SmartAccExternalAction, SmartAccountActivationEvent, SmartAccountDataKey,
    SmartAccountDeactivationEvent, SmartAccountError,
};

use blend_contract_sdk::pool::{self, Positions, Reserve};
use blend_contract_sdk::pool::{PoolConfig, ReserveConfig, ReserveData};

use blend_contract_sdk::pool::{Client as BlendPoolClient, Request};

// Aquarius liquidity pool router imports
use soroban_sdk::BytesN;

const TLL_LEDGERS_YEAR: u32 = 6307200;
const TLL_LEDGERS_10YEAR: u32 = 6307200 * 10;
const WAD_U128: u128 = 10000_0000_00000_00000; // 10^18 for decimals
const XLM_SYMBOL: Symbol = symbol_short!("XLM");
const USDC_SYMBOL: Symbol = symbol_short!("USDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");

// Aquarius pool pair symbol (for XLM-USDC LP tracking)
const AQUARIUS_XLM_USDC_SYMBOL: Symbol = symbol_short!("AQ_XLM_U");

#[contract]
pub struct SmartAccountContract;

#[contractimpl]
impl SmartAccountContract {
    pub fn __constructor(
        env: Env,
        account_manager: Address,
        registry_contract: Address,
        user_address: Address,
    ) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::AccountManager, &account_manager);

        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::RegistryContract, &registry_contract);

        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::OwnerAddress, &user_address);

        let key = SmartAccountDataKey::IsAccountActive;
        // When deployed the smart account is inactive, which should be activated explicitly
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_smart_account(&env, key);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::AccountManager);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::RegistryContract);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::OwnerAddress);
    }

    pub fn deactivate_account(env: &Env) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &false);
        Self::extend_ttl_smart_account(&env, key);
        env.events().publish(
            (Symbol::new(&env, "Smart_Account_Deactivated"),),
            SmartAccountDeactivationEvent {
                margin_account: env.current_contract_address(),
                deactivate_time: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn activate_account(env: &Env) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();
        let key = SmartAccountDataKey::IsAccountActive;
        env.storage().persistent().set(&key, &true);
        Self::extend_ttl_smart_account(&env, key);
        env.events().publish(
            (Symbol::new(&env, "Smart_Account_Activated"),),
            SmartAccountActivationEvent {
                margin_account: env.current_contract_address(),
                activated_time: env.ledger().timestamp(),
            },
        );
        Ok(())
    }

    pub fn remove_borrowed_token_balance(
        env: Env,
        token_symbol: Symbol,
        amount_wad: u128,
    ) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let this_account = env.current_contract_address();

        if token_symbol == XLM_SYMBOL {
            let pool_xlm_address = registry_client.get_lendingpool_xlm();
            let native_xlm_address = registry_client.get_xlm_contract_adddress();
            let xlm_token = token::Client::new(&env, &native_xlm_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, xlm_token.decimals());
            xlm_token.transfer(&this_account, &pool_xlm_address, &amount_scaled);
        } else if token_symbol == USDC_SYMBOL {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            let native_usdc_address = registry_client.get_usdc_contract_address();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &pool_usdc_address, &amount_scaled);
        } else if token_symbol == EURC_SYMBOL {
            let pool_eurc_address = registry_client.get_lendingpool_eurc();
            let native_eurc_address = registry_client.get_eurc_contract_address();
            let eurc_token = token::Client::new(&env, &native_eurc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, eurc_token.decimals());
            eurc_token.transfer(&this_account, &pool_eurc_address, &amount_scaled);
        }
        Ok(())
    }

    pub fn remove_collateral_token_balance(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
        amount_wad: u128,
    ) -> Result<(), SmartAccountError> {
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();
        Self::remove_collateral_token_bal_internal(env, user_address, token_symbol, amount_wad)
    }

    fn remove_collateral_token_bal_internal(
        env: &Env,
        user_address: Address,
        token_symbol: Symbol,
        amount_wad: u128,
    ) -> Result<(), SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let this_account = env.current_contract_address();

        if token_symbol == XLM_SYMBOL {
            let native_xlm_address = registry_client.get_xlm_contract_adddress();
            let xlm_token = token::Client::new(&env, &native_xlm_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, xlm_token.decimals());
            let bal_before = xlm_token.balance(&this_account);
            xlm_token.transfer(&this_account, &user_address, &amount_scaled);
            let bal_after = xlm_token.balance(&this_account);
            log!(
                &env,
                "Transfering xlm ",
                amount_scaled,
                bal_before,
                bal_after
            );
        } else if token_symbol == USDC_SYMBOL {
            let native_usdc_address = registry_client.get_usdc_contract_address();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &user_address, &amount_scaled);
        } else if token_symbol == EURC_SYMBOL {
            let native_eurc_address = registry_client.get_eurc_contract_address();
            let eurc_token = token::Client::new(&env, &native_eurc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, eurc_token.decimals());
            eurc_token.transfer(&this_account, &user_address, &amount_scaled);
        }

        let collateral_balance_wad = Self::get_collateral_token_balance(&env, token_symbol.clone());
        let balance_after_deduction_wad =
            collateral_balance_wad.sub(&U256::from_u128(&env, amount_wad));

        Self::set_collateral_token_bal_internal(
            env,
            token_symbol.clone(),
            balance_after_deduction_wad.clone(),
        );

        if balance_after_deduction_wad == U256::from_u128(&env, 0) {
            Self::remove_collateral_token(&env, token_symbol.clone()).unwrap();
        }

        Ok(())
    }

    pub fn sweep_to(env: &Env, to_address: Address) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let all_collateral_tokens = Self::get_all_collateral_tokens(env);
        for coltoken in all_collateral_tokens.iter() {
            let coltokenbalance = Self::get_collateral_token_balance(env, coltoken.clone());

            let col_token_amount = coltokenbalance.to_u128().unwrap_or_else(|| {
                panic_with_error!(&env, SmartAccountError::IntegerConversionError)
            });

            Self::remove_collateral_token_bal_internal(
                env,
                to_address.clone(),
                coltoken,
                col_token_amount,
            )
            .expect("Failed to remove collateral token balance");
        }
        Ok(())
    }

    pub fn has_debt(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&SmartAccountDataKey::HasDebt)
            .unwrap_or_else(|| false)
    }

    pub fn set_has_debt(env: &Env, has_debt: bool) {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        Self::set_has_debt_internal(env, has_debt);
    }

    fn set_has_debt_internal(env: &Env, has_debt: bool) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::HasDebt, &has_debt);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::HasDebt);
    }

    pub fn get_all_borrowed_tokens(env: &Env) -> Vec<Symbol> {
        let borrowed_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::BorrowedTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        borrowed_tokens_list
    }

    pub fn add_borrowed_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();
        let mut borrowed_tokens_list: Vec<Symbol> = Self::get_all_borrowed_tokens(env);
        if !borrowed_tokens_list.contains(&token_symbol.clone()) {
            borrowed_tokens_list.push_back(token_symbol);
        }
        Self::set_borrowed_token_list(env, borrowed_tokens_list);
        Ok(())
    }

    pub fn remove_borrowed_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let mut borrowed_tokens_list: Vec<Symbol> = Self::get_all_borrowed_tokens(env);
        if borrowed_tokens_list.contains(&token_symbol.clone()) {
            let index = borrowed_tokens_list
                .first_index_of(token_symbol.clone())
                .unwrap();
            borrowed_tokens_list.remove(index);
        }

        if borrowed_tokens_list.is_empty() {
            Self::set_has_debt_internal(&env, false);
        }
        Self::set_borrowed_token_list(env, borrowed_tokens_list);
        Ok(())
    }

    pub fn execute(
        env: &Env,
        target_protocol: Address,
        action: SmartAccExternalAction,
        trader_address: Address,
        tokens: Vec<Symbol>,
        tokens_amount_wad: Vec<u128>,
    ) -> Result<(bool, i128), SmartAccountError> {
        let account_manager: Address = Self::get_account_manager(&env);
        account_manager.require_auth();

        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(env, &registry_address);
        let smart_account = env.current_contract_address();

        // Determine which protocol this is
        // First check if it's an Aquarius-only action (AddLiquidity or RemoveLiquidity)
        let is_aquarius_action = matches!(action, SmartAccExternalAction::AddLiquidity | SmartAccExternalAction::RemoveLiquidity);
        
        if is_aquarius_action {
            return Self::execute_aquarius(
                env,
                &registry_client,
                action,
                &trader_address,
                &smart_account,
                tokens,
                tokens_amount_wad,
            );
        }

        // For shared actions (Deposit, Withdraw, Swap), check target_protocol address
        // to determine which protocol to route to
        // We'll check Aquarius first (supports more actions), then Blend
        
        // Check if Aquarius router is configured
        if registry_client.has_aquarius_router_address() {
            let aquarius_router_address = registry_client.get_aquarius_router_address();
            if target_protocol == aquarius_router_address {
                return Self::execute_aquarius(
                    env,
                    &registry_client,
                    action,
                    &trader_address,
                    &smart_account,
                    tokens,
                    tokens_amount_wad,
                );
            }
        }
        
        // Check if Blend pool is configured
        if registry_client.has_blend_pool_address() {
            let blend_pool_address = registry_client.get_blend_pool_address();
            if target_protocol == blend_pool_address {
                // Handle Blend protocol operations
                let mut request_type: u32 = 0;

                match action {
                    SmartAccExternalAction::Deposit => request_type = 0,
                    SmartAccExternalAction::Withdraw => request_type = 1,
                    SmartAccExternalAction::Swap => request_type = 10,
                    _ => panic!("Invalid action for Blend protocol"),
                }
                
                let blend_pool_client = BlendPoolClient::new(env, &blend_pool_address);

            for (token, amt_wad) in tokens.iter().zip(tokens_amount_wad) {
                log!(&env, "Token symbol passed: {}", token);
                if token == XLM_SYMBOL {
                    let native_xlm_address = registry_client.get_xlm_contract_adddress();
                    let xlm_token = token::Client::new(&env, &native_xlm_address);
                    let amt = Self::scale_from_wad(amt_wad, xlm_token.decimals());
                    let resv: Reserve = blend_pool_client.get_reserve(&native_xlm_address);
                    let pool_config: pool::PoolConfig = blend_pool_client.get_config();
                    blend_pool_client.get_config().oracle;
                    let positions_before = blend_pool_client.get_positions(&smart_account);
                    let b_tokens_before =
                        positions_before.supply.get(resv.config.index).unwrap_or(0);

                    let b_rate = resv.data.b_rate;
                    let request = Request {
                        address: native_xlm_address,
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &resv.asset,
                        &requests,
                    );
                    if request_type == 0 {
                        log!(&env, "Blend Pool Deposit b_rate {}, amount {}", b_rate, amt);
                        let b_tokens_minted =
                            positions.supply.get_unchecked(resv.config.index) - b_tokens_before;
                        return Ok((true, b_tokens_minted));
                    } else if request_type == 1 {
                        let b_tokens_burned =
                            b_tokens_before - positions.supply.get_unchecked(resv.config.index);
                        return Ok((true, -b_tokens_burned));
                    } else {
                        panic!("Unsupported request type for XLM in Blend Pool");
                    }
                } else if token == USDC_SYMBOL {
                    let usdc_contract_address = registry_client.get_usdc_contract_address();
                    let usdc_token = token::Client::new(&env, &usdc_contract_address);
                    let amt = Self::scale_from_wad(amt_wad, usdc_token.decimals());
                    let resv = blend_pool_client.get_reserve(&usdc_contract_address);
                    let b_rate = resv.data.b_rate;
                    let positions_before = blend_pool_client.get_positions(&smart_account);
                    let b_tokens_before =
                        positions_before.supply.get(resv.config.index).unwrap_or(0);

                    let request = Request {
                        address: usdc_contract_address,
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &resv.asset,
                        &requests,
                    );
                    if request_type == 0 {
                        log!(&env, "Blend Pool Deposit b_rate {}, amount {}", b_rate, amt);
                        let b_tokens_minted =
                            positions.supply.get_unchecked(resv.config.index) - b_tokens_before;
                        return Ok((true, b_tokens_minted));
                    } else if request_type == 1 {
                        let b_tokens_burned =
                            b_tokens_before - positions.supply.get_unchecked(resv.config.index);
                        return Ok((true, -b_tokens_burned));
                    } else {
                        panic!("Unsupported request type for XLM in Blend Pool");
                    }
                } else if token == EURC_SYMBOL {
                    let eurc_contract_address = registry_client.get_eurc_contract_address();
                    let eurc_token = token::Client::new(&env, &eurc_contract_address);
                    let amt = Self::scale_from_wad(amt_wad, eurc_token.decimals());
                    let resv = blend_pool_client.get_reserve(&eurc_contract_address);
                    let b_rate = resv.data.b_rate;
                    let positions_before = blend_pool_client.get_positions(&smart_account);
                    let b_tokens_before =
                        positions_before.supply.get(resv.config.index).unwrap_or(0);

                    let request = Request {
                        address: eurc_contract_address,
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &resv.asset,
                        &requests,
                    );
                    if request_type == 0 {
                        log!(&env, "Blend Pool Deposit b_rate {}, amount {}", b_rate, amt);
                        let b_tokens_minted =
                            positions.supply.get_unchecked(resv.config.index) - b_tokens_before;
                        return Ok((true, b_tokens_minted));
                    } else if request_type == 1 {
                        let b_tokens_burned =
                            b_tokens_before - positions.supply.get_unchecked(resv.config.index);
                        return Ok((true, -b_tokens_burned));
                    } else {
                        panic!("Unsupported request type for XLM in Blend Pool");
                    }
                } else {
                    panic!("No external protocol mapped for the given token symbol");
                }
            }
                return Ok((false, 0));
            }
        }
        
        // No matching protocol found
        panic!("No external protocol mapped for the given protocol address");
    }

    fn execute_aquarius(
        env: &Env,
        registry_client: &registry_contract::Client,
        action: SmartAccExternalAction,
        trader_address: &Address,
        smart_account: &Address,
        tokens: Vec<Symbol>,
        tokens_amount_wad: Vec<u128>,
    ) -> Result<(bool, i128), SmartAccountError> {
        let router_address = registry_client.get_aquarius_router_address();
        let router_client = aquarius_router_contract::Client::new(env, &router_address);

        match action {
            SmartAccExternalAction::AddLiquidity => {
                // Add liquidity to XLM/USDC pool
                if tokens.len() != 2 {
                    panic!("AddLiquidity requires exactly 2 tokens");
                }

                let token0 = tokens.get(0).unwrap();
                let token1 = tokens.get(1).unwrap();
                let amount0_wad = tokens_amount_wad.get(0).unwrap();
                let amount1_wad = tokens_amount_wad.get(1).unwrap();

                // Get token addresses
                let token0_address = if token0 == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token0 == USDC_SYMBOL {
                    registry_client.get_usdc_contract_address()
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                let token1_address = if token1 == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token1 == USDC_SYMBOL {
                    registry_client.get_usdc_contract_address()
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                let token0_client = token::Client::new(env, &token0_address);
                let token1_client = token::Client::new(env, &token1_address);

                // Convert from WAD to token decimals
                let amount0 = Self::scale_from_wad(amount0_wad, token0_client.decimals());
                let amount1 = Self::scale_from_wad(amount1_wad, token1_client.decimals());

                // Ensure tokens are sorted (Aquarius requirement)
                let mut token_vec = soroban_sdk::vec![env, token0_address.clone(), token1_address.clone()];
                if token0_address > token1_address {
                    token_vec = soroban_sdk::vec![env, token1_address.clone(), token0_address.clone()];
                }

                // Init pool (will return existing if already initialized)
                let fee_fraction = 30u32; // 0.3% fee
                let (pool_index, _pool_addr) = router_client.init_standard_pool(
                    smart_account,
                    &token_vec,
                    &fee_fraction,
                );

                // Prepare deposit amounts (must match token order)
                let desired_amounts = if token0_address < token1_address {
                    soroban_sdk::vec![env, amount0 as u128, amount1 as u128]
                } else {
                    soroban_sdk::vec![env, amount1 as u128, amount0 as u128]
                };

                // Deposit liquidity
                let min_shares = 0u128; // Slippage protection can be added
                let (_deposited_amounts, lp_tokens_received) = router_client.deposit(
                    smart_account,
                    &token_vec,
                    &pool_index,
                    &desired_amounts,
                    &min_shares,
                );

                log!(
                    env,
                    "Aquarius AddLiquidity: LP tokens received {}",
                    lp_tokens_received
                );

                return Ok((true, lp_tokens_received as i128));
            }

            SmartAccExternalAction::RemoveLiquidity => {
                // Remove liquidity from XLM/USDC pool
                if tokens.len() != 2 {
                    panic!("RemoveLiquidity requires exactly 2 tokens");
                }

                let token0 = tokens.get(0).unwrap();
                let token1 = tokens.get(1).unwrap();
                let lp_amount = tokens_amount_wad.get(0).unwrap(); // LP token amount in first position

                // Get token addresses
                let token0_address = if token0 == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token0 == USDC_SYMBOL {
                    registry_client.get_usdc_contract_address()
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                let token1_address = if token1 == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token1 == USDC_SYMBOL {
                    registry_client.get_usdc_contract_address()
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                // Ensure tokens are sorted
                let mut token_vec = soroban_sdk::vec![env, token0_address.clone(), token1_address.clone()];
                if token0_address > token1_address {
                    token_vec = soroban_sdk::vec![env, token1_address.clone(), token0_address.clone()];
                }

                // Get pool index from registry (assuming it's stored)
                let pool_index = registry_client.get_aquarius_pool_index();

                // Withdraw liquidity
                let min_amounts = soroban_sdk::vec![env, 0u128, 0u128]; // Slippage protection
                let _amounts_out = router_client.withdraw(
                    smart_account,
                    &token_vec,
                    &pool_index,
                    &lp_amount,
                    &min_amounts,
                );

                log!(
                    env,
                    "Aquarius RemoveLiquidity: LP tokens burned {}",
                    lp_amount
                );

                return Ok((true, -(lp_amount as i128)));
            }

            SmartAccExternalAction::Swap => {
                // Swap tokens in Aquarius pool
                if tokens.len() != 2 {
                    panic!("Swap requires exactly 2 tokens (in and out)");
                }

                let token_in = tokens.get(0).unwrap();
                let token_out = tokens.get(1).unwrap();
                let amount_in_wad = tokens_amount_wad.get(0).unwrap();

                // Get token addresses
                let token_in_address = if token_in == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_in == USDC_SYMBOL {
                    registry_client.get_usdc_contract_address()
                } else {
                    panic!("Unsupported token for Aquarius swap");
                };

                let token_out_address = if token_out == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_out == USDC_SYMBOL {
                    registry_client.get_usdc_contract_address()
                } else {
                    panic!("Unsupported token for Aquarius swap");
                };

                let token_in_client = token::Client::new(env, &token_in_address);
                let amount_in = Self::scale_from_wad(amount_in_wad, token_in_client.decimals());

                // Ensure tokens are sorted
                let mut token_vec = soroban_sdk::vec![env, token_in_address.clone(), token_out_address.clone()];
                if token_in_address > token_out_address {
                    token_vec = soroban_sdk::vec![env, token_out_address.clone(), token_in_address.clone()];
                }

                let pool_index = registry_client.get_aquarius_pool_index();
                let min_amount_out = 0u128; // Slippage protection

                // Execute swap
                let amount_out = router_client.swap(
                    smart_account,
                    &token_vec,
                    &token_in_address,
                    &token_out_address,
                    &pool_index,
                    &(amount_in as u128),
                    &min_amount_out,
                );

                log!(
                    env,
                    "Aquarius Swap: {} -> {} out",
                    amount_in,
                    amount_out
                );

                return Ok((true, 0)); // Swap doesn't affect LP tracking
            }

            _ => panic!("Invalid action for Aquarius protocol"),
        }
    }

    fn set_borrowed_token_list(env: &Env, list: Vec<Symbol>) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::BorrowedTokensList, &list);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::BorrowedTokensList);
    }

    pub fn get_all_collateral_tokens(env: &Env) -> Vec<Symbol> {
        let collateral_tokens_list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::CollateralTokensList)
            .unwrap_or_else(|| Vec::new(&env));
        collateral_tokens_list
    }

    pub fn add_collateral_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();

        let mut existing_tokens = Self::get_all_collateral_tokens(&env);
        if !existing_tokens.contains(&token_symbol) {
            existing_tokens.push_back(token_symbol);
        }
        Self::set_collateral_tokens_list(env, existing_tokens);
        Ok(())
    }

    fn remove_collateral_token(env: &Env, token_symbol: Symbol) -> Result<(), SmartAccountError> {
        let mut existing_tokens: Vec<Symbol> = Self::get_all_collateral_tokens(&env);
        if existing_tokens.contains(&token_symbol) {
            let index = existing_tokens.first_index_of(&token_symbol).unwrap();
            existing_tokens.remove(index);
        }
        Self::set_collateral_tokens_list(env, existing_tokens);
        Ok(())
    }

    fn set_collateral_tokens_list(env: &Env, list: Vec<Symbol>) {
        env.storage()
            .persistent()
            .set(&SmartAccountDataKey::CollateralTokensList, &list);
        Self::extend_ttl_smart_account(&env, SmartAccountDataKey::CollateralTokensList);
    }

    pub fn get_collateral_token_balance(env: &Env, token_symbol: Symbol) -> U256 {
        let key_a = SmartAccountDataKey::CollateralBalanceWAD(token_symbol.clone());
        let token_balance = env
            .storage()
            .persistent()
            .get(&key_a)
            .unwrap_or_else(|| U256::from_u128(&env, 0));
        token_balance
    }

    pub fn set_collateral_token_balance(
        env: &Env,
        token_symbol: Symbol,
        balance_wad: U256,
    ) -> Result<(), SmartAccountError> {
        let account_manager = Self::get_account_manager(&env);
        account_manager.require_auth();
        Self::set_collateral_token_bal_internal(env, token_symbol, balance_wad);
        Ok(())
    }

    fn set_collateral_token_bal_internal(env: &Env, token_symbol: Symbol, balance_wad: U256) {
        let key_a = SmartAccountDataKey::CollateralBalanceWAD(token_symbol.clone());
        env.storage().persistent().set(&key_a, &balance_wad);
        Self::extend_ttl_smart_account(&env, key_a);
    }

    pub fn get_borrowed_token_debt(
        env: &Env,
        token_symbol: Symbol,
    ) -> Result<U256, SmartAccountError> {
        let registry_address = Self::get_registry_address(&env);
        let registry_client = registry_contract::Client::new(&env, &registry_address);
        let this_account = env.current_contract_address();

        let debt = if token_symbol == XLM_SYMBOL {
            lending_protocol_xlm::Client::new(&env, &registry_client.get_lendingpool_xlm())
                .get_borrow_balance(&this_account)
        } else if token_symbol == USDC_SYMBOL {
            lending_protocol_usdc::Client::new(&env, &registry_client.get_lendingpool_usdc())
                .get_borrow_balance(&this_account)
        } else if token_symbol == EURC_SYMBOL {
            lending_protocol_eurc::Client::new(&env, &registry_client.get_lendingpool_eurc())
                .get_borrow_balance(&this_account)
        } else {
            panic!("User doesn't have borrows in the given token");
        };

        Ok(debt)
    }

    pub fn is_account_active(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&SmartAccountDataKey::IsAccountActive)
            .unwrap_or(false)
    }

    fn scale_for_operation(amount_wad: u128, xlm_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(xlm_decimals)) / WAD_U128) as i128
    }

    fn scale_from_wad(amount_wad: u128, token_decimals: u32) -> i128 {
        ((amount_wad * 10u128.pow(token_decimals)) / WAD_U128) as i128
    }

    fn extend_ttl_smart_account(env: &Env, key: SmartAccountDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TLL_LEDGERS_YEAR, TLL_LEDGERS_10YEAR);
    }

    fn get_account_manager(env: &Env) -> Address {
        let account_manager: Address = env
            .storage()
            .persistent()
            .get(&SmartAccountDataKey::AccountManager)
            .unwrap_or_else(|| panic!("Failed to get account manager address"));
        account_manager
    }

    fn get_registry_address(env: &Env) -> Address {
        env.storage()
            .persistent()
            .get(&SmartAccountDataKey::RegistryContract)
            .expect("Failed to get registry contract address")
    }
}

pub mod lending_protocol_xlm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_xlm.wasm"
    );
}

pub mod lending_protocol_usdc {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_usdc.wasm"
    );
}

pub mod lending_protocol_eurc {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/lending_protocol_eurc.wasm"
    );
}

pub mod registry_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/registry_contract.wasm"
    );
}

pub mod tracking_token_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/tracking_token_contract.wasm"
    );
}

// Aquarius Router Client trait (will be implemented by actual router contract)
pub mod aquarius_router_contract {
    use soroban_sdk::{contractclient, Address, BytesN, Env, Vec};
    
    #[contractclient(name = "Client")]
    pub trait AquariusRouterTrait {
        fn init_standard_pool(
            env: Env,
            sender: Address,
            tokens: Vec<Address>,
            fee_fraction: u32,
        ) -> (BytesN<32>, Address);
        
        fn deposit(
            env: Env,
            sender: Address,
            tokens: Vec<Address>,
            pool_id: BytesN<32>,
            desired_amounts: Vec<u128>,
            min_shares: u128,
        ) -> (Vec<u128>, u128);
        
        fn withdraw(
            env: Env,
            sender: Address,
            tokens: Vec<Address>,
            pool_id: BytesN<32>,
            share_amount: u128,
            min_amounts: Vec<u128>,
        ) -> Vec<u128>;
        
        fn swap(
            env: Env,
            sender: Address,
            tokens: Vec<Address>,
            token_in: Address,
            token_out: Address,
            pool_id: BytesN<32>,
            amount_in: u128,
            min_amount_out: u128,
        ) -> u128;
    }
}

