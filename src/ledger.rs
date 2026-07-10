use crate::amount::Amount;
use crate::clock::EpochDay;
use crate::error::{CrownError, CrownResult};
use crate::events::{Event, EventKind};
use crate::ids::{AccountId, AssetId, IdAllocator, JournalId, RedemptionId, VaultId, WindowId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Journal {
    events: Vec<Event>,
}

impl Journal {
    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn last(&self) -> Option<&Event> {
        self.events.last()
    }

    pub fn by_kind(&self, kind: EventKind) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| event.kind() == &kind)
            .collect()
    }

    pub fn by_account(&self, account: AccountId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| event.account() == Some(account))
            .collect()
    }

    pub fn by_redemption(&self, redemption: RedemptionId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| event.redemption() == Some(redemption))
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LedgerTotals {
    vault_reserves: BTreeMap<VaultId, Amount>,
    vault_shares: BTreeMap<VaultId, Amount>,
    account_assets: BTreeMap<AssetId, Amount>,
    account_shares: BTreeMap<VaultId, Amount>,
    open_claims: BTreeMap<VaultId, Amount>,
}

impl LedgerTotals {
    pub fn set_vault_reserve(&mut self, vault: VaultId, amount: Amount) {
        self.vault_reserves.insert(vault, amount);
    }

    pub fn set_vault_shares(&mut self, vault: VaultId, amount: Amount) {
        self.vault_shares.insert(vault, amount);
    }

    pub fn add_account_asset(&mut self, asset: AssetId, amount: Amount) -> CrownResult<()> {
        let current = self.account_assets.get(&asset).copied().unwrap_or_default();
        self.account_assets
            .insert(asset, current.checked_add(amount)?);
        Ok(())
    }

    pub fn add_account_shares(&mut self, vault: VaultId, amount: Amount) -> CrownResult<()> {
        let current = self.account_shares.get(&vault).copied().unwrap_or_default();
        self.account_shares
            .insert(vault, current.checked_add(amount)?);
        Ok(())
    }

    pub fn add_open_claim(&mut self, vault: VaultId, amount: Amount) -> CrownResult<()> {
        let current = self.open_claims.get(&vault).copied().unwrap_or_default();
        self.open_claims.insert(vault, current.checked_add(amount)?);
        Ok(())
    }

    pub fn vault_reserve(&self, vault: VaultId) -> Amount {
        self.vault_reserves.get(&vault).copied().unwrap_or_default()
    }

    pub fn vault_shares(&self, vault: VaultId) -> Amount {
        self.vault_shares.get(&vault).copied().unwrap_or_default()
    }

    pub fn account_shares(&self, vault: VaultId) -> Amount {
        self.account_shares.get(&vault).copied().unwrap_or_default()
    }

    pub fn open_claims(&self, vault: VaultId) -> Amount {
        self.open_claims.get(&vault).copied().unwrap_or_default()
    }

    pub fn total_account_assets(&self) -> CrownResult<Amount> {
        Amount::checked_sum(self.account_assets.values().copied())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProtocolLedger {
    allocator: IdAllocator,
    journal: Journal,
}

impl ProtocolLedger {
    pub fn allocator(&self) -> &IdAllocator {
        &self.allocator
    }

    pub fn allocator_mut(&mut self) -> &mut IdAllocator {
        &mut self.allocator
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    pub fn next_account(&mut self) -> AccountId {
        self.allocator.account()
    }

    pub fn next_asset(&mut self) -> AssetId {
        self.allocator.asset()
    }

    pub fn next_vault(&mut self) -> VaultId {
        self.allocator.vault()
    }

    pub fn next_redemption(&mut self) -> RedemptionId {
        self.allocator.redemption()
    }

    pub fn next_window(&mut self) -> WindowId {
        self.allocator.window()
    }

    pub fn next_journal(&mut self) -> JournalId {
        self.allocator.journal()
    }

    pub fn event(&mut self, day: EpochDay, kind: EventKind) -> crate::events::EventBuilder {
        let id = self.next_journal();
        Event::builder(id, day, kind)
    }

    pub fn record(&mut self, event: Event) {
        self.journal.push(event);
    }

    pub fn record_simple(&mut self, day: EpochDay, kind: EventKind, note: impl Into<String>) {
        let event = self.event(day, kind).note(note).build();
        self.record(event);
    }

    pub fn assert_share_conservation(
        &mut self,
        day: EpochDay,
        totals: &LedgerTotals,
    ) -> CrownResult<()> {
        for (vault, shares) in &totals.vault_shares {
            let account_shares = totals.account_shares(*vault);
            if *shares != account_shares {
                return Err(CrownError::Invariant(format!(
                    "vault {vault} shares {shares} differ from account shares {account_shares}"
                )));
            }
        }
        self.record_simple(day, EventKind::InvariantChecked, "share conservation");
        Ok(())
    }

    pub fn assert_claim_coverage(
        &mut self,
        day: EpochDay,
        totals: &LedgerTotals,
    ) -> CrownResult<()> {
        for (vault, reserve) in &totals.vault_reserves {
            let claims = totals.open_claims(*vault);
            if *reserve < claims {
                return Err(CrownError::Invariant(format!(
                    "vault {vault} reserve {reserve} below open claims {claims}"
                )));
            }
        }
        self.record_simple(day, EventKind::InvariantChecked, "claim coverage");
        Ok(())
    }
}
