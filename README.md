# OnlySwaps Solver

OnlySwaps Solver is a competitive multi-chain swap solver built during Loops Hacker House. It implements a Dutch auction mechanism where multiple solvers compete to execute cross-chain swaps, with MEV protection using drand randomness.

## Overview

This project creates a decentralized solver network where:
- **Multiple solvers compete** for swap execution rights through Dutch auctions
- **Aggressive solvers win** by offering higher threshold values in the auction
- **MEV protection** is provided through drand randomness integration
- **Cross-chain swaps** are executed efficiently between different blockchain networks
- **Docker-based deployment** allows easy scaling and management of solver instances

## Architecture

### Dutch Auction Mechanism
The core innovation is a Dutch auction system where:
- Solvers submit bids with threshold values
- Higher threshold (more aggressive) solvers win the right to execute swaps
- Auction prices decrease over time until a solver accepts
- This ensures competitive pricing and efficient execution

### Multi-Solver Competition
- Multiple solver instances run simultaneously in Docker containers
- Each solver competes independently for swap opportunities
- The most aggressive solver (highest threshold) wins the auction
- Failed solvers automatically retry with adjusted parameters

### MEV Protection
- Integration with drand (Distributed Randomness Beacon) for unpredictable timing
- Randomness prevents front-running and sandwich attacks
- Ensures fair competition among solvers without predictable patterns

### Cross-Chain Support
- Supports swaps between multiple blockchain networks
- Demo includes chains 31337 and 43113
- Extensible architecture for additional chain integrations

## Key Features

- **Competitive Dutch Auctions**: Multiple solvers bid for swap execution rights
- **MEV-Resistant Design**: Drand randomness prevents predictable MEV extraction
- **Multi-Chain Architecture**: Execute swaps across different blockchain networks
- **Docker Containerization**: Easy deployment and scaling of solver instances
- **Real-time Competition**: Live auction monitoring and competitive bidding
- **Automatic Retry Logic**: Failed auctions automatically retry with adjusted parameters
- **Health Monitoring**: Built-in healthcheck endpoints for system monitoring

## How It Works

1. **Swap Request**: A user initiates a cross-chain swap request
2. **Auction Start**: Multiple solvers detect the opportunity and enter a Dutch auction
3. **Competitive Bidding**: Solvers submit threshold values (higher = more aggressive)
4. **Winner Selection**: The solver with the highest threshold wins the auction
5. **MEV Protection**: Drand randomness ensures unpredictable execution timing
6. **Swap Execution**: The winning solver executes the cross-chain swap
7. **Settlement**: The swap is completed and settled on both chains

## Demo Workflow

The included demo script demonstrates competitive solver behavior:
- Multiple solver instances compete for the same swap opportunity
- Auction logs show real-time bidding and threshold competition
- Transaction "skipping" indicates active auction competition
- The most aggressive solver ultimately wins and executes the swap

## Build
First retrieve and build submodules with `./build-dependencies.sh` then run `cargo build`

## Test
`cargo test`

## Configuration
| Environment Variable | Mandatory? | Description                                                                         | Example                                  | Default                 |
| -------------------- | ---------- | ----------------------------------------------------------------------------------- | ---------------------------------------- |-------------------------|
| `SOLVER_PRIVATE_KEY` | Yes        | A hex-encoded private key, with or without the `0x`                                 | `0xdeadbeefdeadbeefdeadbeefdeadbeefdead` | —                       |
| `SOLVER_CONFIG_PATH` | No         | Path to your solver configuration JSON (must match format in `config_default.json`) | `/data/config.json`                      | `~/.solver/config.json` |
| `SOLVER_PORT`        | No         | Port on which to host the healthcheck endpoint                                      | `8080`                                   | `8080`                  |

## Running locally
- Go to the `onlyswaps-docker` directory and run the `build-chains.sh` file.
  ```bash
  $ ./build-chains.sh
  ```

- Build the docker image and initiate solvers inside docker container.
```bash
$ sudo docker compose build
$ sudo docker compose up -d
```
- Now you can interact with the solvers at the localhost endpoints.
- All the logs from solvers side can be viewed using `sudo docker compose up -d`.


- We have already created a file to make solvers compete in the dutch auction and the solver with aggressive threshold would win:
  - Start the competitive swap: `./demo-competitive-swap.sh 31337 43113`
  - Go to your docker compose logs and you can view the auction steps.
  - Once you notice transactions are `skipping`, that means the auction is actively running with multiple solvers competing
  - Request additional swaps: `./request-swap.sh 43113 31337`

## Technical Stack

- **Rust**: Core solver implementation
- **Docker**: Containerized deployment and scaling
- **Multi-chain RPC**: Cross-chain communication
- **Drand**: Distributed randomness for MEV protection
- **Dutch Auction Protocol**: Competitive pricing mechanism

## Built at Loops Hacker House

This project was developed during the Loops Hacker House hackathon, focusing on creating a competitive, MEV-resistant cross-chain swap solver using innovative Dutch auction mechanisms.

