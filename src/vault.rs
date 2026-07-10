use crate::amount::{Amount, Rate};
use crate::clock::{EpochDay, UnlockWindow};
use crate::error::{CrownError, CrownResult};
use crate::ids::{AccountId, AssetId, RedemptionId, VaultId, WindowId};
use crate::policy::RedemptionPolicy;
use crate::queue::Lane;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedemptionKind {
    Priority,
    Standard,
}

impl RedemptionKind {
    pub fn lane(self) -> Lane {
        match self {
            RedemptionKind::Priority => Lane::Priority,
            RedemptionKind::Standard => Lane::Standard,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RedemptionKind::Priority => "priority",
            RedemptionKind::Standard => "standard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedemptionStatus {
    Queued,
    PendingUnlock,
    Cancelled,
    Withdrawn,
}

impl RedemptionStatus {
    pub fn may_cancel(self) -> bool {
        matches!(
            self,
            RedemptionStatus::Queued | RedemptionStatus::PendingUnlock
        )
    }

    pub fn may_schedule(self) -> bool {
        matches!(self, RedemptionStatus::Queued)
    }

    pub fn may_withdraw(self) -> bool {
        matches!(self, RedemptionStatus::PendingUnlock)
    }

    pub fn label(self) -> &'static str {
        match self {
            RedemptionStatus::Queued => "queued",
            RedemptionStatus::PendingUnlock => "pending_unlock",
            RedemptionStatus::Cancelled => "cancelled",
            RedemptionStatus::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedemptionTicketDraft {
    pub id: RedemptionId,
    pub account: AccountId,
    pub vault: VaultId,
    pub asset: AssetId,
    pub shares: Amount,
    pub quoted_assets: Amount,
    pub kind: RedemptionKind,
    pub requested_day: EpochDay,
    pub unlock_day: EpochDay,
    pub window: WindowId,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedemptionTicket {
    id: RedemptionId,
    account: AccountId,
    vault: VaultId,
    asset: AssetId,
    shares: Amount,
    quoted_assets: Amount,
    kind: RedemptionKind,
    status: RedemptionStatus,
    requested_day: EpochDay,
    unlock_day: EpochDay,
    window: WindowId,
    sequence: u64,
}

impl RedemptionTicket {
    pub fn from_draft(draft: RedemptionTicketDraft) -> Self {
        Self {
            id: draft.id,
            account: draft.account,
            vault: draft.vault,
            asset: draft.asset,
            shares: draft.shares,
            quoted_assets: draft.quoted_assets,
            kind: draft.kind,
            status: RedemptionStatus::Queued,
            requested_day: draft.requested_day,
            unlock_day: draft.unlock_day,
            window: draft.window,
            sequence: draft.sequence,
        }
    }

    pub fn id(&self) -> RedemptionId {
        self.id
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

    pub fn shares(&self) -> Amount {
        self.shares
    }

    pub fn quoted_assets(&self) -> Amount {
        self.quoted_assets
    }

    pub fn kind(&self) -> RedemptionKind {
        self.kind
    }

    pub fn status(&self) -> RedemptionStatus {
        self.status
    }

    pub fn requested_day(&self) -> EpochDay {
        self.requested_day
    }

    pub fn unlock_day(&self) -> EpochDay {
        self.unlock_day
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn mark_scheduled(&mut self) -> CrownResult<()> {
        if !self.status.may_schedule() {
            return Err(CrownError::InvalidStatus(format!(
                "ticket {} cannot be scheduled from {}",
                self.id,
                self.status.label()
            )));
        }
        self.status = RedemptionStatus::PendingUnlock;
        Ok(())
    }

    pub fn mark_cancelled(&mut self) -> CrownResult<()> {
        if !self.status.may_cancel() {
            return Err(CrownError::InvalidStatus(format!(
                "ticket {} cannot be cancelled from {}",
                self.id,
                self.status.label()
            )));
        }
        self.status = RedemptionStatus::Cancelled;
        Ok(())
    }

    pub fn mark_withdrawn(&mut self) -> CrownResult<()> {
        if !self.status.may_withdraw() {
            return Err(CrownError::InvalidStatus(format!(
                "ticket {} cannot withdraw from {}",
                self.id,
                self.status.label()
            )));
        }
        self.status = RedemptionStatus::Withdrawn;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultConfig {
    id: VaultId,
    name: String,
    asset: AssetId,
    policy: RedemptionPolicy,
    windows: BTreeMap<WindowId, UnlockWindow>,
}

impl VaultConfig {
    pub fn new(
        id: VaultId,
        name: impl Into<String>,
        asset: AssetId,
        policy: RedemptionPolicy,
    ) -> CrownResult<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CrownError::InvalidPolicy("vault name is empty".to_owned()));
        }
        Ok(Self {
            id,
            name,
            asset,
            policy,
            windows: BTreeMap::new(),
        })
    }

    pub fn id(&self) -> VaultId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn asset(&self) -> AssetId {
        self.asset
    }

    pub fn policy(&self) -> RedemptionPolicy {
        self.policy
    }

    pub fn add_window(&mut self, window: UnlockWindow) -> CrownResult<()> {
        if self.windows.contains_key(&window.id()) {
            return Err(CrownError::InvalidPolicy(format!(
                "window {} already configured",
                window.id()
            )));
        }
        self.windows.insert(window.id(), window);
        Ok(())
    }

    pub fn window(&self, id: WindowId) -> CrownResult<UnlockWindow> {
        self.windows
            .get(&id)
            .copied()
            .ok_or_else(|| CrownError::MissingWindow(id.to_string()))
    }

    pub fn first_window(&self) -> CrownResult<UnlockWindow> {
        self.windows
            .values()
            .next()
            .copied()
            .ok_or_else(|| CrownError::MissingWindow(self.id.to_string()))
    }

    pub fn windows(&self) -> impl Iterator<Item = UnlockWindow> + '_ {
        self.windows.values().copied()
    }
}

#[derive(Debug, Clone)]
pub struct VaultState {
    config: VaultConfig,
    reserve_assets: Amount,
    total_shares: Amount,
    pending_assets: Amount,
    tickets: BTreeMap<RedemptionId, RedemptionTicket>,
}

impl VaultState {
    pub fn new(config: VaultConfig) -> Self {
        Self {
            config,
            reserve_assets: Amount::ZERO,
            total_shares: Amount::ZERO,
            pending_assets: Amount::ZERO,
            tickets: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> VaultId {
        self.config.id()
    }

    pub fn name(&self) -> &str {
        self.config.name()
    }

    pub fn asset(&self) -> AssetId {
        self.config.asset()
    }

    pub fn policy(&self) -> RedemptionPolicy {
        self.config.policy()
    }

    pub fn config(&self) -> &VaultConfig {
        &self.config
    }

    pub fn reserve_assets(&self) -> Amount {
        self.reserve_assets
    }

    pub fn total_shares(&self) -> Amount {
        self.total_shares
    }

    pub fn pending_assets(&self) -> Amount {
        self.pending_assets
    }

    pub fn liquid_assets(&self) -> CrownResult<Amount> {
        self.reserve_assets.checked_sub(self.pending_assets)
    }

    pub fn ticket(&self, id: RedemptionId) -> CrownResult<&RedemptionTicket> {
        self.tickets
            .get(&id)
            .ok_or_else(|| CrownError::MissingTicket(id.to_string()))
    }

    pub fn ticket_mut(&mut self, id: RedemptionId) -> CrownResult<&mut RedemptionTicket> {
        self.tickets
            .get_mut(&id)
            .ok_or_else(|| CrownError::MissingTicket(id.to_string()))
    }

    pub fn tickets(&self) -> impl Iterator<Item = &RedemptionTicket> {
        self.tickets.values()
    }

    pub fn deposit_assets(&mut self, amount: Amount) -> CrownResult<()> {
        amount.non_zero("vault deposit")?;
        self.reserve_assets = self.reserve_assets.checked_add(amount)?;
        Ok(())
    }

    pub fn mint_shares(&mut self, amount: Amount) -> CrownResult<()> {
        amount.non_zero("share mint")?;
        self.total_shares = self.total_shares.checked_add(amount)?;
        Ok(())
    }

    pub fn burn_shares_for_redemption(&mut self, amount: Amount) -> CrownResult<()> {
        if self.total_shares < amount {
            return Err(CrownError::Invariant(format!(
                "vault {} shares {} below burn {}",
                self.id(),
                self.total_shares,
                amount
            )));
        }
        self.total_shares = self.total_shares.checked_sub(amount)?;
        Ok(())
    }

    pub fn restore_shares_from_redemption(&mut self, amount: Amount) -> CrownResult<()> {
        self.total_shares = self.total_shares.checked_add(amount)?;
        Ok(())
    }

    pub fn quote_redemption(&self, shares: Amount) -> CrownResult<Amount> {
        shares.non_zero("redemption shares")?;
        if self.total_shares.is_zero() {
            return Err(CrownError::InvalidAmount("vault has no shares".to_owned()));
        }
        let rate = Rate::new(self.reserve_assets.raw(), self.total_shares.raw())?;
        shares.checked_mul_rate(rate)
    }

    pub fn insert_ticket(&mut self, ticket: RedemptionTicket) -> CrownResult<()> {
        if self.tickets.contains_key(&ticket.id()) {
            return Err(CrownError::Invariant(format!(
                "ticket {} already exists",
                ticket.id()
            )));
        }
        self.pending_assets = self.pending_assets.checked_add(ticket.quoted_assets())?;
        self.tickets.insert(ticket.id(), ticket);
        Ok(())
    }

    pub fn cancel_ticket(&mut self, id: RedemptionId) -> CrownResult<RedemptionTicket> {
        let ticket = self.ticket_mut(id)?;
        ticket.mark_cancelled()?;
        let clone = ticket.clone();
        self.pending_assets = self.pending_assets.checked_sub(clone.quoted_assets())?;
        Ok(clone)
    }

    pub fn schedule_ticket(&mut self, id: RedemptionId) -> CrownResult<RedemptionTicket> {
        let ticket = self.ticket_mut(id)?;
        ticket.mark_scheduled()?;
        Ok(ticket.clone())
    }

    pub fn withdraw_ticket(
        &mut self,
        id: RedemptionId,
        day: EpochDay,
    ) -> CrownResult<RedemptionTicket> {
        let ticket = self.ticket(id)?.clone();
        if day < ticket.unlock_day() {
            return Err(CrownError::WindowNotReady(format!(
                "ticket {} unlocks at day {}",
                id,
                ticket.unlock_day().raw()
            )));
        }
        let ticket_mut = self.ticket_mut(id)?;
        ticket_mut.mark_withdrawn()?;
        self.debit_reserve(ticket.quoted_assets())?;
        self.pending_assets = self.pending_assets.checked_sub(ticket.quoted_assets())?;
        Ok(ticket)
    }

    pub fn complete_ticket_from_claim(
        &mut self,
        id: RedemptionId,
    ) -> CrownResult<Option<RedemptionTicket>> {
        let ticket = self.ticket(id)?.clone();
        if ticket.status() == RedemptionStatus::Cancelled {
            return Ok(None);
        }
        if ticket.status() == RedemptionStatus::Withdrawn {
            return Ok(None);
        }
        let ticket_mut = self.ticket_mut(id)?;
        ticket_mut.mark_withdrawn()?;
        self.pending_assets = self.pending_assets.checked_sub(ticket.quoted_assets())?;
        Ok(Some(ticket))
    }

    pub fn settle_external_claim(&mut self, amount: Amount) -> CrownResult<()> {
        self.debit_reserve(amount)
    }

    fn debit_reserve(&mut self, amount: Amount) -> CrownResult<()> {
        if self.reserve_assets < amount {
            return Err(CrownError::InsufficientAssets(format!(
                "vault {} reserve {} below withdrawal {}",
                self.id(),
                self.reserve_assets,
                amount
            )));
        }
        self.reserve_assets = self.reserve_assets.checked_sub(amount)?;
        Ok(())
    }

    pub fn share_price_hint(&self) -> CrownResult<Rate> {
        if self.total_shares.is_zero() {
            return Rate::new(1, 1);
        }
        Rate::new(self.reserve_assets.raw(), self.total_shares.raw())
    }

    pub fn status_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for ticket in self.tickets.values() {
            *counts.entry(ticket.status().label()).or_insert(0) += 1;
        }
        counts
    }
}
