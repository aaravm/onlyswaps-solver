use alloy::sol;

sol!(
    #[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[sol(rpc)]
    ERC20FaucetToken,
    "onlysubs-solidity/out/ERC20FaucetToken.sol/ERC20FaucetToken.json"
);

sol!(
    #[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[sol(rpc)]
    Router,
    "onlysubs-solidity/out/Router.sol/Router.json",
);
