use anchor_lang::prelude::*;
use crate::state::ProtocolType;

declare_id!("H5sewgM4P61yo75GtnbsVcevhEAVKpoRxJjsHWXoNYV7");

pub mod state;
pub mod instructions;
pub mod errors;

use instructions::*;

#[program]
pub mod rebalancer {

    use super::*;
    
    pub fn initialize_portfolio(
        ctx: Context<InitializePortfolio>,
        manager: Pubkey,
        rebalance_threshold: u8,
        min_rebalance_interval: i64,
    ) -> Result<()> {
        instructions::initialize_portfolio(ctx, manager, rebalance_threshold, min_rebalance_interval)
    }
    
    pub fn register_strategy(
        ctx: Context<RegisterStrategy>,
        strategy_id: Pubkey,
        protocol_type: ProtocolType,
        initial_balance: u64,
    ) -> Result<()> {
        instructions::register_strategy(ctx, strategy_id, protocol_type, initial_balance)
    }

    pub fn update_performance(
        ctx: Context<UpdatePerformance>,
        strategy_id: Pubkey,
        yield_rate: u64,
        volatility_score: u32,
        current_balance: u64,
    ) -> Result<()> {
        instructions::update_performance(ctx, strategy_id, yield_rate, volatility_score, current_balance)
    }

    pub fn execute_ranking_cycle(
        ctx: Context<ExecuteRankingCycle>,
    ) -> Result<()> {
        instructions::execute_ranking_cycle(ctx)
    }
}

