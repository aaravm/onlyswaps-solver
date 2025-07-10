use crate::model::Trade;

pub(crate) struct TradeExecutor {
    
}

impl TradeExecutor {
    pub fn new() -> Self {
        Self {
            
        }
    }
    pub fn execute(&self, trades: Vec<Trade>) {
        println!("gr8 trades m8 {:?}", trades);
    }
}