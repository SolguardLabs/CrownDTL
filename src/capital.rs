use crate::error::{CrownError, CrownResult};
use std::collections::BTreeSet;

pub const BPS: u128 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapitalInput {
    pub vault: String,
    pub reserve_assets: u128,
    pub liquid_assets: u128,
    pub total_shares: u128,
    pub open_claims: u128,
    pub priority_capacity: u128,
    pub reserve_haircut_bps: u128,
    pub claim_shock_bps: u128,
    pub operational_buffer_bps: u128,
    pub unlock_days: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapitalMetrics {
    pub vault: String,
    pub effective_reserve: u128,
    pub stressed_claims: u128,
    pub operational_buffer: u128,
    pub required_reserve: u128,
    pub reserve_shortfall: u128,
    pub coverage_bps: u128,
    pub liquidity_bps: u128,
    pub claim_share_bps: u128,
    pub available_priority_capacity: u128,
    pub priority_utilization_bps: u128,
    pub unlock_days: u64,
    pub compliant: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioMetrics {
    pub vaults: Vec<CapitalMetrics>,
    pub total_reserve: u128,
    pub total_required_reserve: u128,
    pub total_shortfall: u128,
    pub coverage_bps: u128,
    pub claim_hhi_bps: u128,
    pub largest_claim_concentration_bps: u128,
    pub weighted_unlock_days: u128,
    pub compliant_vaults: usize,
    pub compliant: bool,
}

impl CapitalInput {
    pub fn validate(&self) -> CrownResult<()> {
        if self.vault.trim().is_empty() || self.vault != self.vault.trim() {
            return Err(CrownError::Invariant(
                "vault identifier must be normalized".to_owned(),
            ));
        }
        if self.liquid_assets > self.reserve_assets {
            return Err(CrownError::Invariant(
                "liquid assets exceed reserve".to_owned(),
            ));
        }
        if self.reserve_haircut_bps > BPS {
            return Err(CrownError::Invariant(
                "reserve haircut exceeds 10000 bps".to_owned(),
            ));
        }
        if self.total_shares == 0 {
            return Err(CrownError::Invariant(
                "total shares must be positive".to_owned(),
            ));
        }
        Ok(())
    }
}

pub fn evaluate_route(input: &CapitalInput) -> CrownResult<CapitalMetrics> {
    input.validate()?;
    let effective_reserve = mul_div_floor(
        input.reserve_assets,
        BPS.checked_sub(input.reserve_haircut_bps)
            .ok_or_else(|| CrownError::arithmetic("haircut underflow"))?,
        BPS,
    )?;
    let stressed_claims = mul_div_ceil(
        input.open_claims,
        BPS.checked_add(input.claim_shock_bps)
            .ok_or_else(|| CrownError::arithmetic("claim shock overflow"))?,
        BPS,
    )?;
    let operational_buffer = mul_div_ceil(input.total_shares, input.operational_buffer_bps, BPS)?;
    let required_reserve = stressed_claims
        .checked_add(operational_buffer)
        .ok_or_else(|| CrownError::arithmetic("required reserve overflow"))?;
    let available_reserve = effective_reserve.min(input.liquid_assets);
    let reserve_shortfall = required_reserve.saturating_sub(available_reserve);
    let coverage_bps = ratio_bps(effective_reserve, required_reserve)?;
    let liquidity_bps = ratio_bps(input.liquid_assets, input.reserve_assets)?;
    let claim_share_bps = ratio_bps(input.open_claims, input.total_shares)?;
    let priority_utilization_bps = ratio_bps_ceil(input.open_claims, input.priority_capacity)?;
    let available_priority_capacity = input
        .priority_capacity
        .min(available_reserve.saturating_sub(required_reserve));
    Ok(CapitalMetrics {
        vault: input.vault.clone(),
        effective_reserve,
        stressed_claims,
        operational_buffer,
        required_reserve,
        reserve_shortfall,
        coverage_bps,
        liquidity_bps,
        claim_share_bps,
        available_priority_capacity,
        priority_utilization_bps,
        unlock_days: input.unlock_days,
        compliant: reserve_shortfall == 0,
    })
}

pub fn evaluate_portfolio(inputs: &[CapitalInput]) -> CrownResult<PortfolioMetrics> {
    if inputs.is_empty() {
        return Err(CrownError::Invariant(
            "portfolio requires a vault".to_owned(),
        ));
    }
    let mut metrics = PortfolioMetrics {
        vaults: Vec::with_capacity(inputs.len()),
        total_reserve: 0,
        total_required_reserve: 0,
        total_shortfall: 0,
        coverage_bps: 0,
        claim_hhi_bps: 0,
        largest_claim_concentration_bps: 0,
        weighted_unlock_days: 0,
        compliant_vaults: 0,
        compliant: false,
    };
    let mut total_claims = 0u128;
    let mut weighted_unlock = 0u128;
    let mut vaults = BTreeSet::new();
    for input in inputs {
        if !vaults.insert(input.vault.as_str()) {
            return Err(CrownError::Invariant(
                "portfolio contains a duplicate vault".to_owned(),
            ));
        }
        let route = evaluate_route(input)?;
        metrics.total_reserve = checked_add(metrics.total_reserve, route.effective_reserve)?;
        metrics.total_required_reserve =
            checked_add(metrics.total_required_reserve, route.required_reserve)?;
        metrics.total_shortfall = checked_add(metrics.total_shortfall, route.reserve_shortfall)?;
        total_claims = checked_add(total_claims, input.open_claims)?;
        weighted_unlock = checked_add(
            weighted_unlock,
            input
                .open_claims
                .checked_mul(input.unlock_days as u128)
                .ok_or_else(|| CrownError::arithmetic("weighted unlock overflow"))?,
        )?;
        if route.compliant {
            metrics.compliant_vaults += 1;
        }
        metrics.vaults.push(route);
    }
    metrics
        .vaults
        .sort_by(|left, right| left.vault.cmp(&right.vault));
    metrics.coverage_bps = ratio_bps(metrics.total_reserve, metrics.total_required_reserve)?;
    if let Some(weighted_unlock_days) = weighted_unlock.checked_div(total_claims) {
        metrics.weighted_unlock_days = weighted_unlock_days;
        for input in inputs {
            let share = ratio_bps(input.open_claims, total_claims)?;
            metrics.claim_hhi_bps =
                checked_add(metrics.claim_hhi_bps, mul_div_floor(share, share, BPS)?)?;
            metrics.largest_claim_concentration_bps =
                metrics.largest_claim_concentration_bps.max(share);
        }
    }
    metrics.compliant = metrics.total_shortfall == 0 && metrics.compliant_vaults == inputs.len();
    Ok(metrics)
}

fn checked_add(left: u128, right: u128) -> CrownResult<u128> {
    left.checked_add(right)
        .ok_or_else(|| CrownError::arithmetic("capital aggregation overflow"))
}

fn ratio_bps(numerator: u128, denominator: u128) -> CrownResult<u128> {
    if denominator == 0 {
        return Ok(if numerator == 0 { 0 } else { BPS });
    }
    mul_div_floor(numerator, BPS, denominator)
}

fn ratio_bps_ceil(numerator: u128, denominator: u128) -> CrownResult<u128> {
    if denominator == 0 {
        return Ok(0);
    }
    mul_div_ceil(numerator, BPS, denominator)
}

fn mul_div_floor(value: u128, numerator: u128, denominator: u128) -> CrownResult<u128> {
    if denominator == 0 {
        return Err(CrownError::arithmetic("division by zero"));
    }
    value
        .checked_mul(numerator)
        .ok_or_else(|| CrownError::arithmetic("multiplication overflow"))
        .map(|product| product / denominator)
}

fn mul_div_ceil(value: u128, numerator: u128, denominator: u128) -> CrownResult<u128> {
    if value == 0 || numerator == 0 {
        return Ok(0);
    }
    let product = value
        .checked_mul(numerator)
        .ok_or_else(|| CrownError::arithmetic("multiplication overflow"))?;
    product
        .checked_add(denominator - 1)
        .ok_or_else(|| CrownError::arithmetic("rounding overflow"))
        .map(|rounded| rounded / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(vault: &str, claims: u128) -> CapitalInput {
        CapitalInput {
            vault: vault.to_owned(),
            reserve_assets: 1_000_000,
            liquid_assets: 800_000,
            total_shares: 900_000,
            open_claims: claims,
            priority_capacity: 300_000,
            reserve_haircut_bps: 500,
            claim_shock_bps: 2_000,
            operational_buffer_bps: 800,
            unlock_days: 2,
        }
    }

    #[test]
    fn capital_rounding_is_conservative() {
        let result = evaluate_route(&input("vault:senior", 300_000)).unwrap();
        assert_eq!(result.effective_reserve, 950_000);
        assert_eq!(result.stressed_claims, 360_000);
        assert_eq!(result.operational_buffer, 72_000);
        assert_eq!(result.required_reserve, 432_000);
        assert!(result.compliant);
    }

    #[test]
    fn portfolio_reports_claim_concentration() {
        let result = evaluate_portfolio(&[
            input("vault:senior", 750_000),
            input("vault:junior", 250_000),
        ])
        .unwrap();
        assert_eq!(result.claim_hhi_bps, 6_250);
        assert_eq!(result.largest_claim_concentration_bps, 7_500);
    }

    #[test]
    fn invalid_liquidity_fails_closed() {
        let mut value = input("vault:senior", 100_000);
        value.liquid_assets = value.reserve_assets + 1;
        assert!(evaluate_route(&value).is_err());
    }

    #[test]
    fn insufficient_liquidity_is_not_compliant() {
        let mut value = input("vault:senior", 300_000);
        value.liquid_assets = 400_000;
        let result = evaluate_route(&value).unwrap();
        assert_eq!(result.reserve_shortfall, 32_000);
        assert!(!result.compliant);
    }

    #[test]
    fn portfolio_rejects_duplicate_vaults() {
        assert!(evaluate_portfolio(&[
            input("vault:senior", 100_000),
            input("vault:senior", 200_000),
        ])
        .is_err());
    }
}
