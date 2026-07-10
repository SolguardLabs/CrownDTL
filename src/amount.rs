use crate::error::{CrownError, CrownResult};
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Amount(u128);

pub type BasisPoints = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    numerator: u128,
    denominator: u128,
}

impl Amount {
    pub const ZERO: Amount = Amount(0);
    pub const ONE: Amount = Amount(1);

    pub fn new(value: u128) -> Self {
        Amount(value)
    }

    pub fn from_u64(value: u64) -> Self {
        Amount(value as u128)
    }

    pub fn raw(self) -> u128 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn non_zero(self, label: &str) -> CrownResult<Self> {
        if self.is_zero() {
            Err(CrownError::InvalidAmount(format!(
                "{label} must be non-zero"
            )))
        } else {
            Ok(self)
        }
    }

    pub fn checked_add(self, rhs: Amount) -> CrownResult<Amount> {
        self.0
            .checked_add(rhs.0)
            .map(Amount)
            .ok_or_else(|| CrownError::arithmetic("amount addition overflow"))
    }

    pub fn checked_sub(self, rhs: Amount) -> CrownResult<Amount> {
        self.0
            .checked_sub(rhs.0)
            .map(Amount)
            .ok_or_else(|| CrownError::arithmetic("amount subtraction underflow"))
    }

    pub fn checked_mul(self, rhs: u128) -> CrownResult<Amount> {
        self.0
            .checked_mul(rhs)
            .map(Amount)
            .ok_or_else(|| CrownError::arithmetic("amount multiplication overflow"))
    }

    pub fn checked_div(self, rhs: u128) -> CrownResult<Amount> {
        if rhs == 0 {
            return Err(CrownError::arithmetic("division by zero"));
        }
        Ok(Amount(self.0 / rhs))
    }

    pub fn checked_mul_bps(self, bps: BasisPoints) -> CrownResult<Amount> {
        self.checked_mul(bps as u128)?.checked_div(10_000)
    }

    pub fn checked_mul_rate(self, rate: Rate) -> CrownResult<Amount> {
        if rate.denominator == 0 {
            return Err(CrownError::arithmetic("rate denominator is zero"));
        }
        self.checked_mul(rate.numerator)?
            .checked_div(rate.denominator)
    }

    pub fn saturating_add(self, rhs: Amount) -> Amount {
        Amount(self.0.saturating_add(rhs.0))
    }

    pub fn saturating_sub(self, rhs: Amount) -> Amount {
        Amount(self.0.saturating_sub(rhs.0))
    }

    pub fn min(self, rhs: Amount) -> Amount {
        if self <= rhs {
            self
        } else {
            rhs
        }
    }

    pub fn max(self, rhs: Amount) -> Amount {
        if self >= rhs {
            self
        } else {
            rhs
        }
    }

    pub fn as_u64(self) -> CrownResult<u64> {
        u64::try_from(self.0).map_err(|_| CrownError::arithmetic("amount does not fit u64"))
    }

    pub fn format_units(self, decimals: u8) -> String {
        if decimals == 0 {
            return self.0.to_string();
        }
        let scale = 10u128.pow(decimals as u32);
        let whole = self.0 / scale;
        let fraction = self.0 % scale;
        let mut tail = format!("{fraction:0width$}", width = decimals as usize);
        while tail.ends_with('0') && tail.len() > 1 {
            tail.pop();
        }
        format!("{whole}.{tail}")
    }

    pub fn checked_sum(values: impl IntoIterator<Item = Amount>) -> CrownResult<Amount> {
        let mut total = Amount::ZERO;
        for value in values {
            total = total.checked_add(value)?;
        }
        Ok(total)
    }
}

impl Rate {
    pub fn new(numerator: u128, denominator: u128) -> CrownResult<Self> {
        if denominator == 0 {
            return Err(CrownError::arithmetic("rate denominator is zero"));
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub fn numerator(self) -> u128 {
        self.numerator
    }

    pub fn denominator(self) -> u128 {
        self.denominator
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Amount) -> Self::Output {
        Amount(self.0 + rhs.0)
    }
}

impl Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Amount) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Amount) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Amount) {
        self.0 -= rhs.0;
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount::from_u64(value)
    }
}

impl From<u128> for Amount {
    fn from(value: u128) -> Self {
        Amount::new(value)
    }
}

impl Display for Amount {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedAmount {
    magnitude: Amount,
    negative: bool,
}

impl SignedAmount {
    pub fn positive(value: Amount) -> Self {
        Self {
            magnitude: value,
            negative: false,
        }
    }

    pub fn negative(value: Amount) -> Self {
        Self {
            magnitude: value,
            negative: true,
        }
    }

    pub fn zero() -> Self {
        Self::positive(Amount::ZERO)
    }

    pub fn magnitude(self) -> Amount {
        self.magnitude
    }

    pub fn is_negative(self) -> bool {
        self.negative && !self.magnitude.is_zero()
    }

    pub fn apply_to(self, value: Amount) -> CrownResult<Amount> {
        if self.is_negative() {
            value.checked_sub(self.magnitude)
        } else {
            value.checked_add(self.magnitude)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bps_rounds_down() {
        assert_eq!(
            Amount::from(10_001_u64).checked_mul_bps(100).unwrap().raw(),
            100
        );
    }

    #[test]
    fn signed_delta_applies() {
        let base = Amount::from(100_u64);
        assert_eq!(
            SignedAmount::positive(Amount::from(5_u64))
                .apply_to(base)
                .unwrap()
                .raw(),
            105
        );
        assert_eq!(
            SignedAmount::negative(Amount::from(5_u64))
                .apply_to(base)
                .unwrap()
                .raw(),
            95
        );
    }
}
