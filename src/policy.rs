use crate::accounts::AccountTier;
use crate::amount::Amount;
use crate::clock::EpochDay;
use crate::error::{CrownError, CrownResult};
use crate::ids::{AccountId, VaultId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyLimitPolicy {
    standard_limit: Amount,
    vip_limit: Amount,
    institutional_limit: Amount,
}

impl DailyLimitPolicy {
    pub fn new(
        standard_limit: Amount,
        vip_limit: Amount,
        institutional_limit: Amount,
    ) -> CrownResult<Self> {
        standard_limit.non_zero("standard daily limit")?;
        vip_limit.non_zero("vip daily limit")?;
        institutional_limit.non_zero("institutional daily limit")?;
        if standard_limit > vip_limit || vip_limit > institutional_limit {
            return Err(CrownError::InvalidPolicy(
                "daily limits must be ordered by tier".to_owned(),
            ));
        }
        Ok(Self {
            standard_limit,
            vip_limit,
            institutional_limit,
        })
    }

    pub fn for_tier(self, tier: AccountTier) -> Amount {
        match tier {
            AccountTier::Standard => self.standard_limit,
            AccountTier::Vip => self.vip_limit,
            AccountTier::Institutional => self.institutional_limit,
        }
    }

    pub fn standard_limit(self) -> Amount {
        self.standard_limit
    }

    pub fn vip_limit(self) -> Amount {
        self.vip_limit
    }

    pub fn institutional_limit(self) -> Amount {
        self.institutional_limit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityPolicy {
    daily_capacity: Amount,
    per_user_priority_limit: Amount,
    priority_unlock_delay: u64,
    standard_unlock_delay: u64,
    max_queue_depth: usize,
    priority_fee_bps: u32,
}

impl PriorityPolicy {
    pub fn new(
        daily_capacity: Amount,
        per_user_priority_limit: Amount,
        priority_unlock_delay: u64,
        standard_unlock_delay: u64,
        max_queue_depth: usize,
        priority_fee_bps: u32,
    ) -> CrownResult<Self> {
        daily_capacity.non_zero("daily priority capacity")?;
        per_user_priority_limit.non_zero("per user priority limit")?;
        if per_user_priority_limit > daily_capacity {
            return Err(CrownError::InvalidPolicy(
                "per user priority limit exceeds daily capacity".to_owned(),
            ));
        }
        if priority_unlock_delay > standard_unlock_delay {
            return Err(CrownError::InvalidPolicy(
                "priority delay must not exceed standard delay".to_owned(),
            ));
        }
        if max_queue_depth == 0 {
            return Err(CrownError::InvalidPolicy(
                "queue depth must be non-zero".to_owned(),
            ));
        }
        if priority_fee_bps > 1_000 {
            return Err(CrownError::InvalidPolicy(
                "priority fee exceeds protocol maximum".to_owned(),
            ));
        }
        Ok(Self {
            daily_capacity,
            per_user_priority_limit,
            priority_unlock_delay,
            standard_unlock_delay,
            max_queue_depth,
            priority_fee_bps,
        })
    }

    pub fn daily_capacity(self) -> Amount {
        self.daily_capacity
    }

    pub fn per_user_priority_limit(self) -> Amount {
        self.per_user_priority_limit
    }

    pub fn priority_unlock_delay(self) -> u64 {
        self.priority_unlock_delay
    }

    pub fn standard_unlock_delay(self) -> u64 {
        self.standard_unlock_delay
    }

    pub fn max_queue_depth(self) -> usize {
        self.max_queue_depth
    }

    pub fn priority_fee_bps(self) -> u32 {
        self.priority_fee_bps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedemptionPolicy {
    daily_limits: DailyLimitPolicy,
    priority: PriorityPolicy,
    minimum_ticket: Amount,
    max_ticket: Amount,
}

impl RedemptionPolicy {
    pub fn new(
        daily_limits: DailyLimitPolicy,
        priority: PriorityPolicy,
        minimum_ticket: Amount,
        max_ticket: Amount,
    ) -> CrownResult<Self> {
        minimum_ticket.non_zero("minimum ticket")?;
        if max_ticket < minimum_ticket {
            return Err(CrownError::InvalidPolicy(
                "max ticket below minimum".to_owned(),
            ));
        }
        Ok(Self {
            daily_limits,
            priority,
            minimum_ticket,
            max_ticket,
        })
    }

    pub fn daily_limits(self) -> DailyLimitPolicy {
        self.daily_limits
    }

    pub fn priority(self) -> PriorityPolicy {
        self.priority
    }

    pub fn minimum_ticket(self) -> Amount {
        self.minimum_ticket
    }

    pub fn max_ticket(self) -> Amount {
        self.max_ticket
    }

    pub fn validate_amount(self, amount: Amount) -> CrownResult<()> {
        amount.non_zero("redemption amount")?;
        if amount < self.minimum_ticket {
            return Err(CrownError::InvalidAmount(format!(
                "amount {amount} below minimum {}",
                self.minimum_ticket
            )));
        }
        if amount > self.max_ticket {
            return Err(CrownError::InvalidAmount(format!(
                "amount {amount} above maximum {}",
                self.max_ticket
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LimitKey {
    account: AccountId,
    vault: VaultId,
    day: EpochDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LimitBucket {
    requested: Amount,
    released: Amount,
    completed: Amount,
}

impl LimitBucket {
    pub fn requested(self) -> Amount {
        self.requested
    }

    pub fn released(self) -> Amount {
        self.released
    }

    pub fn completed(self) -> Amount {
        self.completed
    }

    pub fn active(self) -> CrownResult<Amount> {
        self.requested.checked_sub(self.released)
    }

    pub fn consume(&mut self, amount: Amount) -> CrownResult<()> {
        self.requested = self.requested.checked_add(amount)?;
        Ok(())
    }

    pub fn release(&mut self, amount: Amount) -> CrownResult<()> {
        let active = self.active()?;
        if active < amount {
            return Err(CrownError::Invariant(format!(
                "limit release {amount} exceeds active {active}"
            )));
        }
        self.released = self.released.checked_add(amount)?;
        Ok(())
    }

    pub fn complete(&mut self, amount: Amount) -> CrownResult<()> {
        self.completed = self.completed.checked_add(amount)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct LimitLedger {
    buckets: BTreeMap<LimitKey, LimitBucket>,
}

impl LimitLedger {
    pub fn consume(
        &mut self,
        account: AccountId,
        vault: VaultId,
        tier: AccountTier,
        day: EpochDay,
        amount: Amount,
        policy: DailyLimitPolicy,
    ) -> CrownResult<()> {
        let key = LimitKey {
            account,
            vault,
            day,
        };
        let current = self.bucket(account, vault, day).active()?;
        let limit = policy.for_tier(tier);
        let next = current.checked_add(amount)?;
        if next > limit {
            return Err(CrownError::LimitExceeded(format!(
                "daily limit {limit} exceeded by projected {next}"
            )));
        }
        self.buckets.entry(key).or_default().consume(amount)?;
        Ok(())
    }

    pub fn release(
        &mut self,
        account: AccountId,
        vault: VaultId,
        day: EpochDay,
        amount: Amount,
    ) -> CrownResult<()> {
        let key = LimitKey {
            account,
            vault,
            day,
        };
        self.buckets.entry(key).or_default().release(amount)
    }

    pub fn complete(
        &mut self,
        account: AccountId,
        vault: VaultId,
        day: EpochDay,
        amount: Amount,
    ) -> CrownResult<()> {
        let key = LimitKey {
            account,
            vault,
            day,
        };
        self.buckets.entry(key).or_default().complete(amount)
    }

    pub fn bucket(&self, account: AccountId, vault: VaultId, day: EpochDay) -> LimitBucket {
        self.buckets
            .get(&LimitKey {
                account,
                vault,
                day,
            })
            .copied()
            .unwrap_or_default()
    }

    pub fn active_for(
        &self,
        account: AccountId,
        vault: VaultId,
        day: EpochDay,
    ) -> CrownResult<Amount> {
        self.bucket(account, vault, day).active()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CapacityKey {
    vault: VaultId,
    day: EpochDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapacityBucket {
    consumed: Amount,
    released: Amount,
}

impl CapacityBucket {
    pub fn consumed(self) -> Amount {
        self.consumed
    }

    pub fn released(self) -> Amount {
        self.released
    }

    pub fn net(self) -> CrownResult<Amount> {
        self.consumed.checked_sub(self.released)
    }

    pub fn available(self, capacity: Amount) -> CrownResult<Amount> {
        capacity.checked_sub(self.net()?)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapacityLedger {
    buckets: BTreeMap<CapacityKey, CapacityBucket>,
}

impl CapacityLedger {
    pub fn consume(
        &mut self,
        vault: VaultId,
        day: EpochDay,
        amount: Amount,
        policy: PriorityPolicy,
    ) -> CrownResult<()> {
        let key = CapacityKey { vault, day };
        let bucket = self.buckets.entry(key).or_default();
        let available = bucket.available(policy.daily_capacity())?;
        if available < amount {
            return Err(CrownError::QueueCapacity(format!(
                "priority capacity {available} below request {amount}"
            )));
        }
        bucket.consumed = bucket.consumed.checked_add(amount)?;
        Ok(())
    }

    pub fn release(&mut self, vault: VaultId, day: EpochDay, amount: Amount) -> CrownResult<()> {
        let key = CapacityKey { vault, day };
        let bucket = self.buckets.entry(key).or_default();
        let net = bucket.net()?;
        if net < amount {
            return Err(CrownError::Invariant(format!(
                "capacity release {amount} exceeds net {net}"
            )));
        }
        bucket.released = bucket.released.checked_add(amount)?;
        Ok(())
    }

    pub fn bucket(&self, vault: VaultId, day: EpochDay) -> CapacityBucket {
        self.buckets
            .get(&CapacityKey { vault, day })
            .copied()
            .unwrap_or_default()
    }

    pub fn available(
        &self,
        vault: VaultId,
        day: EpochDay,
        policy: PriorityPolicy,
    ) -> CrownResult<Amount> {
        self.bucket(vault, day).available(policy.daily_capacity())
    }
}
