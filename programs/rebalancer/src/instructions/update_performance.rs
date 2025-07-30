use anchor_lang::prelude::*;
use crate::state::*;
use crate::errors::RebalancerErrorCode;

#[derive(Accounts)]
#[instruction(strategy_id: Pubkey)]
pub struct UpdatePerformance<'info> {
    #[account(
        mut,
        seeds = [b"portfolio", portfolio.manager.as_ref()],
        bump = portfolio.bump,
        has_one = manager @ RebalancerErrorCode::UnauthorizedManager
    )]
    pub portfolio: Account<'info, Portfolio>,
    
    #[account(
        mut,
        seeds = [b"strategy", portfolio.key().as_ref(), strategy_id.as_ref()],
        bump = strategy.bump,
        constraint = strategy.strategy_id == strategy_id @ RebalancerErrorCode::StrategyNotFound
    )]
    pub strategy: Account<'info, Strategy>,
    
    #[account(mut)]
    pub manager: Signer<'info>,
}

pub fn update_performance(
    ctx: Context<UpdatePerformance>,
    _strategy_id: Pubkey,
    yield_rate: u64,
    volatility_score: u32,
    current_balance: u64,
) -> Result<()> {
    let strategy = &mut ctx.accounts.strategy;
    let current_time = Clock::get()?.unix_timestamp;
    
    // COMPREHENSIVE INPUT VALIDATIONS
    Strategy::validate_yield_rate(yield_rate)?;
    Strategy::validate_volatility_score(volatility_score)?;
    Strategy::validate_balance_update(current_balance)?;
    require!(strategy.status == StrategyStatus::Active, RebalancerErrorCode::StrategyNotFound);
    
    // UPDATE STRATEGY METRICS
    strategy.yield_rate = yield_rate;
    strategy.volatility_score = volatility_score;
    strategy.current_balance = current_balance;
    strategy.last_updated = current_time;
    
    // CALCULATE PERFORMANCE SCORE WITH WEIGHTED FORMULA
    strategy.performance_score = calculate_performance_score(
        yield_rate,
        current_balance,
        volatility_score,
    )?;
    
    msg!("Performance updated: strategy={}, yield={}bps, volatility={}, balance={}, score={}", 
         strategy.strategy_id, yield_rate, volatility_score, current_balance, strategy.performance_score);
    
    Ok(())
}

// EXACT WEIGHTED PERFORMANCE SCORING ALGORITHM
pub fn calculate_performance_score(
    yield_rate: u64,      // Annual yield in basis points (0-50000)
    balance: u64,         // Current capital allocated in lamports
    volatility: u32,      // Risk score 0-10000 (100.00% max)
) -> Result<u64> {
    // NORMALIZATION TO 0-10000 SCALE FOR EACH METRIC
    
    // Normalize yield rate: 0-50000 basis points -> 0-10000 scale
    let normalized_yield = if yield_rate > 50000 {
        10000u64
    } else {
        (yield_rate as u128 * 10000u128 / 50000u128) as u64
    };
    
    // Normalize balance: Use logarithmic scaling for better distribution
    // Range: 100M lamports (0.1 SOL) to 100B lamports (100 SOL) -> 0-10000 scale
    let normalized_balance = if balance == 0 {
        0u64
    } else if balance >= 100_000_000_000u64 { // 100 SOL cap
        10000u64
    } else if balance < 100_000_000u64 { // 0.1 SOL minimum
        (balance as u128 * 1000u128 / 100_000_000u128) as u64 // Linear below minimum
    } else {
        // Logarithmic scaling between 0.1 and 100 SOL
        let log_balance = ((balance as f64).ln() * 1000.0) as u64;
        let log_min = ((100_000_000f64).ln() * 1000.0) as u64;
        let log_max = ((100_000_000_000f64).ln() * 1000.0) as u64;
        
        if log_max > log_min {
            ((log_balance.saturating_sub(log_min) as u128 * 10000u128) / 
             (log_max - log_min) as u128) as u64
        } else {
            5000u64 // Fallback to median if calculation fails
        }
    };
    
    // Normalize inverse volatility: 0-10000 volatility -> 10000-0 inverse scale
    let normalized_inverse_volatility = 10000u32.saturating_sub(volatility.min(10000)) as u64;
    
    // WEIGHTED COMPOSITE CALCULATION: Yield(45%) + Balance(35%) + InverseVolatility(20%)
    let yield_component = normalized_yield
        .checked_mul(4500)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?
        .checked_div(10000)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?;
    
    let balance_component = normalized_balance
        .checked_mul(3500)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?
        .checked_div(10000)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?;
    
    let volatility_component = normalized_inverse_volatility
        .checked_mul(2000)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?
        .checked_div(10000)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?;
    
    // FINAL COMPOSITE SCORE
    let performance_score = yield_component
        .checked_add(balance_component)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?
        .checked_add(volatility_component)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?;
    
    Ok(performance_score)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_performance_score_calculation() {
        // Test case 1: High yield, high balance, low volatility (best case)
        let score1 = calculate_performance_score(
            20000,        // 200% yield
            50_000_000_000, // 50 SOL
            1000,         // 10% volatility
        ).unwrap();
        
        // Test case 2: Low yield, low balance, high volatility (worst case)
        let score2 = calculate_performance_score(
            500,          // 5% yield
            100_000_000,  // 0.1 SOL
            9000,         // 90% volatility
        ).unwrap();
        
        // Score1 should be significantly higher than Score2
        assert!(score1 > score2);
        assert!(score1 <= 10000); // Within expected range
        assert!(score2 <= 10000); // Within expected range
    }
    
    #[test]
    fn test_edge_cases() {
        // Zero balance
        let score_zero = calculate_performance_score(10000, 0, 5000).unwrap();
        assert_eq!(score_zero, 5000); // Should only get yield + volatility components
        
        // Maximum values
        let score_max = calculate_performance_score(50000, 100_000_000_000, 0).unwrap();
        assert_eq!(score_max, 10000); // Perfect score
        
        // Minimum values  
        let score_min = calculate_performance_score(0, 100_000_000, 10000).unwrap();
        assert!(score_min < 5000); // Low score as expected
    }
}