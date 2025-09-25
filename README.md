# onlyswaps-solver

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
  - Once you notice transactions are `skipping`, that mean
`$ ./request-swap.sh 43113 31337`
