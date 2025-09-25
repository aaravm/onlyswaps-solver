#!/bin/bash

set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <src_chain_id> <dst_chain_id>"
  exit 1
fi

RPC_URL=http://localhost:$1
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
ADDRESS=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
OTHER_CHAIN_ID=$2

RUSD=0xaeeb4e7d4ff71e14079e193ce5a33402f0109c42
ROUTER=0x3aa2927492c4ee514f484691f14e72c48e2b351a

# mint some tokens on this chain (only if balance is low)
echo "checking token balance"
BALANCE_RAW=$(cast call $RUSD "balanceOf(address)(uint256)" $ADDRESS --rpc-url "$RPC_URL")
BALANCE=$(echo $BALANCE_RAW | cut -d' ' -f1)
BALANCE_HUMAN=$(cast to-unit $BALANCE ether)
echo "Current balance: $BALANCE_HUMAN RUSD"

if [ $(echo "$BALANCE_HUMAN < 100" | bc -l) -eq 1 ]; then
    echo "Balance is low, attempting to mint tokens"
    cast send $RUSD "mint()" --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY
else
    echo "Balance is sufficient, skipping mint"
fi

# enable transferring to other chain
echo "enabling destination chain"
cast send $ROUTER "permitDestinationChainId(uint256)" "$OTHER_CHAIN_ID" --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# enable token
echo "checking if token mapping exists"
TOKEN_MAPPED=$(cast call $ROUTER "isDstTokenMapped(address, uint256, address)(bool)" $RUSD "$OTHER_CHAIN_ID" $RUSD --rpc-url "$RPC_URL")
if [ "$TOKEN_MAPPED" = "false" ]; then
    echo "enabling token mapping"
    cast send $ROUTER "setTokenMapping(uint256, address, address)" "$OTHER_CHAIN_ID" $RUSD $RUSD --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY
else
    echo "token mapping already exists, skipping"
fi

# allow the contract to spend our erc20s 
echo "approving token spend"
cast send $RUSD "approve(address, uint256)" $ROUTER 101ether --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# send the money to the bridge
echo "making bridge request"
cast send -vvvv $ROUTER "requestCrossChainSwap(address, address, uint256, uint256, uint256, address)" $RUSD $RUSD 100ether 1 "$OTHER_CHAIN_ID" $ADDRESS --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY