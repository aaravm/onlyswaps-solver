# onlyswaps-solver

## Build
First retrieve and build submodules with `./build-dependencies.sh` then run `cargo build`

## Test
`cargo test`

## Docker build
`docker build .`

## Configuration
| Environment Variable | Mandatory? | Description                                                                         | Example                                  | Default                 |
| -------------------- | ---------- | ----------------------------------------------------------------------------------- | ---------------------------------------- |-------------------------|
| `SOLVER_PRIVATE_KEY` | Yes        | A hex-encoded private key, with or without the `0x`                                 | `0xdeadbeefdeadbeefdeadbeefdeadbeefdead` | —                       |
| `SOLVER_CONFIG_PATH` | No         | Path to your solver configuration JSON (must match format in `config_default.json`) | `/data/config.json`                      | `~/.solver/config.json` |
| `SOLVER_PORT`        | No         | Port on which to host the healthcheck endpoint                                      | `8080`                                   | `8080`                  |

## Running locally
- Start two anvil blockchains (note: block time is necessary or chain state can get confused):
  ```bash
  $ anvil --port 1337 --chain-id 1337 --block-time 3
  $ anvil --port 1338 --chain-id 1338 --block-time 3
  ```

- Deploy the contracts from onlysubs-solidity by:
  - `cd` into [onlysubs-solidity](./onlysubs-solidity)
  - create an env file there with:
    ```
    PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
    BLS_PUBLIC_KEY_X0=17445541620214498517833872661220947475697073327136585274784354247720096233162
    BLS_PUBLIC_KEY_X1=18268991875563357240413244408004758684187086817233527689475815128036446189503
    BLS_PUBLIC_KEY_Y0=11401601170172090472795479479864222172123705188644469125048759621824127399516
    BLS_PUBLIC_KEY_Y1=8044854403167346152897273335539146380878155193886184396711544300199836788154
    ```
  - `source .env`
  - `$ forge script script/DeployAllContracts.s.sol --broadcast --rpc-url http://127.0.0.1:1337 --private-key $PRIVATE_KEY` 
  - `$ forge script script/DeployAllContracts.s.sol --broadcast --rpc-url http://127.0.0.1:1338 --private-key $PRIVATE_KEY`
 
- Run the agent configured with the [local config](./config-local.json) and the second anvil key:
`$ cargo run -- --config-file ./config-local.json --private-key 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d`
 
- request a swap a monitor the logs for it happening:
`$ ./request-swap.sh 1337 1338`
 
- you can also swap in the other direction:
`$ ./request-swap.sh 1338 1337`
