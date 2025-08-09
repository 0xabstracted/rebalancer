use anchor_lang::prelude::*;

use crate::errors::RebalancerErrorCode;

#[account]
#[derive(Debug)]
pub struct Portfolio {
    pub manager: Pubkey,                    // 32 bytes - Portfolio manager authority
    pub total_capital_moved: u64,           // 8 bytes - Lifetime capital rebalanced (lamports)
    pub last_rebalance: i64,                // 8 bytes - Unix timestamp of last rebalance
    pub min_rebalance_interval: i64,        // 8 bytes - Minimum seconds between rebalances
    pub portfolio_creation: i64,            // 8 bytes - Portfolio creation timestamp
    pub total_strategies: u32,              // 4 bytes - Current strategy count
    pub performance_fee_bps: u16,           // 2 bytes - Performance fee in basis points
    pub base_threshold: u8,                 // 1 byte - Base threshold for dynamic calculation (1-50)
    pub emergency_pause: bool,              // 1 byte - Emergency stop flag
    pub bump: u8,                           // 1 byte - PDA bump seed
    pub reserved: [u8; 31],                 // 31 bytes - Future expansion buffer
}
// Total: 136 bytes

impl Portfolio {
    pub const MAX_SIZE: usize = 8 
    + 32 // manager
    + 8 // total_capital_moved
    + 8 // last_rebalance
    + 8 // min_rebalance_interval
    + 8 // portfolio_creation
    + 4 // total_strategies
    + 2 // performance_fee_bps
    + 1 // rebalance_threshold
    + 1 // emergency_pause
    + 1 // bump
    + 31; // reserved
    // 112 bytes
    pub fn validate_base_threshold(threshold: u8) -> Result<()> {
        require!(threshold >= 1 && threshold <= 50, RebalancerErrorCode::InvalidRebalanceThreshold);
        Ok(())
    }
    
    pub fn can_rebalance(&self, current_time: i64) -> bool {
        !self.emergency_pause && 
        current_time >= self.last_rebalance.saturating_add(self.min_rebalance_interval)
    }
    
    pub fn validate_min_interval(interval: i64) -> Result<()> {
        require!(interval >= 1 && interval <= 86400, RebalancerErrorCode::InvalidRebalanceInterval);
        Ok(())
    }
}