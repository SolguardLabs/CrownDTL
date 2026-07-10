use crate::amount::Amount;
use crate::clock::EpochDay;
use crate::error::{CrownError, CrownResult};
use crate::ids::{AccountId, AssetId, RedemptionId, VaultId, WindowId};
use crate::queue::Lane;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimState {
    Pending,
    Consumed,
    Released,
}

impl ClaimState {
    pub fn is_open(self) -> bool {
        matches!(self, ClaimState::Pending)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorityClaimDraft {
    pub redemption_id: RedemptionId,
    pub account: AccountId,
    pub vault: VaultId,
    pub asset: AssetId,
    pub amount: Amount,
    pub unlock_day: EpochDay,
    pub window: WindowId,
    pub lane: Lane,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorityClaim {
    redemption_id: RedemptionId,
    account: AccountId,
    vault: VaultId,
    asset: AssetId,
    amount: Amount,
    unlock_day: EpochDay,
    window: WindowId,
    lane: Lane,
    state: ClaimState,
}

impl PriorityClaim {
    pub fn from_draft(draft: PriorityClaimDraft) -> Self {
        Self {
            redemption_id: draft.redemption_id,
            account: draft.account,
            vault: draft.vault,
            asset: draft.asset,
            amount: draft.amount,
            unlock_day: draft.unlock_day,
            window: draft.window,
            lane: draft.lane,
            state: ClaimState::Pending,
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

    pub fn asset(&self) -> AssetId {
        self.asset
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn unlock_day(&self) -> EpochDay {
        self.unlock_day
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    pub fn lane(&self) -> Lane {
        self.lane
    }

    pub fn state(&self) -> ClaimState {
        self.state
    }

    pub fn is_mature(&self, day: EpochDay) -> bool {
        self.state.is_open() && day >= self.unlock_day
    }

    pub fn mark_consumed(&mut self) -> CrownResult<()> {
        if !self.state.is_open() {
            return Err(CrownError::InvalidStatus(format!(
                "claim {} is not pending",
                self.redemption_id
            )));
        }
        self.state = ClaimState::Consumed;
        Ok(())
    }

    pub fn mark_released(&mut self) -> CrownResult<()> {
        if !self.state.is_open() {
            return Err(CrownError::InvalidStatus(format!(
                "claim {} is not pending",
                self.redemption_id
            )));
        }
        self.state = ClaimState::Released;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct PriorityBook {
    claims: BTreeMap<RedemptionId, PriorityClaim>,
}

impl PriorityBook {
    pub fn insert(&mut self, claim: PriorityClaim) -> CrownResult<()> {
        if self.claims.contains_key(&claim.redemption_id()) {
            return Err(CrownError::Invariant(format!(
                "claim {} already exists",
                claim.redemption_id()
            )));
        }
        self.claims.insert(claim.redemption_id(), claim);
        Ok(())
    }

    pub fn get(&self, redemption_id: RedemptionId) -> CrownResult<&PriorityClaim> {
        self.claims
            .get(&redemption_id)
            .ok_or_else(|| CrownError::MissingClaim(redemption_id.to_string()))
    }

    pub fn get_mut(&mut self, redemption_id: RedemptionId) -> CrownResult<&mut PriorityClaim> {
        self.claims
            .get_mut(&redemption_id)
            .ok_or_else(|| CrownError::MissingClaim(redemption_id.to_string()))
    }

    pub fn release_claim(&mut self, redemption_id: RedemptionId) -> CrownResult<Amount> {
        let claim = self.get_mut(redemption_id)?;
        let amount = claim.amount();
        claim.mark_released()?;
        Ok(amount)
    }

    pub fn consume_claim(
        &mut self,
        redemption_id: RedemptionId,
        day: EpochDay,
    ) -> CrownResult<PriorityClaim> {
        let claim = self.get(redemption_id)?.clone();
        if !claim.is_mature(day) {
            return Err(CrownError::WindowNotReady(format!(
                "claim {} unlocks at day {}",
                redemption_id,
                claim.unlock_day().raw()
            )));
        }
        self.get_mut(redemption_id)?.mark_consumed()?;
        Ok(claim)
    }

    pub fn consume_mature_for_account(
        &mut self,
        account: AccountId,
        vault: VaultId,
        day: EpochDay,
    ) -> CrownResult<Vec<PriorityClaim>> {
        let ids: Vec<RedemptionId> = self
            .claims
            .values()
            .filter(|claim| {
                claim.account() == account && claim.vault() == vault && claim.is_mature(day)
            })
            .map(|claim| claim.redemption_id())
            .collect();
        let mut out = Vec::new();
        for id in ids {
            out.push(self.consume_claim(id, day)?);
        }
        Ok(out)
    }

    pub fn pending_for_account(&self, account: AccountId, vault: VaultId) -> CrownResult<Amount> {
        let mut total = Amount::ZERO;
        for claim in self.claims.values() {
            if claim.account() == account && claim.vault() == vault && claim.state().is_open() {
                total = total.checked_add(claim.amount())?;
            }
        }
        Ok(total)
    }

    pub fn pending_for_ticket(&self, redemption_id: RedemptionId) -> Amount {
        self.claims
            .get(&redemption_id)
            .filter(|claim| claim.state().is_open())
            .map(|claim| claim.amount())
            .unwrap_or_default()
    }

    pub fn open_claims(&self) -> impl Iterator<Item = &PriorityClaim> {
        self.claims.values().filter(|claim| claim.state().is_open())
    }

    pub fn all_claims(&self) -> impl Iterator<Item = &PriorityClaim> {
        self.claims.values()
    }
}
