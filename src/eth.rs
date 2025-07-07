use alloy::sol;

sol!(
    #[sol(rpc)]
    ERC20Token,
    "onlysubs-solidity/out/ERC20Token.sol/ERC20Token.json"
);

sol!(
    #[sol(rpc)]
    Router,
    "onlysubs-solidity/out/IRouter.sol/IRouter.json",
);