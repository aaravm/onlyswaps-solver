#!/usr/bin/env bash
# deploys the contracts from onlysubs-solidity to a locally running anvil instance with
# one of the default private keys

set -euo pipefail

if [ "$#" -ne "1" ]; then
  echo "Usage: $0 <port>"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RPC_URL=http://127.0.0.1:$1
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
# this re-export is a weird quirk that forge seems to need
export PRIVATE_KEY="$PRIVATE_KEY"
export DEFAULT_CONTRACT_ADMIN="$PRIVATE_KEY"

pushd $SCRIPT_DIR/onlyswaps-solidity
forge script script/onlyswaps/DeployAllContracts.s.sol:DeployAllContracts --broadcast --rpc-url $RPC_URL --private-key $PRIVATE_KEY --force

