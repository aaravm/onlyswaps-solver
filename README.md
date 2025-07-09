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
- Start two anvil blockchains:
`$ anvil --port 1337 --chain-id 1337`
`$ anvil --port 1338 --chain-id 1338`

- Deploy the contracts from onlysubs-solidity by running
`$ cd onlysubs-solidity && forge script script/DeployAllContracts.s.sol --broadcast --rpc-url http://127.0.0.1:1337 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
` then
`$ forge script script/DeployAllContracts.s.sol --broadcast --rpc-url http://127.0.0.1:1338 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80`
 
- Run the agent with your config (for each of )