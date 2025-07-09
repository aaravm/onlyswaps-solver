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

RUSD=0x43748DC0b4735463105c528816f59bB7F37009dE
ROUTER=0xE91fB8BBfb1C6beFC2383cDFd5E502BCA14f03a9

# mint some tokens on this chain
echo "minting tokens"
cast send $RUSD "mint(address, uint256)" $ME 10000 --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# enable transferring to other chain
echo "enabling destination chain"
cast send $ROUTER "allowDstChainId(uint256, bool)" "$OTHER_CHAIN_ID" true --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# enable token
echo "enabling token"
cast send $ROUTER "setTokenMapping(uint256, address, address)" "$OTHER_CHAIN_ID" $RUSD $RUSD --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# allow the contract to spend our erc20s
echo "approving token spend"
 cast send $RUSD "approve(address, uint256)" $ROUTER 101ether --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# send the money to the bridge
echo "making bridge request"
cast send -vvvv $ROUTER "bridge(address, uint256, uint256, uint256, address, uint256)" $RUSD 100 1 "$OTHER_CHAIN_ID" $ME $RANDOM --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY
