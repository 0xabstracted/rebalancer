use anchor_lang::prelude::*;
use crate::errors::RebalancerErrorCode;
use crate::instructions::execute_ranking::StrategyData;

/// Calculate the average volatility across all strategies
/// 
/// This function computes the average volatility score from a slice of StrategyData.
/// It uses checked arithmetic to prevent overflow and ensures the denominator is never zero.
/// 
/// # Arguments
/// * `strategies` - A slice of StrategyData containing volatility scores
/// 
/// # Returns
/// * `Result<u32>` - The average volatility score, or an error if:
///   - Strategies slice is empty
///   - Mathematical overflow occurs
/// 
/// # Mathematical Safety
/// - Uses u64 for intermediate calculations to prevent overflow
/// - Uses checked_div to prevent division by zero
/// - Ensures final result fits in u32
pub fn calculate_average_volatility(strategies: &[StrategyData]) -> Result<u32> {
    require!(!strategies.is_empty(), RebalancerErrorCode::InsufficientStrategies);
    
    let total_volatility: u64 = strategies.iter()
        .map(|s| s.volatility_score as u64)
        .sum::<u64>();
    
    // Use checked division to prevent overflow and division by zero
    let average = total_volatility.checked_div(strategies.len() as u64)
        .ok_or(RebalancerErrorCode::BalanceOverflow)?;
    
    // Ensure average fits in u32 (volatility scores are u32, so this should always be true)
    Ok(average as u32)
}

/// Calculate the dynamic threshold based on base threshold and average volatility
/// 
/// This function implements the dynamic threshold formula:
/// Dynamic Threshold = Base Threshold + Volatility Adjustment
/// where Volatility Adjustment = (Average Volatility / 100) × 20%
/// 
/// The final threshold is clamped to the range 10% - 40%.
/// 
/// # Arguments
/// * `base_threshold` - The base threshold percentage (e.g., 15 for 15%)
/// * `average_volatility` - The average volatility score across strategies
/// 
/// # Returns
/// * `Result<u8>` - The dynamic threshold percentage (10-40), or an error if:
///   - Base threshold is invalid (> 100)
///   - Mathematical overflow occurs
/// 
/// # Formula
/// Dynamic Threshold = Base Threshold + ((Average Volatility / 100) × 20)
/// Final Range: 10% minimum to 40% maximum
/// 
/// # Example
/// If base_threshold = 15 and average_volatility = 3000:
/// Volatility Adjustment = (3000 / 100) × 20 = 30 × 20 = 600
/// Dynamic Threshold = 15 + 6 = 21%
/// Final Threshold = 21% (within 10-40% range)
pub fn calculate_dynamic_threshold(
    base_threshold: u8, 
    average_volatility: u32
) -> Result<u8> {
    // Base validation
    require!(base_threshold <= 100, RebalancerErrorCode::InvalidRebalanceThreshold);
    
    // Calculate volatility adjustment: (Average Volatility / 100) × 20%
    // Note: average_volatility is expressed in basis points (0-10000 for 0-100%).
    // Therefore: adjustment = (average_volatility * 20) / 10000 → integer percent points
    // Use u64 for intermediate calculations to prevent overflow
    let volatility_adjustment = (average_volatility as u64 * 20) / 10_000;
    
    // Calculate dynamic threshold: Base + Volatility Adjustment (both in percent points)
    let dynamic_threshold = (base_threshold as u64)
        .checked_add(volatility_adjustment)
        .ok_or(RebalancerErrorCode::BalanceOverflow)? as u8;
    
    // Clamp to range: 10% minimum, 40% maximum
    Ok(dynamic_threshold.clamp(10, 40))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;
    
    #[test]
    fn test_calculate_average_volatility_normal() {
        let strategies = vec![
            StrategyData {
                strategy_id: Pubkey::new_unique(),
                performance_score: 1000,
                current_balance: 1_000_000_000,
                volatility_score: 2000,
                percentile_rank: 0,
            },
            StrategyData {
                strategy_id: Pubkey::new_unique(),
                performance_score: 2000,
                current_balance: 2_000_000_000,
                volatility_score: 3000,
                percentile_rank: 0,
            },
            StrategyData {
                strategy_id: Pubkey::new_unique(),
                performance_score: 3000,
                current_balance: 3_000_000_000,
                volatility_score: 4000,
                percentile_rank: 0,
            },
        ];
        
        let average = calculate_average_volatility(&strategies).unwrap();
        assert_eq!(average, 3000); // (2000 + 3000 + 4000) / 3 = 3000
    }
    
    #[test]
    fn test_calculate_average_volatility_empty() {
        let strategies: Vec<StrategyData> = vec![];
        
        let result = calculate_average_volatility(&strategies);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RebalancerErrorCode::InsufficientStrategies.into());
    }
    
    #[test]
    fn test_calculate_average_volatility_single() {
        let strategies = vec![
            StrategyData {
                strategy_id: Pubkey::new_unique(),
                performance_score: 1000,
                current_balance: 1_000_000_000,
                volatility_score: 5000,
                percentile_rank: 0,
            },
        ];
        
        let average = calculate_average_volatility(&strategies).unwrap();
        assert_eq!(average, 5000);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_normal() {
        let base_threshold = 15;
        let average_volatility = 3000;
        
        let dynamic_threshold = calculate_dynamic_threshold(base_threshold, average_volatility).unwrap();
        
        // Volatility adjustment = (3000 / 100) × 20 = 600
        // Dynamic threshold = 15 + 6 = 21
        assert_eq!(dynamic_threshold, 21);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_low_volatility() {
        let base_threshold = 15;
        let average_volatility = 500;
        
        let dynamic_threshold = calculate_dynamic_threshold(base_threshold, average_volatility).unwrap();
        
        // Volatility adjustment = (500 / 100) × 20 = 100
        // Dynamic threshold = 15 + 1 = 16
        assert_eq!(dynamic_threshold, 16);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_high_volatility() {
        let base_threshold = 15;
        let average_volatility = 10000;
        
        let dynamic_threshold = calculate_dynamic_threshold(base_threshold, average_volatility).unwrap();
        
        // Volatility adjustment = (10000 / 100) × 20 = 2000
        // Dynamic threshold = 15 + 20 = 35
        assert_eq!(dynamic_threshold, 35);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_clamping_min() {
        let base_threshold = 15;
        let average_volatility = 100;
        
        let dynamic_threshold = calculate_dynamic_threshold(base_threshold, average_volatility).unwrap();
        
        // Volatility adjustment = (100 / 100) × 20 = 20
        // Dynamic threshold = 15 + 0 = 15
        // Should not be clamped as it's above minimum
        assert_eq!(dynamic_threshold, 15);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_clamping_max() {
        let base_threshold = 15;
        let average_volatility = 15000;
        
        let dynamic_threshold = calculate_dynamic_threshold(base_threshold, average_volatility).unwrap();
        
        // Volatility adjustment = (15000 / 100) × 20 = 3000
        // Dynamic threshold = 15 + 30 = 45
        // Should be clamped to maximum of 40
        assert_eq!(dynamic_threshold, 40);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_extreme_clamping_min() {
        let base_threshold = 5;
        let average_volatility = 100;
        
        let dynamic_threshold = calculate_dynamic_threshold(base_threshold, average_volatility).unwrap();
        
        // Volatility adjustment = (100 / 100) × 20 = 20
        // Dynamic threshold = 5 + 0 = 5
        // Should be clamped to minimum of 10
        assert_eq!(dynamic_threshold, 10);
    }
    
    #[test]
    fn test_calculate_dynamic_threshold_invalid_base() {
        let base_threshold = 150; // Invalid: > 100
        let average_volatility = 3000;
        
        let result = calculate_dynamic_threshold(base_threshold, average_volatility);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RebalancerErrorCode::InvalidRebalanceThreshold.into());
    }
}