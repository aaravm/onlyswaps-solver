#!/bin/bash

set -euo pipefail

echo "🚀 Starting OnlySwaps Competitive Solver Demo"
echo "=============================================="
echo ""
echo "This demo shows 3 competing solvers with different auction threshold strategies:"
echo "  • AggressiveSolver (5.0x multiplier) - Executes at 20% down from start price"
echo "  • ModerateSolver (3.0x multiplier) - Executes at 33% down from start price"
echo "  • ConservativeSolver (2.0x multiplier) - Executes at 50% down from start price"
echo ""
echo "In a Dutch auction, the price starts high and decreases over time."
echo "The solver with the HIGHEST multiplier executes earliest and wins!"
echo ""

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <src_chain_id> <dst_chain_id>"
  echo "Example: $0 31337 43113"
  exit 1
fi

RPC_URL=http://localhost:$1
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
ADDRESS=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
OTHER_CHAIN_ID=$2

RUSD=0xaeeb4e7d4ff71e14079e193ce5a33402f0109c42
ROUTER=0x3aa2927492c4ee514f484691f14e72c48e2b351a

echo "📊 Solver Status Check:"
echo "----------------------"
echo "AggressiveSolver (port 8080): $(curl -s http://localhost:8080/health 2>/dev/null || echo 'Not responding')"
echo "ModerateSolver (port 8081): $(curl -s http://localhost:8081/health 2>/dev/null || echo 'Not responding')"
echo "ConservativeSolver (port 8082): $(curl -s http://localhost:8082/health 2>/dev/null || echo 'Not responding')"
echo ""

# mint some tokens on this chain (only if balance is low)
echo "💰 Checking token balance..."
BALANCE_RAW=$(cast call $RUSD "balanceOf(address)(uint256)" $ADDRESS --rpc-url "$RPC_URL")
BALANCE=$(echo $BALANCE_RAW | cut -d' ' -f1)
BALANCE_HUMAN=$(cast to-unit $BALANCE ether)
echo "Current balance: $BALANCE_HUMAN RUSD"

if [ $(echo "$BALANCE_HUMAN < 100" | bc -l) -eq 1 ]; then
    echo "Balance is low, minting tokens..."
    cast send $RUSD "mint()" --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY
else
    echo "Balance is sufficient, skipping mint"
fi

# enable transferring to other chain
echo ""
echo "⚙️  Configuring cross-chain transfer..."
cast send $ROUTER "permitDestinationChainId(uint256)" "$OTHER_CHAIN_ID" --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

# enable token
echo "🔗 Checking token mapping..."
TOKEN_MAPPED=$(cast call $ROUTER "isDstTokenMapped(address, uint256, address)(bool)" $RUSD "$OTHER_CHAIN_ID" $RUSD --rpc-url "$RPC_URL")
if [ "$TOKEN_MAPPED" = "false" ]; then
    echo "Creating token mapping..."
    cast send $ROUTER "setTokenMapping(uint256, address, address)" "$OTHER_CHAIN_ID" $RUSD $RUSD --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY
else
    echo "Token mapping already exists"
fi

# allow the contract to spend our erc20s
echo "✅ Approving token spend..."
cast send $RUSD "approve(address, uint256)" $ROUTER 101ether --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

echo ""
echo "🎯 Initiating Cross-Chain Swap (Dutch Auction)..."
echo "================================================"
echo "Amount: 100 RUSD"
echo "Slippage: 0.01 (1%)"
echo "From: Chain $1"
echo "To: Chain $OTHER_CHAIN_ID"
echo ""
echo "⏰ Watch the solver logs to see which one wins the auction!"
echo "   The AggressiveSolver should win with its 5.0x multiplier (20% down threshold)."
echo ""

# send the money to the bridge
cast send -vvvv $ROUTER "requestCrossChainSwap(address, address, uint256, uint256, uint256, address)" $RUSD $RUSD 100ether 100 "$OTHER_CHAIN_ID" $ADDRESS --rpc-url "$RPC_URL" --private-key $PRIVATE_KEY

echo ""
echo "✨ Swap request submitted! Check the solver container logs to see the competition:"
echo "   docker compose logs -f solver_1 solver_2 solver_3"
echo ""
echo "🏆 Expected winner: AggressiveSolver (highest multiplier = earliest execution)"