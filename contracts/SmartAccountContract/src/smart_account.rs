use core::panic;

use soroban_sdk::{
    Address, Env, Symbol, U256, Vec, contract, contractimpl, log, panic_with_error, symbol_short,
    token, IntoVal,
};
use soroban_sdk::auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation};

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
const BLUSDC_SYMBOL: Symbol = symbol_short!("BLUSDC");
const AQUSDC_SYMBOL: Symbol = symbol_short!("AQUSDC");
const SOUSDC_SYMBOL: Symbol = symbol_short!("SOUSDC");
const EURC_SYMBOL: Symbol = symbol_short!("EURC");
const AQUARIUS_USDC_CONTRACT: &str = "CAZRY5GSFBFXD7H6GAFBA5YGYQTDXU4QKWKMYFWBAZFUCURN3WKX6LF5";
const SOROSWAP_USDC_CONTRACT: &str = "CB3TLW74NBIOT3BUWOZ3TUM6RFDF6A4GVIRUQRQZABG5KPOUL4JJOV2F";

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
        } else if token_symbol == USDC_SYMBOL || token_symbol == BLUSDC_SYMBOL {
            let pool_usdc_address = registry_client.get_lendingpool_usdc();
            let native_usdc_address = registry_client.get_usdc_contract_address();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &pool_usdc_address, &amount_scaled);
        } else if token_symbol == AQUSDC_SYMBOL {
            let pool_usdc_address = registry_client.get_lendingpool_aquarius_usdc();
            let native_usdc_address = registry_client.get_aquarius_usdc_addr();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &pool_usdc_address, &amount_scaled);
        } else if token_symbol == SOUSDC_SYMBOL {
            let pool_usdc_address = registry_client.get_lendingpool_soroswap_usdc();
            let native_usdc_address = registry_client.get_soroswap_usdc_addr();
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
        } else if token_symbol == USDC_SYMBOL || token_symbol == BLUSDC_SYMBOL {
            let native_usdc_address = registry_client.get_usdc_contract_address();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &user_address, &amount_scaled);
        } else if token_symbol == AQUSDC_SYMBOL {
            let native_usdc_address = registry_client.get_aquarius_usdc_addr();
            let usdc_token = token::Client::new(&env, &native_usdc_address);
            let amount_scaled = Self::scale_for_operation(amount_wad, usdc_token.decimals());
            usdc_token.transfer(&this_account, &user_address, &amount_scaled);
        } else if token_symbol == SOUSDC_SYMBOL {
            let native_usdc_address = registry_client.get_soroswap_usdc_addr();
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

        // Route by target_protocol address to determine which external protocol to invoke.

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

        // Route Soroswap-like actions directly by target protocol address.
        // This keeps compatibility with older deployed Registry ABIs that may not
        // expose has/get_soroswap_router_address.
        if action == SmartAccExternalAction::AddLiquidity
            || action == SmartAccExternalAction::RemoveLiquidity
            || action == SmartAccExternalAction::Swap
        {
            let is_blend_target = if registry_client.has_blend_pool_address() {
                target_protocol == registry_client.get_blend_pool_address()
            } else {
                false
            };

            if !is_blend_target {
                return Self::execute_soroswap(
                    env,
                    &registry_client,
                    &target_protocol,
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
                        address: native_xlm_address.clone(),
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let mut auth_args = soroban_sdk::Vec::new(env);
                    auth_args.push_back(smart_account.to_val());
                    auth_args.push_back(blend_pool_address.to_val());
                    auth_args.push_back((amt as i128).into_val(env));
                    env.authorize_as_current_contract(soroban_sdk::vec![env,
                        InvokerContractAuthEntry::Contract(SubContractInvocation {
                            context: ContractContext {
                                contract: native_xlm_address.clone(),
                                fn_name: Symbol::new(env, "transfer"),
                                args: auth_args,
                            },
                            sub_invocations: soroban_sdk::Vec::new(env),
                        })
                    ]);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &smart_account,
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
                        address: usdc_contract_address.clone(),
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let mut auth_args = soroban_sdk::Vec::new(env);
                    auth_args.push_back(smart_account.to_val());
                    auth_args.push_back(blend_pool_address.to_val());
                    auth_args.push_back((amt as i128).into_val(env));
                    env.authorize_as_current_contract(soroban_sdk::vec![env,
                        InvokerContractAuthEntry::Contract(SubContractInvocation {
                            context: ContractContext {
                                contract: usdc_contract_address.clone(),
                                fn_name: Symbol::new(env, "transfer"),
                                args: auth_args,
                            },
                            sub_invocations: soroban_sdk::Vec::new(env),
                        })
                    ]);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &smart_account,
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
                        address: eurc_contract_address.clone(),
                        amount: amt,
                        request_type,
                    };
                    let mut requests = Vec::new(env);
                    requests.push_back(request);

                    let mut auth_args = soroban_sdk::Vec::new(env);
                    auth_args.push_back(smart_account.to_val());
                    auth_args.push_back(blend_pool_address.to_val());
                    auth_args.push_back((amt as i128).into_val(env));
                    env.authorize_as_current_contract(soroban_sdk::vec![env,
                        InvokerContractAuthEntry::Contract(SubContractInvocation {
                            context: ContractContext {
                                contract: eurc_contract_address.clone(),
                                fn_name: Symbol::new(env, "transfer"),
                                args: auth_args,
                            },
                            sub_invocations: soroban_sdk::Vec::new(env),
                        })
                    ]);

                    let positions = blend_pool_client.submit(
                        &smart_account,
                        &smart_account,
                        &smart_account,
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
                    Self::get_aquarius_usdc_address(env)
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                let token1_address = if token1 == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token1 == USDC_SYMBOL {
                    Self::get_aquarius_usdc_address(env)
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                let token0_client = token::Client::new(env, &token0_address);
                let token1_client = token::Client::new(env, &token1_address);

                // Convert from WAD to token decimals
                let amount0 = Self::scale_from_wad(amount0_wad, token0_client.decimals());
                let amount1 = Self::scale_from_wad(amount1_wad, token1_client.decimals());

                // Ensure tokens are sorted (Aquarius requirement)
                let (token_a, token_b, amount_a, amount_b) = if token0_address <= token1_address {
                    (
                        token0_address.clone(),
                        token1_address.clone(),
                        amount0 as u128,
                        amount1 as u128,
                    )
                } else {
                    (
                        token1_address.clone(),
                        token0_address.clone(),
                        amount1 as u128,
                        amount0 as u128,
                    )
                };

                let token_vec = soroban_sdk::vec![env, token_a.clone(), token_b.clone()];
                let desired_amounts = soroban_sdk::vec![env, amount_a, amount_b];

                // Use registry-configured pool index (pool should be pre-initialized)
                let pool_index = registry_client.get_aquarius_pool_index();
                let pool_address = router_client.get_pool(&token_vec, &pool_index);
                let pool_client = aquarius_pool_contract::Client::new(env, &pool_address);

                // Deposit liquidity directly on pool.
                // NOTE: For the live Aquarius pool deployment on testnet, transfer auth entries
                // must be recorded as top-level invocations for contract-address auth matching.
                let min_shares = 0u128; // Slippage protection can be added
                env.authorize_as_current_contract(soroban_sdk::vec![
                    env,
                    // Auth for pool.deposit
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: pool_address.clone(),
                            fn_name: Symbol::new(env, "deposit"),
                            args: (
                                smart_account.clone(),
                                desired_amounts.clone(),
                                min_shares,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    // Auth for token_a.transfer
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: token_a.clone(),
                            fn_name: Symbol::new(env, "transfer"),
                            args: (
                                smart_account.clone(),
                                pool_address.clone(),
                                amount_a as i128,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    // Auth for token_b.transfer
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: token_b.clone(),
                            fn_name: Symbol::new(env, "transfer"),
                            args: (
                                smart_account.clone(),
                                pool_address.clone(),
                                amount_b as i128,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                ]);
                let (_deposited_amounts, lp_tokens_received) = pool_client.deposit(
                    smart_account,
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
                    Self::get_aquarius_usdc_address(env)
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                let token1_address = if token1 == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token1 == USDC_SYMBOL {
                    Self::get_aquarius_usdc_address(env)
                } else {
                    panic!("Unsupported token for Aquarius");
                };

                // Ensure tokens are sorted
                let (token_a, token_b) = if token0_address <= token1_address {
                    (token0_address.clone(), token1_address.clone())
                } else {
                    (token1_address.clone(), token0_address.clone())
                };
                let token_vec = soroban_sdk::vec![env, token_a.clone(), token_b.clone()];

                // Get pool index from registry (assuming it's stored)
                let pool_index = registry_client.get_aquarius_pool_index();
                let pool_address = router_client.get_pool(&token_vec, &pool_index);
                let share_token_address = router_client.share_id(&token_vec, &pool_index);

                // Withdraw liquidity directly on pool. This keeps authorization deterministic
                // and avoids router->pool raw invoke auth propagation issues.
                let min_amounts = soroban_sdk::vec![env, 0u128, 0u128]; // Slippage protection
                let pool_client = aquarius_pool_contract::Client::new(env, &pool_address);

                // Register both withdraw and burn as top-level auth entries. In this
                // contract-invoker flow, nested-only burn auth can be rejected by the
                // token contract require_auth path.
                env.authorize_as_current_contract(soroban_sdk::vec![
                    env,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: pool_address.clone(),
                            fn_name: Symbol::new(env, "withdraw"),
                            args: (
                                smart_account.clone(),
                                lp_amount,
                                min_amounts.clone(),
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: share_token_address.clone(),
                            fn_name: Symbol::new(env, "burn"),
                            args: (smart_account.clone(), lp_amount as i128).into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                ]);

                let _amounts_out = pool_client.withdraw(
                    smart_account,
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
                    Self::get_aquarius_usdc_address(env)
                } else {
                    panic!("Unsupported token for Aquarius swap");
                };

                let token_out_address = if token_out == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_out == USDC_SYMBOL {
                    Self::get_aquarius_usdc_address(env)
                } else {
                    panic!("Unsupported token for Aquarius swap");
                };

                let token_in_client = token::Client::new(env, &token_in_address);
                let amount_in = Self::scale_from_wad(amount_in_wad, token_in_client.decimals());

                // Ensure tokens are sorted
                let (token_a, token_b) = if token_in_address <= token_out_address {
                    (token_in_address.clone(), token_out_address.clone())
                } else {
                    (token_out_address.clone(), token_in_address.clone())
                };
                let token_vec = soroban_sdk::vec![env, token_a.clone(), token_b.clone()];

                let pool_index = registry_client.get_aquarius_pool_index();
                let pool_address = router_client.get_pool(&token_vec, &pool_index);
                let min_amount_out = 0u128; // Slippage protection

                let (in_idx, out_idx) = if token_in_address == token_a {
                    (0u32, 1u32)
                } else {
                    (1u32, 0u32)
                };

                // Register router.swap and pool.swap as separate top-level auth entries.
                // The Aquarius router calls pool.swap via e.invoke_contract (raw host call),
                // which does not propagate nested sub-invocations. Registering pool.swap as a
                // top-level entry ensures the pool's user.require_auth() can find it directly.
                env.authorize_as_current_contract(soroban_sdk::vec![
                    env,
                    // Auth for router.swap (router calls user.require_auth)
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: router_address.clone(),
                            fn_name: Symbol::new(env, "swap"),
                            args: (
                                smart_account.clone(),
                                token_vec.clone(),
                                token_in_address.clone(),
                                token_out_address.clone(),
                                pool_index.clone(),
                                amount_in as u128,
                                min_amount_out,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    // Auth for pool.swap (pool calls user.require_auth after router invokes it)
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: pool_address.clone(),
                            fn_name: Symbol::new(env, "swap"),
                            args: (
                                smart_account.clone(),
                                in_idx,
                                out_idx,
                                amount_in as u128,
                                min_amount_out,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::vec![
                            env,
                            // Auth for token.transfer (pool calls transfer after require_auth)
                            InvokerContractAuthEntry::Contract(SubContractInvocation {
                                context: ContractContext {
                                    contract: token_in_address.clone(),
                                    fn_name: Symbol::new(env, "transfer"),
                                    args: (
                                        smart_account.clone(),
                                        pool_address.clone(),
                                        amount_in as i128,
                                    )
                                        .into_val(env),
                                },
                                sub_invocations: soroban_sdk::Vec::new(env),
                            }),
                        ],
                    }),
                ]);

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

    fn execute_soroswap(
        env: &Env,
        registry_client: &registry_contract::Client,
        router_address: &Address,
        action: SmartAccExternalAction,
        _trader_address: &Address,
        smart_account: &Address,
        tokens: Vec<Symbol>,
        tokens_amount_wad: Vec<u128>,
    ) -> Result<(bool, i128), SmartAccountError> {
        let router_client = soroswap_router_contract::Client::new(env, &router_address);
        // Set a generous deadline: current ledger timestamp + 1 day in seconds
        let deadline = env.ledger().timestamp() + 86400u64;

        // Use Soroswap-specific USDC if configured, otherwise fall back to the global Registry USDC.
        // This allows Soroswap and Aquarius to each use their own USDC token simultaneously.
        let soroswap_usdc_address = Self::get_soroswap_usdc_address(env);

        match action {
            SmartAccExternalAction::AddLiquidity => {
                if tokens.len() != 2 {
                    panic!("Soroswap AddLiquidity requires exactly 2 tokens");
                }

                let token_a_sym = tokens.get(0).unwrap();
                let token_b_sym = tokens.get(1).unwrap();
                let amount_a_wad = tokens_amount_wad.get(0).unwrap();
                let amount_b_wad = tokens_amount_wad.get(1).unwrap();

                let token_a_address = if token_a_sym == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_a_sym == USDC_SYMBOL {
                    soroswap_usdc_address.clone()
                } else if token_a_sym == EURC_SYMBOL {
                    registry_client.get_eurc_contract_address()
                } else {
                    panic!("Unsupported token for Soroswap");
                };

                let token_b_address = if token_b_sym == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_b_sym == USDC_SYMBOL {
                    soroswap_usdc_address.clone()
                } else if token_b_sym == EURC_SYMBOL {
                    registry_client.get_eurc_contract_address()
                } else {
                    panic!("Unsupported token for Soroswap");
                };

                let token_a_client = token::Client::new(env, &token_a_address);
                let token_b_client = token::Client::new(env, &token_b_address);

                let amount_a = Self::scale_from_wad(amount_a_wad, token_a_client.decimals());
                let amount_b = Self::scale_from_wad(amount_b_wad, token_b_client.decimals());

                // Compute pair address for auth sub-invocations
                let pair_address = router_client.router_pair_for(&token_a_address, &token_b_address);

                // Soroswap add_liquidity can transfer slightly less than desired on one side
                // (based on current reserves). Auth must match those exact transfer args.
                let (auth_amount_a, auth_amount_b) =
                    Self::compute_soroswap_add_liquidity_auth_amounts(
                        env,
                        &pair_address,
                        &token_a_address,
                        amount_a,
                        amount_b,
                    );

                // Register auth: router.add_liquidity requires smart_account's auth,
                // which internally transfers token_a and token_b from smart_account to pair.
                // Register all three as top-level entries for deterministic auth matching.
                env.authorize_as_current_contract(soroban_sdk::vec![
                    env,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: router_address.clone(),
                            fn_name: Symbol::new(env, "add_liquidity"),
                            args: (
                                token_a_address.clone(),
                                token_b_address.clone(),
                                amount_a,
                                amount_b,
                                0i128,
                                0i128,
                                smart_account.clone(),
                                deadline,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: token_a_address.clone(),
                            fn_name: Symbol::new(env, "transfer"),
                            args: (
                                smart_account.clone(),
                                pair_address.clone(),
                                auth_amount_a,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: token_b_address.clone(),
                            fn_name: Symbol::new(env, "transfer"),
                            args: (
                                smart_account.clone(),
                                pair_address.clone(),
                                auth_amount_b,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                ]);

                let (_actual_a, _actual_b, liquidity) = router_client.add_liquidity(
                    &token_a_address,
                    &token_b_address,
                    &amount_a,
                    &amount_b,
                    &0i128,
                    &0i128,
                    smart_account,
                    &deadline,
                );

                log!(
                    env,
                    "Soroswap AddLiquidity: LP tokens received {}",
                    liquidity
                );

                Ok((true, liquidity))
            }

            SmartAccExternalAction::RemoveLiquidity => {
                if tokens.len() != 2 {
                    panic!("Soroswap RemoveLiquidity requires exactly 2 tokens");
                }

                let token_a_sym = tokens.get(0).unwrap();
                let token_b_sym = tokens.get(1).unwrap();
                // LP amount is passed as raw token units (not WAD-scaled)
                let lp_amount = tokens_amount_wad.get(0).unwrap() as i128;

                let token_a_address = if token_a_sym == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_a_sym == USDC_SYMBOL {
                    soroswap_usdc_address.clone()
                } else if token_a_sym == EURC_SYMBOL {
                    registry_client.get_eurc_contract_address()
                } else {
                    panic!("Unsupported token for Soroswap");
                };

                let token_b_address = if token_b_sym == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_b_sym == USDC_SYMBOL {
                    soroswap_usdc_address.clone()
                } else if token_b_sym == EURC_SYMBOL {
                    registry_client.get_eurc_contract_address()
                } else {
                    panic!("Unsupported token for Soroswap");
                };

                // In Soroswap the pair contract IS the LP token contract
                let pair_address = router_client.router_pair_for(&token_a_address, &token_b_address);

                // Auth: router.remove_liquidity calls smart_account.require_auth(),
                // then transfers LP tokens from smart_account to pair.
                env.authorize_as_current_contract(soroban_sdk::vec![
                    env,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: router_address.clone(),
                            fn_name: Symbol::new(env, "remove_liquidity"),
                            args: (
                                token_a_address.clone(),
                                token_b_address.clone(),
                                lp_amount,
                                0i128,
                                0i128,
                                smart_account.clone(),
                                deadline,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    // LP token (pair contract) transfer from smart_account to pair
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: pair_address.clone(),
                            fn_name: Symbol::new(env, "transfer"),
                            args: (
                                smart_account.clone(),
                                pair_address.clone(),
                                lp_amount,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                ]);

                router_client.remove_liquidity(
                    &token_a_address,
                    &token_b_address,
                    &lp_amount,
                    &0i128,
                    &0i128,
                    smart_account,
                    &deadline,
                );

                log!(
                    env,
                    "Soroswap RemoveLiquidity: LP tokens burned {}",
                    lp_amount
                );

                Ok((true, -lp_amount))
            }

            SmartAccExternalAction::Swap => {
                if tokens.len() != 2 {
                    panic!("Soroswap Swap requires exactly 2 tokens (in and out)");
                }

                let token_in_sym = tokens.get(0).unwrap();
                let token_out_sym = tokens.get(1).unwrap();
                let amount_in_wad = tokens_amount_wad.get(0).unwrap();

                let token_in_address = if token_in_sym == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_in_sym == USDC_SYMBOL {
                    soroswap_usdc_address.clone()
                } else if token_in_sym == EURC_SYMBOL {
                    registry_client.get_eurc_contract_address()
                } else {
                    panic!("Unsupported input token for Soroswap swap");
                };

                let token_out_address = if token_out_sym == XLM_SYMBOL {
                    registry_client.get_xlm_contract_adddress()
                } else if token_out_sym == USDC_SYMBOL {
                    soroswap_usdc_address.clone()
                } else if token_out_sym == EURC_SYMBOL {
                    registry_client.get_eurc_contract_address()
                } else {
                    panic!("Unsupported output token for Soroswap swap");
                };

                let token_in_client = token::Client::new(env, &token_in_address);
                let amount_in = Self::scale_from_wad(amount_in_wad, token_in_client.decimals());

                let mut path = soroban_sdk::Vec::new(env);
                path.push_back(token_in_address.clone());
                path.push_back(token_out_address.clone());

                // Get pair address for input-token transfer auth
                let pair_address = router_client.router_pair_for(&token_in_address, &token_out_address);

                // Auth: router.swap_exact_tokens_for_tokens calls smart_account.require_auth(),
                // then transfers token_in from smart_account to pair.
                env.authorize_as_current_contract(soroban_sdk::vec![
                    env,
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: router_address.clone(),
                            fn_name: Symbol::new(env, "swap_exact_tokens_for_tokens"),
                            args: (
                                amount_in,
                                0i128,
                                path.clone(),
                                smart_account.clone(),
                                deadline,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                    InvokerContractAuthEntry::Contract(SubContractInvocation {
                        context: ContractContext {
                            contract: token_in_address.clone(),
                            fn_name: Symbol::new(env, "transfer"),
                            args: (
                                smart_account.clone(),
                                pair_address.clone(),
                                amount_in,
                            )
                                .into_val(env),
                        },
                        sub_invocations: soroban_sdk::Vec::new(env),
                    }),
                ]);

                let amounts = router_client.swap_exact_tokens_for_tokens(
                    &amount_in,
                    &0i128,
                    &path,
                    smart_account,
                    &deadline,
                );

                let amount_out = amounts.get(amounts.len() - 1).unwrap_or(0);
                log!(
                    env,
                    "Soroswap Swap: {} in -> {} out",
                    amount_in,
                    amount_out
                );

                Ok((true, 0)) // Swap does not affect LP tracking
            }

            _ => panic!("Invalid action for Soroswap protocol"),
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
        } else if token_symbol == USDC_SYMBOL || token_symbol == BLUSDC_SYMBOL {
            lending_protocol_usdc::Client::new(&env, &registry_client.get_lendingpool_usdc())
                .get_borrow_balance(&this_account)
        } else if token_symbol == AQUSDC_SYMBOL {
            lending_protocol_usdc::Client::new(&env, &registry_client.get_lendingpool_aquarius_usdc())
                .get_borrow_balance(&this_account)
        } else if token_symbol == SOUSDC_SYMBOL {
            lending_protocol_usdc::Client::new(&env, &registry_client.get_lendingpool_soroswap_usdc())
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

    fn get_aquarius_usdc_address(env: &Env) -> Address {
        Address::from_str(env, AQUARIUS_USDC_CONTRACT)
    }

    fn get_soroswap_usdc_address(env: &Env) -> Address {
        Address::from_str(env, SOROSWAP_USDC_CONTRACT)
    }

    fn compute_soroswap_add_liquidity_auth_amounts(
        env: &Env,
        pair_address: &Address,
        token_a_address: &Address,
        amount_a_desired: i128,
        amount_b_desired: i128,
    ) -> (i128, i128) {
        let mut auth_amount_a = amount_a_desired;
        let mut auth_amount_b = amount_b_desired;

        if amount_a_desired <= 0 || amount_b_desired <= 0 {
            return (auth_amount_a, auth_amount_b);
        }

        let pair_client = soroswap_pair_contract::Client::new(env, pair_address);
        let (reserve0, reserve1) = pair_client.get_reserves();
        let token0 = pair_client.token_0();

        // Map reserves into the same (A,B) token order used for add_liquidity call.
        let (reserve_a, reserve_b) = if *token_a_address == token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };

        // If pool already has reserves, router computes optimal counterpart amount via quote().
        if reserve_a > 0 && reserve_b > 0 {
            let amount_b_optimal = (amount_a_desired * reserve_b) / reserve_a;
            if amount_b_optimal <= amount_b_desired {
                auth_amount_b = amount_b_optimal;
            } else {
                let amount_a_optimal = (amount_b_desired * reserve_a) / reserve_b;
                auth_amount_a = amount_a_optimal;
            }
        }

        (auth_amount_a, auth_amount_b)
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

        fn get_pool(
            env: Env,
            tokens: Vec<Address>,
            pool_id: BytesN<32>,
        ) -> Address;

        fn share_id(
            env: Env,
            tokens: Vec<Address>,
            pool_id: BytesN<32>,
        ) -> Address;
        
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

pub mod aquarius_pool_contract {
    use soroban_sdk::{contractclient, Address, Env, Vec};

    #[contractclient(name = "Client")]
    pub trait AquariusPoolTrait {
        fn deposit(
            env: Env,
            user: Address,
            desired_amounts: Vec<u128>,
            min_shares: u128,
        ) -> (Vec<u128>, u128);

        fn withdraw(
            env: Env,
            user: Address,
            share_amount: u128,
            min_amounts: Vec<u128>,
        ) -> Vec<u128>;
    }
}

// Soroswap Router client trait
pub mod soroswap_router_contract {
    use soroban_sdk::{contractclient, Address, Env, Vec};

    #[contractclient(name = "Client")]
    pub trait SoroswapRouterTrait {
        /// Add liquidity to a token pair pool, creating it if needed.
        /// Returns (amount_a_actual, amount_b_actual, lp_tokens_minted).
        fn add_liquidity(
            env: Env,
            token_a: Address,
            token_b: Address,
            amount_a_desired: i128,
            amount_b_desired: i128,
            amount_a_min: i128,
            amount_b_min: i128,
            to: Address,
            deadline: u64,
        ) -> (i128, i128, i128);

        /// Remove liquidity from a token pair pool by burning LP tokens.
        /// Returns (amount_a_received, amount_b_received).
        fn remove_liquidity(
            env: Env,
            token_a: Address,
            token_b: Address,
            liquidity: i128,
            amount_a_min: i128,
            amount_b_min: i128,
            to: Address,
            deadline: u64,
        ) -> (i128, i128);

        /// Swap exact input tokens for as many output tokens as possible.
        /// Returns amounts at each hop of the path.
        fn swap_exact_tokens_for_tokens(
            env: Env,
            amount_in: i128,
            amount_out_min: i128,
            path: Vec<Address>,
            to: Address,
            deadline: u64,
        ) -> Vec<i128>;

        /// Get the factory contract address used by this router.
        fn get_factory(env: Env) -> Address;

        /// Compute the deterministic pair address for two tokens.
        fn router_pair_for(env: Env, token_a: Address, token_b: Address) -> Address;
    }
}

// Soroswap Pair client trait
pub mod soroswap_pair_contract {
    use soroban_sdk::{contractclient, Address, Env};

    #[contractclient(name = "Client")]
    pub trait SoroswapPairTrait {
        fn get_reserves(env: Env) -> (i128, i128);
        fn token_0(env: Env) -> Address;
    }
}
