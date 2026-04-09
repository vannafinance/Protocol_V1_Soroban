#!/bin/bash

set -e  # Exit on any error

# Configuration
NETWORK="--network testnet"
WASM_DIR="target/wasm32v1-none/release-with-logs/"
HEMANTH="vanna_deployer"
ELEPHANT2="elephant2"
ELEPHANT4="elephant4"
ELEPHANT5="elephant5"
ADMIN="GAUVY7FNDKVWRMW3SYEMX6QMFSWQDKC6XIPJJKAMOEMLZPAI7XZPDV3D"
LENDER="GAKEPI64RXSQDRGEDBTHJO3JZJ6HERW37AX6PJWQJ6UW7HSI6PSQX2S6"
USER1="GCVJJEHEEWLA5A6KM26ZEUUXLW33NY353AXMVQ5GIUHEJSZNYVURTK2F"
USER2="GBBDNBO7KNF4RCHLIRTGL64W4IHPUIMPVVVFZCIFCQD4M6ZU54XUYTP5"
BLUSDC_CONTRACT="CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU"

run_with_retry_capture() {
  local max_attempts=$1
  shift
  local attempt=1
  local output

  while [ ${attempt} -le ${max_attempts} ]; do
    output=$("$@" 2>&1)
    if [ $? -eq 0 ]; then
      printf '%s\n' "${output}"
      return 0
    fi

    if [ ${attempt} -eq ${max_attempts} ]; then
      printf '%s\n' "${output}"
      return 1
    fi

    echo "Retrying command (${attempt}/${max_attempts})..." >&2
    attempt=$((attempt + 1))
  done
}

# Function to install WASM and extract hash
install_wasm() {
  local name=$1
  local file=$2
  echo "Installing ${name}..." >&2
  local output
  output=$(run_with_retry_capture 4 stellar contract upload --wasm "${WASM_DIR}${file}.wasm" --source "${HEMANTH}" ${NETWORK})
  if [ $? -ne 0 ]; then
    echo "Error: Failed to upload WASM for ${name}" >&2
    echo "${output}" >&2
    exit 1
  fi
  local hash=$(echo "${output}" | grep -oE '[0-9a-f]{64}' | tail -1)
  if [ -z "${hash}" ]; then
    echo "Error: Failed to extract WASM hash for ${name}" >&2
    echo "${output}" >&2
    exit 1
  fi
  printf '%s\n' "${hash}"
}

echo "=== Deploying deployer contract (apple) ==="
DEPLOYER_APPLE_OUTPUT=$(stellar contract deploy --wasm "${WASM_DIR}deployer_contract.wasm" --source "${HEMANTH}" ${NETWORK} --alias deployer_apple -- --admin "${ADMIN}")
DEPLOYER_APPLE_ID=$(echo "${DEPLOYER_APPLE_OUTPUT}" | grep -oE 'C[A-Z0-9]{55}' | tail -1)
echo "${DEPLOYER_APPLE_OUTPUT}"

if [ -z "${DEPLOYER_APPLE_ID}" ]; then
  echo "❌ Failed to extract deployer_apple contract ID"
  exit 1
fi

# Install all supporting contracts and collect WASM hashes
REGISTRY_WASM_HASH=$(install_wasm "registry_contract" "registry_contract")
RATE_MODEL_WASM_HASH=$(install_wasm "rate_model_contract" "rate_model_contract")
RISK_ENGINE_WASM_HASH=$(install_wasm "risk_engine_contract" "risk_engine_contract")
ORACLE_WASM_HASH=$(install_wasm "oracle_contract" "oracle_contract")
SMART_ACCOUNT_HASH=$(install_wasm "smart_account_contract" "smart_account_contract")
ACCOUNT_MANAGER_WASM_HASH=$(install_wasm "account_manager_contract" "account_manager_contract")
LENDING_POOL_XLM_HASH=$(install_wasm "lending_protocol_xlm" "lending_protocol_xlm")
LENDING_POOL_USDC_HASH=$(install_wasm "lending_protocol_usdc" "lending_protocol_usdc")
LENDING_POOL_EURC_HASH=$(install_wasm "lending_protocol_eurc" "lending_protocol_eurc")
VXLM_TOKEN_HASH=$(install_wasm "vxlm_token_contract" "vxlm_token_contract")
VUSDC_TOKEN_HASH=$(install_wasm "vusdc_token_contract" "vusdc_token_contract")
VEURC_TOKEN_HASH=$(install_wasm "veurc_token_contract" "veurc_token_contract")

