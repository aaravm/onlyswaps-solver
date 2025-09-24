#!/usr/bin/env bash

# enables transfers between one chain and another, and maps
# the token address between the two chains
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

echo "[+] funding solvers"
# Fund all three solver addresses
cast send --value 10ether 0xa0Ee7A142d267C1f36714E4a8F75612F20a79720 --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:31337
cast send --value 10ether 0xa0Ee7A142d267C1f36714E4a8F75612F20a79720 --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:43113

# Second solver address (derived from private key 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d)
cast send --value 10ether 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:31337
cast send --value 10ether 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:43113

# Third solver address (derived from private key 0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a)
cast send --value 10ether 0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:31337
cast send --value 10ether 0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:43113

echo "[+] building solver configs"

# the addresses should be the same on both chains
RUSD_ADDRESS=$(jq -r '.transactions | .[] | select(.contractName=="ERC20FaucetToken") | .contractAddress' $SCRIPT_DIR/onlyswaps-solidity/broadcast/DeployAllContracts.s.sol/31337/run-latest.json)
ROUTER_ADDRESS=$(jq -r '.transactions | .[] | select(.contractName=="UUPSProxy") | .contractAddress' $SCRIPT_DIR/onlyswaps-solidity/broadcast/DeployAllContracts.s.sol/31337/run-latest.json | head -n1)

# Generate shared solver config (all solvers will use same network config)
RUSD_ADDRESS=$RUSD_ADDRESS ROUTER_ADDRESS=$ROUTER_ADDRESS envsubst < solver-config-template.json > build/solver-config.json

echo "[+] minting tokens for all solvers"
# Private keys for the three solvers
SOLVER_1_KEY="0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6"
SOLVER_2_KEY="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"  
SOLVER_3_KEY="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"

# Mint tokens on both chains for all solvers
for chain_port in 31337 43113; do
    echo "Minting tokens on chain $chain_port for all solvers..."
    cast send $RUSD_ADDRESS "mint()" --rpc-url "http://127.0.0.1:$chain_port" --private-key $SOLVER_1_KEY
    cast send $RUSD_ADDRESS "mint()" --rpc-url "http://127.0.0.1:$chain_port" --private-key $SOLVER_2_KEY
    cast send $RUSD_ADDRESS "mint()" --rpc-url "http://127.0.0.1:$chain_port" --private-key $SOLVER_3_KEY
done

echo "Solver configs generated successfully with:"
echo "  RUSD_ADDRESS=$RUSD_ADDRESS"
echo "  ROUTER_ADDRESS=$ROUTER_ADDRESS"
echo "  AggressiveSolver (threshold: 3.0x) - port 8080"
echo "  ModerateSolver (threshold: 2.5x) - port 8081"  
echo "  ConservativeSolver (threshold: 2.0x) - port 8082"
