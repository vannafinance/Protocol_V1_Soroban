#!/bin/bash

# Configuration
SOURCE_ACCOUNT="hemanth"  # Replace with your source account alias if different
NETWORK="testnet"
WASM_DIR="target/wasm32v1-none/release-with-logs"  # Adjust if your build path differs
CONTRACTS=(
    "registry_contract"
    "rate_model_contract"
    "risk_engine_contract"
    "oracle_contract"
    "smart_account_contract"
    "account_manager_contract"
    "lending_protocol_xlm"
    "lending_protocol_usdc"
    "lending_protocol_eurc"
    # Add more if needed, e.g., "lending_protocol_xlm"
)

# Array to store hashes
declare -A HASHES

# Function to install a single contract and extract hash
install_contract() {
    local contract_name="$1"
    local wasm_file="${WASM_DIR}/${contract_name}.wasm"
    
    if [ ! -f "$wasm_file" ]; then
        echo "Error: WASM file not found: $wasm_file"
        return 1
    fi
    
    echo "Installing $contract_name..."
    stellar contract upload --wasm "$wasm_file" --source "$SOURCE_ACCOUNT" --network "$NETWORK" 2>&1
    
    # Parse the hash from output (looks for "Using wasm hash <hash>")
    hash=$(echo "$output" | grep -oP 'Using wasm hash \K[0-9a-f]{64}')
    
    if [ -z "$hash" ]; then
        echo "Error: Failed to extract hash for $contract_name. Output:"
        echo "$output"
        return 1
    fi
    
    HASHES["$contract_name"]="$hash"
    echo "Success: $contract_name hash = $hash"
}

# Install all contracts
for contract in "${CONTRACTS[@]}"; do
    install_contract "$contract"
done

# Output collected hashes
echo -e "\nAll hashes collected:"
for contract in "${!HASHES[@]}"; do
    echo "$contract: ${HASHES[$contract]}"
done

# Output as Rust constants for deployer contract
echo -e "\nReady-to-copy Rust constants (add to your deployer contract):"
for contract in "${!HASHES[@]}"; do
    upper_name=$(echo "$contract" | tr '[:lower:]' '[:upper:]' | sed 's/_CONTRACT//')
    echo "const ${upper_name}_WASM_HASH: BytesN<32> = bytesn!(&env, 0x${HASHES[$contract]});"
done