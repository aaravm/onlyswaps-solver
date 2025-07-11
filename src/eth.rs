use alloy::sol;

sol!(
    #[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[sol(rpc)]
    ERC20Token,
    "onlysubs-solidity/out/ERC20Token.sol/ERC20Token.json"
);

sol!(
    #[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[sol(rpc)]
    Router,
    "onlysubs-solidity/out/Router.sol/Router.json",
);
