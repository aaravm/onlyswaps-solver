#!/usr/bin/env bash

# enables transfers between one chain and another, and maps
# the token address between the two chains
set -euo pipefail

if [ "$#" -ne "2" ]; then
  echo "Usage: $0 <src-chain-id> <dest-chain-id>"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
# this export is a weird quirk that forge needs
export PRIVATE_KEY="$PRIVATE_KEY"
RPC_URL=http://127.0.0.1:$1

RUSD_ADDRESS=$(jq -r '.transactions | .[] | select(.contractName=="ERC20FaucetToken") | .contractAddress' $SCRIPT_DIR/onlyswaps-solidity/broadcast/DeployAllContracts.s.sol/$1/run-latest.json)
ROUTER_ADDRESS=$(jq -r '.transactions | .[] | select(.contractName=="UUPSProxy") | .contractAddress' $SCRIPT_DIR/onlyswaps-solidity/broadcast/DeployAllContracts.s.sol/$1/run-latest.json | head -n1)

export ROUTER_SRC_ADDRESS="$ROUTER_ADDRESS"
export ERC20_SRC_ADDRESS="$RUSD_ADDRESS"
export ERC20_DST_ADDRESS="$RUSD_ADDRESS"
export DST_CHAIN_ID="$2"

pushd onlyswaps-solidity
echo "[+] configuring Router for $1"
forge script script/onlyswaps/utils/ConfigureRouterScript.s.sol:ConfigureRouterScript --broadcast --rpc-url $RPC_URL --private-key $PRIVATE_KEY --force
