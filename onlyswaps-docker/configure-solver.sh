#!/usr/bin/env bash

# enables transfers between one chain and another, and maps
# the token address between the two chains
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

echo "[+] funding solver"
cast send --value 10ether 0xa0Ee7A142d267C1f36714E4a8F75612F20a79720 --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:31337
cast send --value 10ether 0xa0Ee7A142d267C1f36714E4a8F75612F20a79720 --private-key $PRIVATE_KEY --rpc-url http://127.0.0.1:43113

echo "[+] building solver config"

# the addresses should be the same on both chains
RUSD_ADDRESS=$(jq -r '.transactions | .[] | select(.contractName=="ERC20FaucetToken") | .contractAddress' $SCRIPT_DIR/onlyswaps-solidity/broadcast/DeployAllContracts.s.sol/31337/run-latest.json)
ROUTER_ADDRESS=$(jq -r '.transactions | .[] | select(.contractName=="UUPSProxy") | .contractAddress' $SCRIPT_DIR/onlyswaps-solidity/broadcast/DeployAllContracts.s.sol/31337/run-latest.json | head -n1)

RUSD_ADDRESS=$RUSD_ADDRESS ROUTER_ADDRESS=$ROUTER_ADDRESS envsubst < solver-config-template.json > build/solver-config.json
