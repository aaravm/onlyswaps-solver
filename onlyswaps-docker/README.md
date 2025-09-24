# onlyswaps-docker

This project allows you to build a local docker setup with the latest version of onlyswaps for testing or fiddling purposes

## Prequisites
- git
- Docker (naturally)
- [npm](https://www.youtube.com/watch?v=E4WlUXrJgy4)
- [foundry](https://getfoundry.sh)

## Quickstart
- Build the necessary state  
`./build-chains.sh`

- Run the corresponding docker zoo 
`docker compose up -d`

You should now be able to run onlyswaps verifiers and UIs until your heart's content!

## Other Info
- [build](./build) is the artifact output directory and will be cleared every run
