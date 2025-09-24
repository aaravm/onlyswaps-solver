# OnlySwaps Competitive Solver Demo

This demo showcases multiple competing solvers in the OnlySwaps protocol, demonstrating how Dutch auctions work with different threshold strategies.

## Overview

The system now runs **3 competing solvers** instead of 1, each with different auction threshold strategies:

- **AggressiveSolver** (Solver #1): 5.0x multiplier - Executes at 20% down from auction start price
- **ModerateSolver** (Solver #2): 3.0x multiplier - Executes at 33% down from auction start price  
- **ConservativeSolver** (Solver #3): 2.0x multiplier - Executes at 50% down from auction start price

## How Dutch Auctions Work

In a Dutch auction, the price starts **high** and decreases over time:
- **Start Price**: Highest acceptable price (based on slippage tolerance)
- **Current Price**: Decreases over time as auction progresses
- **Reserve Price**: Minimum acceptable price (minAllowedCost)
- **Execution Threshold**: Calculated as percentage down from start price

## Competitive Behavior

The solver with the **highest threshold wins** because:
1. Higher multiplier = willing to execute earlier (closer to start price)
2. Earlier execution means higher profit margins for users
3. First solver to accept executes the trade and blocks others

**Expected Winner Order:**
1. 🥇 AggressiveSolver (5.0x = 20% down) - Wins most auctions by executing early
2. 🥈 ModerateSolver (3.0x = 33% down) - Wins when Aggressive is busy/slow
3. 🥉 ConservativeSolver (2.0x = 50% down) - Wins only when others miss opportunities

## Processing Speed Advantage

Each solver also has different processing speeds:
- **AggressiveSolver**: 0ms delay (fastest processing)
- **ModerateSolver**: 50ms delay 
- **ConservativeSolver**: 100ms delay (simulates real-world differences)

## Setup and Running

### 1. Build and Deploy Contracts
```bash
cd onlyswaps-docker
./build-chains.sh
```

### 2. Deploy Contracts on Local Chains
```bash
./deploy-anvil.sh
```

### 3. Configure Solvers (funds addresses and mints tokens)
```bash
./configure-solver.sh
```

### 4. Start All Services (3 solvers + 2 anvil chains)
```bash
docker compose up --build
```

### 5. Run Demo Swap
```bash
# Request swap from chain 31337 to 43113
./demo-competitive-swap.sh 31337 43113
```

## Monitoring the Competition

Watch solver logs to see the competition:
```bash
docker compose logs -f solver_1 solver_2 solver_3
```

### Key Log Messages to Look For:

**Auction Creation:**
```
🚀 Started slippage-based Dutch auction for request [...] on DESTINATION chain
```

**Solver Decision Making:**
```
💰 Solver 'AggressiveSolver#1' Auction [...] - Current price: 95000, MinAllowedCost: 99000, Threshold (3.0x): 97000, Execute: true
💰 Solver 'ModerateSolver#2' Auction [...] - Current price: 95000, MinAllowedCost: 99000, Threshold (2.5x): 97500, Execute: false  
```

**Winner Execution:**
```
✅ Solver 'AggressiveSolver#1' executing trade [...] at price 95000
```

## Configuration

### Solver Ports
- AggressiveSolver: http://localhost:8080
- ModerateSolver: http://localhost:8081  
- ConservativeSolver: http://localhost:8082

### Private Keys Used
- Solver 1: `0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6`
- Solver 2: `0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d`
- Solver 3: `0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a`

### Customizing Thresholds

To use custom thresholds, modify the solver config JSON file and add:
```json
{
  "networks": [...],
  "solver_config": {
    "threshold_multiplier": 2.8,
    "solver_name": "CustomSolver"
  }
}
```

Or use different private keys and solver IDs in docker-compose.yml.

## Real-World Implications

This competitive model mirrors real DeFi solver networks where:
- **Higher profit margins** = more aggressive bidding
- **Gas costs** vs **profit opportunities** create natural competition
- **Speed of execution** becomes crucial for profitability
- **Risk tolerance** varies between solver operators

The AggressiveSolver trades higher costs for guaranteed execution, while ConservativeSolver maximizes profit margins but risks losing opportunities.