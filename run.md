## First create account on testnet :

stellar keys generate --global < use any name for account> --network testnet

stellar keys fund < use any name for account> --network testnet

## Build contract
```
stellar contract build
```

- this will build a wasm file in target/<wasm_directory>/release/vanna_finance.wasm

- use the path to this wasm file to deploy the contract.


```
stellar contract deploy \
  --wasm target/wasm32v1-none/release/vanna_finance.wasm \
  --source hemanth \
  --network testnet \
  --alias vanna_finance_test
```

- you will get a contract id! after deployment..

- use contract id in --id <Contract id>

- now invoke set_admin function in liquidity_pool_xlm.rs
```
stellar contract invoke \
  --id <Contract id> \
  --source <Ur account name> \
  --network testnet \
  -- \
  set_admin \ ```
  --admin <Admin address {use ur account public key}>

- Now run get_admin to see admin set address

```stellar contract invoke \
  --id <Contract id> \
  --source <ur account name> \
  --network testnet \
  -- \
  get_admin ```

- To initialise pool..

- {WILL BE DONE ONLY ONCE} Initialise vXLM token contract address using 
- creating asset token address  for vXLM in testnet with my Public(admin) key
- Admin will be mint authority for this token using token contract address
   ```stellar contract id asset --network testnet --asset vXLM:<admin pub address>```
- the above command will generate token contract address for vXLM on testnet, 


## Initialise XLM pool.
```
stellar contract invoke \
  --id <deployed contract id> \
  --source <ur account name> \
  --network testnet \
  -- \
  initialize_pool_xlm \
  --native_token_address CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC \
  --vxlm_token_address <vXLM token contract address >
  ```



