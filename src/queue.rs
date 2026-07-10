use crate::accounts::AccountTier;
use crate::amount::Amount;
use crate::clock::EpochDay;
use crate::error::{CrownError, CrownResult};
use crate::ids::{AccountId, RedemptionId, Sequence, VaultId, WindowId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lane {
    Priority,
    Standard,
}

impl Lane {
    pub fn label(self) -> &'static str {
        match self {
            Lane::Priority => "priority",
            Lane::Standard => "standard",
        }
    }

    pub fn base_score(self) -> u32 {
        match self {
            Lane::Priority => 1_000,
            Lane::Standard => 100,
        }
    }

    pub fn is_priority(self) -> bool {
        matches!(self, Lane::Priority)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRequest {
    pub redemption_id: RedemptionId,
    pub account: AccountId,
    pub vault: VaultId,
    pub amount: Amount,
    pub lane: Lane,
    pub tier: AccountTier,
    pub requested_day: EpochDay,
    pub window: WindowId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    redemption_id: RedemptionId,
    account: AccountId,
    vault: VaultId,
    amount: Amount,
    lane: Lane,
    tier: AccountTier,
    requested_day: EpochDay,
    window: WindowId,
    sequence: u64,
    score: u32,
}

impl QueueItem {
    pub fn from_request(request: QueueRequest, sequence: u64) -> Self {
        let score = request.lane.base_score() + request.tier.priority_weight();
        Self {
            redemption_id: request.redemption_id,
            account: request.account,
            vault: request.vault,
            amount: request.amount,
            lane: request.lane,
            tier: request.tier,
            requested_day: request.requested_day,
            window: request.window,
            sequence,
            score,
        }
    }

    pub fn redemption_id(&self) -> RedemptionId {
        self.redemption_id
    }

    pub fn account(&self) -> AccountId {
        self.account
    }

    pub fn vault(&self) -> VaultId {
        self.vault
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn lane(&self) -> Lane {
        self.lane
    }

    pub fn tier(&self) -> AccountTier {
        self.tier
    }

    pub fn requested_day(&self) -> EpochDay {
        self.requested_day
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueSnapshot {
    vault: VaultId,
    priority_depth: usize,
    standard_depth: usize,
    total_amount: Amount,
    head: Option<RedemptionId>,
}

impl QueueSnapshot {
    pub fn vault(&self) -> VaultId {
        self.vault
    }

    pub fn priority_depth(&self) -> usize {
        self.priority_depth
    }

    pub fn standard_depth(&self) -> usize {
        self.standard_depth
    }

    pub fn total_amount(&self) -> Amount {
        self.total_amount
    }

    pub fn head(&self) -> Option<RedemptionId> {
        self.head
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueueBook {
    items: BTreeMap<VaultId, Vec<QueueItem>>,
    sequences: BTreeMap<VaultId, Sequence>,
}

impl QueueBook {
    pub fn enqueue(&mut self, request: QueueRequest, max_depth: usize) -> CrownResult<QueueItem> {
        let vault = request.vault;
        let depth = self.items.get(&vault).map_or(0, Vec::len);
        if depth >= max_depth {
            return Err(CrownError::QueueCapacity(format!(
                "vault {vault} queue depth {depth} reached"
            )));
        }
        let sequence = self.sequences.entry(vault).or_default().bump();
        let item = QueueItem::from_request(request, sequence);
        self.items.entry(vault).or_default().push(item.clone());
        Ok(item)
    }

    pub fn cancel(
        &mut self,
        vault: VaultId,
        redemption_id: RedemptionId,
    ) -> CrownResult<QueueItem> {
        let items = self
            .items
            .get_mut(&vault)
            .ok_or_else(|| CrownError::MissingTicket(redemption_id.to_string()))?;
        let index = items
            .iter()
            .position(|item| item.redemption_id() == redemption_id)
            .ok_or_else(|| CrownError::MissingTicket(redemption_id.to_string()))?;
        Ok(items.remove(index))
    }

    pub fn pop_next(&mut self, vault: VaultId) -> Option<QueueItem> {
        let items = self.items.get_mut(&vault)?;
        if items.is_empty() {
            return None;
        }
        let mut best_index = 0usize;
        for (index, item) in items.iter().enumerate().skip(1) {
            let best = &items[best_index];
            if Self::item_precedes(item, best) {
                best_index = index;
            }
        }
        Some(items.remove(best_index))
    }

    pub fn pop_batch(&mut self, vault: VaultId, limit: usize) -> Vec<QueueItem> {
        let mut out = Vec::new();
        for _ in 0..limit {
            if let Some(item) = self.pop_next(vault) {
                out.push(item);
            } else {
                break;
            }
        }
        out
    }

    pub fn peek_order(&self, vault: VaultId) -> Vec<RedemptionId> {
        let mut items = self.items.get(&vault).cloned().unwrap_or_default();
        items.sort_by(|a, b| {
            if Self::item_precedes(a, b) {
                std::cmp::Ordering::Less
            } else if Self::item_precedes(b, a) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        items.into_iter().map(|item| item.redemption_id()).collect()
    }

    pub fn snapshot(&self, vault: VaultId) -> CrownResult<QueueSnapshot> {
        let items = self.items.get(&vault).cloned().unwrap_or_default();
        let mut priority_depth = 0usize;
        let mut standard_depth = 0usize;
        let mut total_amount = Amount::ZERO;
        for item in &items {
            if item.lane().is_priority() {
                priority_depth += 1;
            } else {
                standard_depth += 1;
            }
            total_amount = total_amount.checked_add(item.amount())?;
        }
        let head = self.peek_order(vault).first().copied();
        Ok(QueueSnapshot {
            vault,
            priority_depth,
            standard_depth,
            total_amount,
            head,
        })
    }

    pub fn depth(&self, vault: VaultId) -> usize {
        self.items.get(&vault).map_or(0, Vec::len)
    }

    pub fn contains(&self, vault: VaultId, redemption_id: RedemptionId) -> bool {
        self.items
            .get(&vault)
            .map(|items| {
                items
                    .iter()
                    .any(|item| item.redemption_id() == redemption_id)
            })
            .unwrap_or(false)
    }

    fn item_precedes(left: &QueueItem, right: &QueueItem) -> bool {
        left.score() > right.score()
            || (left.score() == right.score() && left.sequence() < right.sequence())
    }
}
