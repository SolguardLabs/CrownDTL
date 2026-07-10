use crate::amount::Amount;
use crate::clock::EpochDay;
use crate::ids::{AccountId, AssetId, JournalId, RedemptionId, VaultId, WindowId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    AccountRegistered,
    AssetRegistered,
    VaultOpened,
    SharesMinted,
    RedemptionRequested,
    RedemptionQueued,
    RedemptionCancelled,
    RedemptionScheduled,
    WithdrawalCompleted,
    LimitConsumed,
    LimitReleased,
    CapacityConsumed,
    CapacityReleased,
    WindowProcessed,
    InvariantChecked,
}

impl EventKind {
    pub fn label(&self) -> &'static str {
        match self {
            EventKind::AccountRegistered => "account_registered",
            EventKind::AssetRegistered => "asset_registered",
            EventKind::VaultOpened => "vault_opened",
            EventKind::SharesMinted => "shares_minted",
            EventKind::RedemptionRequested => "redemption_requested",
            EventKind::RedemptionQueued => "redemption_queued",
            EventKind::RedemptionCancelled => "redemption_cancelled",
            EventKind::RedemptionScheduled => "redemption_scheduled",
            EventKind::WithdrawalCompleted => "withdrawal_completed",
            EventKind::LimitConsumed => "limit_consumed",
            EventKind::LimitReleased => "limit_released",
            EventKind::CapacityConsumed => "capacity_consumed",
            EventKind::CapacityReleased => "capacity_released",
            EventKind::WindowProcessed => "window_processed",
            EventKind::InvariantChecked => "invariant_checked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    id: JournalId,
    day: EpochDay,
    kind: EventKind,
    account: Option<AccountId>,
    vault: Option<VaultId>,
    asset: Option<AssetId>,
    redemption: Option<RedemptionId>,
    window: Option<WindowId>,
    amount: Option<Amount>,
    note: String,
}

impl Event {
    pub fn builder(id: JournalId, day: EpochDay, kind: EventKind) -> EventBuilder {
        EventBuilder {
            event: Event {
                id,
                day,
                kind,
                account: None,
                vault: None,
                asset: None,
                redemption: None,
                window: None,
                amount: None,
                note: String::new(),
            },
        }
    }

    pub fn id(&self) -> JournalId {
        self.id
    }

    pub fn day(&self) -> EpochDay {
        self.day
    }

    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    pub fn account(&self) -> Option<AccountId> {
        self.account
    }

    pub fn vault(&self) -> Option<VaultId> {
        self.vault
    }

    pub fn asset(&self) -> Option<AssetId> {
        self.asset
    }

    pub fn redemption(&self) -> Option<RedemptionId> {
        self.redemption
    }

    pub fn window(&self) -> Option<WindowId> {
        self.window
    }

    pub fn amount(&self) -> Option<Amount> {
        self.amount
    }

    pub fn note(&self) -> &str {
        &self.note
    }
}

pub struct EventBuilder {
    event: Event,
}

impl EventBuilder {
    pub fn account(mut self, id: AccountId) -> Self {
        self.event.account = Some(id);
        self
    }

    pub fn vault(mut self, id: VaultId) -> Self {
        self.event.vault = Some(id);
        self
    }

    pub fn asset(mut self, id: AssetId) -> Self {
        self.event.asset = Some(id);
        self
    }

    pub fn redemption(mut self, id: RedemptionId) -> Self {
        self.event.redemption = Some(id);
        self
    }

    pub fn window(mut self, id: WindowId) -> Self {
        self.event.window = Some(id);
        self
    }

    pub fn amount(mut self, amount: Amount) -> Self {
        self.event.amount = Some(amount);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.event.note = note.into();
        self
    }

    pub fn build(self) -> Event {
        self.event
    }
}
