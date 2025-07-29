use anchor_lang::prelude::*;

#[account]
#[derive(Debug)]
pub struct CapitalPosition {
    pub strategy_id: Pubkey,                // 32 bytes - Reference to strategy
    pub token_a_amount: u64,                // 8 bytes - Token A quantity
    pub token_b_amount: u64,                // 8 bytes - Token B quantity (0 for single asset)
    pub lp_tokens: u64,                     // 8 bytes - LP tokens held
    pub platform_controlled_lp: u64,       // 8 bytes - LP tokens under platform control
    pub entry_price_a: u64,                 // 8 bytes - Entry price token A (6 decimals)
    pub entry_price_b: u64,                 // 8 bytes - Entry price token B (6 decimals)
    pub last_rebalance: i64,                // 8 bytes - Last position update
    pub accrued_fees: u64,                  // 8 bytes - Accumulated fees in position
    pub impermanent_loss: i64,              // 8 bytes - IL tracking (can be negative)
    pub position_type: PositionType,        // 1 byte - Position classification
    pub bump: u8,                           // 1 byte - PDA bump seed
    pub reserved: [u8; 14],                 // 14 bytes - Future expansion
}
// Total: 145 bytes

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub enum PositionType {
    SingleAsset,
    LiquidityPair,
    StakedPosition,
}

impl CapitalPosition {
    pub const MAX_SIZE: usize = 8 
    + 32 // strategy_id
    + 8 // token_a_amount
    + 8 // token_b_amount
    + 8 // lp_tokens
    + 8 // platform_controlled_lp
    + 8 // entry_price_a
    + 8 // entry_price_b
    + 8 // last_rebalance
    + 8 // accrued_fees
    + 8 // impermanent_loss
    + 1 // position_type
    + 1 // bump
    + 14; // reserved 
    // 128 bytes
}