echo "=== Deploying core contracts via deploy_all ==="
output=$(stellar contract invoke \
  --id "${DEPLOYER_APPLE_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  deploy_all \
  --registry_contract_wasm_hash "${REGISTRY_WASM_HASH}" \
  --risk_engine_wasm_hash "${RISK_ENGINE_WASM_HASH}" \
  --rate_model_wasm_hash "${RATE_MODEL_WASM_HASH}" \
  --oracle_wasm_hash "${ORACLE_WASM_HASH}" \
  --account_manager_wasm_hash "${ACCOUNT_MANAGER_WASM_HASH}" \
  --smart_account_hash "${SMART_ACCOUNT_HASH}" 2>&1)

# Extract deployed IDs from the final JSON array (order: registry, rate_model, risk_engine, oracle, account_manager)
IDS=$(echo "${output}" | tail -1)
REGISTRY_ID=$(node -e "const ids = JSON.parse(process.argv[1]); console.log(ids[0]);" "${IDS}")
RATE_MODEL_ID=$(node -e "const ids = JSON.parse(process.argv[1]); console.log(ids[1]);" "${IDS}")
RISK_ENGINE_ID=$(node -e "const ids = JSON.parse(process.argv[1]); console.log(ids[2]);" "${IDS}")
ORACLE_ID=$(node -e "const ids = JSON.parse(process.argv[1]); console.log(ids[3]);" "${IDS}")
ACCOUNT_MANAGER_ID=$(node -e "const ids = JSON.parse(process.argv[1]); console.log(ids[4]);" "${IDS}")

echo "=== Deploying pool deployer contract (mango) ==="
POOL_DEPLOYER_OUTPUT=$(stellar contract deploy --wasm "${WASM_DIR}pool_deployer_contract.wasm" --source "${HEMANTH}" ${NETWORK} --alias deployer_mango -- --admin "${ADMIN}")
POOL_DEPLOYER_ID=$(echo "${POOL_DEPLOYER_OUTPUT}" | grep -oE 'C[A-Z0-9]{55}' | tail -1)
echo "${POOL_DEPLOYER_OUTPUT}"

if [ -z "${POOL_DEPLOYER_ID}" ]; then
  echo "❌ Failed to extract deployer_mango contract ID"
  exit 1
fi

echo "=== Deploying XLM pool and VXLM token via deploy_lps_and_token_contracts ==="
output=$(stellar contract invoke \
  --id "${POOL_DEPLOYER_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  deploy_lps_and_token_contracts \
    --registry_address "${REGISTRY_ID}" \
    --account_manager "${ACCOUNT_MANAGER_ID}" \
    --rate_model "${RATE_MODEL_ID}" \
    --lending_pool_xlm_hash "${LENDING_POOL_XLM_HASH}" \
    --vxlm_contract_hash "${VXLM_TOKEN_HASH}" \
    --vusdc_contract_hash "${VUSDC_TOKEN_HASH}" \
    --veurc_contract_hash "${VEURC_TOKEN_HASH}" 2>&1)

# Extract deployed IDs from log addresses (first: xlm_pool, second: vxlm_token)
XLM_POOL_ID=$(echo "${output}" | grep 'Deployed xlm pool contract' | sed -n 's/.*"address":"\([A-Z0-9]\+\)".*/\1/p' | tail -1)
VXLM_TOKEN_ID=$(echo "${output}" | grep 'Deployed vxlm token contract' | sed -n 's/.*"address":"\([A-Z0-9]\+\)".*/\1/p' | tail -1)

if [ -z "${XLM_POOL_ID}" ] || [ -z "${VXLM_TOKEN_ID}" ]; then
  echo "❌ Failed to extract XLM pool or VXLM token IDs from deploy logs"
  echo "${output}"
  exit 1
fi

echo "=== Initializing VXLM token contract ==="
stellar contract invoke \
  --id "${VXLM_TOKEN_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  initialize \
   --admin "${XLM_POOL_ID}" \
   --decimal 6 \
   --name 'VXLM TOKEN' \
   --symbol 'VXLM'

echo "=== Initializing XLM pool ==="
stellar contract invoke \
  --id "${XLM_POOL_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  initialize_pool_xlm \
  --vxlm_token_contract_address "${VXLM_TOKEN_ID}"

echo "=== Depositing XLM (11 units) ==="
stellar contract invoke \
  --id "${XLM_POOL_ID}" \
  --source-account "${ELEPHANT2}" \
  ${NETWORK} \
  -- \
  deposit_xlm \
  --lender "${LENDER}" \
  --amount_wad 11000000000000000000 || echo "⚠️ Skipping XLM deposit seed step (funding or account state issue)"

echo "=== Creating margin account for user1 (${USER1}) ==="
set +e
output=$(stellar contract invoke \
  --id "${ACCOUNT_MANAGER_ID}" \
  --source-account "${ELEPHANT4}" \
  ${NETWORK} \
  -- \
  create_account \
  --trader_address "${USER1}" 2>&1)
status=$?
if [ ${status} -eq 0 ]; then
  MARGIN1_ID=$(echo "${output}" | tail -1)
  echo "Margin account 1 ID: ${MARGIN1_ID}"
else
  MARGIN1_ID=""
  echo "⚠️ Skipping margin account 1 seed step"
  echo "${output}"
fi

echo "=== Creating margin account for user2 (${USER2}) ==="
output=$(stellar contract invoke \
  --id "${ACCOUNT_MANAGER_ID}" \
  --source-account "${ELEPHANT5}" \
  ${NETWORK} \
  -- \
  create_account \
  --trader_address "${USER2}" 2>&1)
status=$?
if [ ${status} -eq 0 ]; then
  MARGIN2_ID=$(echo "${output}" | tail -1)
  echo "Margin account 2 ID: ${MARGIN2_ID}"
else
  MARGIN2_ID=""
  echo "⚠️ Skipping margin account 2 seed step"
  echo "${output}"
fi
set -e

echo "=== Setting USDC as allowed collateral ==="
stellar contract invoke \
  --id "${ACCOUNT_MANAGER_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  set_iscollateral_allowed \
  --token_symbol BLUSDC

echo "=== Setting AqUSDC as allowed collateral ==="
stellar contract invoke \
  --id "${ACCOUNT_MANAGER_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  set_iscollateral_allowed \
  --token_symbol AQUSDC

echo "=== Setting SoUSDC as allowed collateral ==="
stellar contract invoke \
  --id "${ACCOUNT_MANAGER_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  set_iscollateral_allowed \
  --token_symbol SOUSDC

echo "=== Setting max asset cap (10) ==="
stellar contract invoke \
  --id "${ACCOUNT_MANAGER_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  set_max_asset_cap \
  --cap 10000000000000000000

echo "=== Setting USDC issuer on registry ==="
stellar contract invoke \
  --id "${REGISTRY_ID}" \
  --source-account "${HEMANTH}" \
  ${NETWORK} \
  -- \
  set_native_usdc_contract_address \
  --usdc_contract_address "${BLUSDC_CONTRACT}"

echo "=== Depositing USDC collateral (3 units) for user2 ==="
if [ -n "${MARGIN2_ID}" ]; then
  stellar contract invoke \
    --id "${ACCOUNT_MANAGER_ID}" \
    --source-account "${ELEPHANT5}" \
    ${NETWORK} \
    -- \
    deposit_collateral_tokens \
    --smart_account "${MARGIN2_ID}" \
    --token_symbol BLUSDC \
    --token_amount_wad 3000000000000000000 || echo "⚠️ Skipping collateral seed step"
else
  echo "⚠️ Skipping collateral seed step because margin account 2 was not created"
fi

echo "=== Automation complete! Summary of key contract IDs ==="
echo "Deployer Apple: ${DEPLOYER_APPLE_ID}"
echo "Registry: ${REGISTRY_ID}"
echo "Rate Model: ${RATE_MODEL_ID}"
echo "Risk Engine: ${RISK_ENGINE_ID}"
echo "Oracle: ${ORACLE_ID}"
echo "Account Manager: ${ACCOUNT_MANAGER_ID}"
echo "Pool Deployer Mango: ${POOL_DEPLOYER_ID}"
echo "XLM Pool: ${XLM_POOL_ID}"
echo "VXLM Token: ${VXLM_TOKEN_ID}"
echo "Margin Account 1 (User1): ${MARGIN1_ID}"
echo "Margin Account 2 (User2): ${MARGIN2_ID}"