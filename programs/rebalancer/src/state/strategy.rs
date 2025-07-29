use anchor_lang::prelude::*;
use crate::errors::RebalancerErrorCode;

#[account]
#[derive(Debug)]
pub struct Strategy {
    pub strategy_id: Pubkey,                // 32 bytes - Unique strategy identifier
    pub current_balance: u64,               // 8 bytes - Current capital allocated (lamports)
    pub yield_rate: u64,                    // 8 bytes - Annual yield in basis points (0-50000)
    pub performance_score: u64,             // 8 bytes - Calculated composite score
    pub total_deposits: u64,                // 8 bytes - Lifetime deposits tracking
    pub total_withdrawals: u64,             // 8 bytes - Lifetime withdrawals tracking
    pub protocol_type: ProtocolType,        // Variable size - Protocol-specific data
    pub volatility_score: u32,              // 4 bytes - Risk metric (0-10000, 100.00% max)
    pub last_updated: i64,                  // 8 bytes - Last metric update timestamp
    pub creation_time: i64,                 // 8 bytes - Strategy creation timestamp
    pub status: StrategyStatus,             // 1 byte - Current strategy status
    pub percentile_rank: u8,                // 1 byte - 0-100 ranking position
    pub bump: u8,                           // 1 byte - PDA bump seed
    pub reserved: [u8; 29],                 // 21 bytes - Future expansion
}
// Total: ~144 bytes + protocol_type size

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub enum ProtocolType {
    StableLending { 
        pool_id: Pubkey,                    // 32 bytes - Solend pool identifier
        reserve_address: Pubkey,            // 32 bytes - Reserve account address
        utilization: u16,                   // 2 bytes - Pool utilization in basis points
    },  // 66 bytes total
    YieldFarming { 
        pair_id: Pubkey,                    // 32 bytes - Orca pair identifier
        token_a_mint: Pubkey,               // 32 bytes - Token A mint address
        token_b_mint: Pubkey,               // 32 bytes - Token B mint address
        fee_tier: u16,                      // 2 bytes - Pool fee in basis points
        reward_multiplier: u8,              // 1 byte - Reward boost (1-10x)
    },  // 99 bytes total
    LiquidStaking { 
        validator_id: Pubkey,               // 32 bytes - Marinade validator
        stake_pool: Pubkey,                 // 32 bytes - Stake pool address
        unstake_delay: u32,                 // 4 bytes - Unstaking delay in epochs
        commission: u16,                    // 2 bytes - Validator commission (basis points)
    },  // 70 bytes total
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum StrategyStatus {
    Active,      // Normal operation, participates in rebalancing
    Paused,      // Temporarily disabled, no new allocations
    Deprecated,  // Marked for removal, extract capital when possible
}

impl Strategy {
    pub const MAX_SIZE: usize = 8 
    + 32 // strategy_id
    + 8 // current_balance
    + 8 // yield_rate
    + 8 // performance_score
    + 8 // total_deposits
    + 8 // total_withdrawals
    + 100 // protocol_type
    + 4 // volatility_score
    + 8 // last_updated
    + 8 // creation_time
    + 1 // status
    + 1 // percentile_rank
    + 1 // bump
    + 29; // reserved
    // 232 bytes
    
    pub fn validate_yield_rate(rate: u64) -> Result<()> {
        require!(rate <= 50000, RebalancerErrorCode::ExcessiveYieldRate);
        Ok(())
    }
    
    pub fn validate_balance_update(new_balance: u64) -> Result<()> {
        require!(new_balance < u64::MAX / 1000, RebalancerErrorCode::BalanceOverflow);
        Ok(())
    }
    
    pub fn validate_volatility_score(score: u32) -> Result<()> {
        require!(score <= 10000, RebalancerErrorCode::InvalidVolatilityScore);
        Ok(())
    }
}

impl ProtocolType {
    pub fn validate(&self) -> Result<()> {
        match self {
            ProtocolType::StableLending { pool_id, utilization, reserve_address } => {
                require!(*pool_id != Pubkey::default(), RebalancerErrorCode::InvalidPoolId);
                require!(*reserve_address != Pubkey::default(), RebalancerErrorCode::InvalidReserveAddress);
                require!(*utilization <= 10000, RebalancerErrorCode::InvalidUtilization);
                Ok(())
            },
            ProtocolType::YieldFarming { 
                pair_id, reward_multiplier, token_a_mint, token_b_mint, fee_tier 
            } => {
                require!(*pair_id != Pubkey::default(), RebalancerErrorCode::InvalidPairId);
                require!(*token_a_mint != Pubkey::default(), RebalancerErrorCode::InvalidTokenMint);
                require!(*token_b_mint != Pubkey::default(), RebalancerErrorCode::InvalidTokenMint);
                require!(*token_a_mint != *token_b_mint, RebalancerErrorCode::DuplicateTokenMints);
                require!(*reward_multiplier >= 1 && *reward_multiplier <= 10, RebalancerErrorCode::InvalidRewardMultiplier);
                require!(*fee_tier <= 1000, RebalancerErrorCode::InvalidFeeTier);
                Ok(())
            },
            ProtocolType::LiquidStaking { 
                validator_id, commission, stake_pool, unstake_delay 
            } => {
                require!(*validator_id != Pubkey::default(), RebalancerErrorCode::InvalidValidatorId);
                require!(*stake_pool != Pubkey::default(), RebalancerErrorCode::InvalidStakePool);
                require!(*commission <= 1000, RebalancerErrorCode::InvalidCommission);
                require!(*unstake_delay <= 50, RebalancerErrorCode::InvalidUnstakeDelay);
                Ok(())
            },
        }
    }
    
    pub fn get_protocol_name(&self) -> &'static str {
        match self {
            ProtocolType::StableLending { .. } => "Stable Lending",
            ProtocolType::YieldFarming { .. } => "Yield Farming",
            ProtocolType::LiquidStaking { .. } => "Liquid Staking",
        }
    }

    pub fn get_expected_tokens(&self) -> Vec<Pubkey> {
        match self {
            ProtocolType::StableLending { reserve_address, .. } => {
                vec![*reserve_address]
            },
            ProtocolType::YieldFarming { token_a_mint, token_b_mint, .. } => {
                vec![*token_a_mint, *token_b_mint]
            },
            ProtocolType::LiquidStaking { stake_pool, .. } => {
                vec![*stake_pool]
            },
        }
    }
    
    pub fn validate_balance_constraints(&self, balance: u64) -> Result<()> {
        match self {
            ProtocolType::StableLending { .. } => {
                // Minimum 0.1 SOL for lending protocols
                require!(balance >= 100_000_000, RebalancerErrorCode::InsufficientBalance);
            },
            ProtocolType::YieldFarming { .. } => {
                // Minimum 0.5 SOL for LP positions (gas + slippage)
                require!(balance >= 500_000_000, RebalancerErrorCode::InsufficientBalance);
            },
            ProtocolType::LiquidStaking { .. } => {
                // Minimum 1 SOL for staking (epoch requirements)
                require!(balance >= 1_000_000_000, RebalancerErrorCode::InsufficientBalance);
            },
        }
        Ok(())
    }
}