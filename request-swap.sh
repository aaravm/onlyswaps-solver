#!/bin/bash

set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <src_chain_id> <dst_chain_id>"
  exit 1
fi

RPC_URL=http://localhost:$1
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
ME=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
OTHER_CHAIN_ID=$2

RUSD=0xaEeb4e7D4Ff71e14079E193CE5a33402F0109C42
ROUTER=0x3aA2927492c4Ee514f484691f14e72C48e2b351a

# mint some tokens on this chain
echo "minting tokens"
cast send $RUSD "mint()" $ME --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# enable transferring to other chain
echo "enabling destination chain"
cast send $ROUTER "permitDestinationChainId(uint256)" "$OTHER_CHAIN_ID" --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# enable token
echo "enabling token"
cast send $ROUTER "setTokenMapping(uint256, address, address)" "$OTHER_CHAIN_ID" $RUSD $RUSD --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# allow the contract to spend our erc20s
echo "approving token spend"
 cast send $RUSD "approve(address, uint256)" $ROUTER 101ether --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# send the money to the bridge
echo "making bridge request"
cast send -vvvv $ROUTER "requestCrossChainSwap(address, uint256, uint256, uint256, address)" $RUSD 100 1 "$OTHER_CHAIN_ID" $ME --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY
