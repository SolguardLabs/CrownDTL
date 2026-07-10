use crate::accounts::{Account, AccountRegistry, AccountTier};
use crate::amount::Amount;
use crate::asset::{AssetMetadata, AssetRegistry};
use crate::clock::{Clock, EpochDay, UnlockWindow, WindowKind, WindowSpec};
use crate::error::{CrownError, CrownResult};
use crate::events::EventKind;
use crate::ids::{AccountId, AssetId, RedemptionId, VaultId};
use crate::ledger::{LedgerTotals, ProtocolLedger};
use crate::policy::{
    CapacityLedger, DailyLimitPolicy, LimitLedger, PriorityPolicy, RedemptionPolicy,
};
use crate::priority::{PriorityBook, PriorityClaim, PriorityClaimDraft};
use crate::queue::{Lane, QueueBook, QueueRequest, QueueSnapshot};
use crate::vault::{
    RedemptionKind, RedemptionStatus, RedemptionTicket, RedemptionTicketDraft, VaultConfig,
    VaultState,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    default_asset_decimals: u8,
    invariant_checks: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            default_asset_decimals: 6,
            invariant_checks: true,
        }
    }
}

impl EngineConfig {
    pub fn default_asset_decimals(self) -> u8 {
        self.default_asset_decimals
    }

    pub fn invariant_checks(self) -> bool {
        self.invariant_checks
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestReceipt {
    pub redemption_id: RedemptionId,
    pub account: AccountId,
    pub vault: VaultId,
    pub kind: RedemptionKind,
    pub shares: Amount,
    pub quoted_assets: Amount,
    pub unlock_day: EpochDay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawalReceipt {
    pub account: AccountId,
    pub vault: VaultId,
    pub asset: AssetId,
    pub amount: Amount,
    pub redemptions: Vec<RedemptionId>,
}

#[derive(Debug, Clone)]
pub struct CrownEngine {
    config: EngineConfig,
    clock: Clock,
    ledger: ProtocolLedger,
    assets: AssetRegistry,
    accounts: AccountRegistry,
    vaults: BTreeMap<VaultId, VaultState>,
    queue: QueueBook,
    claims: PriorityBook,
    limits: LimitLedger,
    capacity: CapacityLedger,
}

impl Default for CrownEngine {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

impl CrownEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            clock: Clock::default(),
            ledger: ProtocolLedger::default(),
            assets: AssetRegistry::default(),
            accounts: AccountRegistry::default(),
            vaults: BTreeMap::new(),
            queue: QueueBook::default(),
            claims: PriorityBook::default(),
            limits: LimitLedger::default(),
            capacity: CapacityLedger::default(),
        }
    }

    pub fn fixture() -> Self {
        let mut engine = Self::default();
        let asset = engine
            .register_asset("cUSD", "Crown settlement dollar", 6, Amount::from(1_u64))
            .expect("fixture asset");
        let vault = engine
            .open_vault(
                "Crown DTL senior vault",
                asset,
                Self::fixture_policy().expect("fixture policy"),
            )
            .expect("fixture vault");
        let alice = engine
            .register_account("alice", AccountTier::Standard)
            .expect("fixture alice");
        let bob = engine
            .register_account("bob", AccountTier::Vip)
            .expect("fixture bob");
        let carol = engine
            .register_account("carol", AccountTier::Institutional)
            .expect("fixture carol");
        let dan = engine
            .register_account("dan", AccountTier::Standard)
            .expect("fixture dan");
        engine
            .mint_position(alice, vault, Amount::from(2_000_u64))
            .expect("alice mint");
        engine
            .mint_position(bob, vault, Amount::from(2_000_u64))
            .expect("bob mint");
        engine
            .mint_position(carol, vault, Amount::from(3_000_u64))
            .expect("carol mint");
        engine
            .mint_position(dan, vault, Amount::from(1_000_u64))
            .expect("dan mint");
        engine
    }

    pub fn fixture_policy() -> CrownResult<RedemptionPolicy> {
        let daily = DailyLimitPolicy::new(
            Amount::from(750_u64),
            Amount::from(1_250_u64),
            Amount::from(2_500_u64),
        )?;
        let priority = PriorityPolicy::new(
            Amount::from(1_500_u64),
            Amount::from(1_000_u64),
            1,
            3,
            128,
            25,
        )?;
        RedemptionPolicy::new(
            daily,
            priority,
            Amount::from(10_u64),
            Amount::from(2_000_u64),
        )
    }

    pub fn day(&self) -> EpochDay {
        self.clock.day()
    }

    pub fn advance_days(&mut self, days: u64) -> CrownResult<EpochDay> {
        self.clock.advance_days(days)
    }

    pub fn set_day(&mut self, day: EpochDay) {
        self.clock.set_day(day);
    }

    pub fn accounts(&self) -> &AccountRegistry {
        &self.accounts
    }

    pub fn assets(&self) -> &AssetRegistry {
        &self.assets
    }

    pub fn ledger(&self) -> &ProtocolLedger {
        &self.ledger
    }

    pub fn queue_snapshot(&self, vault: VaultId) -> CrownResult<QueueSnapshot> {
        self.queue.snapshot(vault)
    }

    pub fn vault(&self, id: VaultId) -> CrownResult<&VaultState> {
        self.vaults
            .get(&id)
            .ok_or_else(|| CrownError::MissingVault(id.to_string()))
    }

    pub fn vault_mut(&mut self, id: VaultId) -> CrownResult<&mut VaultState> {
        self.vaults
            .get_mut(&id)
            .ok_or_else(|| CrownError::MissingVault(id.to_string()))
    }

    pub fn vaults(&self) -> impl Iterator<Item = &VaultState> {
        self.vaults.values()
    }

    pub fn claims(&self) -> &PriorityBook {
        &self.claims
    }

    pub fn account_id(&self, label: &str) -> CrownResult<AccountId> {
        self.accounts.by_label(label)
    }

    pub fn first_vault_id(&self) -> CrownResult<VaultId> {
        self.vaults
            .keys()
            .next()
            .copied()
            .ok_or_else(|| CrownError::MissingVault("no vaults registered".to_owned()))
    }

    pub fn register_asset(
        &mut self,
        symbol: impl Into<String>,
        name: impl Into<String>,
        decimals: u8,
        minimum_redemption: Amount,
    ) -> CrownResult<AssetId> {
        let id = self.ledger.next_asset();
        let metadata = AssetMetadata::new(id, symbol, name, decimals, minimum_redemption)?;
        self.assets.insert(metadata)?;
        let event = self
            .ledger
            .event(self.day(), EventKind::AssetRegistered)
            .asset(id)
            .build();
        self.ledger.record(event);
        Ok(id)
    }

    pub fn register_account(
        &mut self,
        label: impl Into<String>,
        tier: AccountTier,
    ) -> CrownResult<AccountId> {
        let id = self.ledger.next_account();
        let account = Account::new(id, label, tier)?;
        self.accounts.insert(account)?;
        let event = self
            .ledger
            .event(self.day(), EventKind::AccountRegistered)
            .account(id)
            .note(tier.label())
            .build();
        self.ledger.record(event);
        Ok(id)
    }

    pub fn open_vault(
        &mut self,
        name: impl Into<String>,
        asset: AssetId,
        policy: RedemptionPolicy,
    ) -> CrownResult<VaultId> {
        if !self.assets.contains(asset) {
            return Err(CrownError::MissingAsset(asset.to_string()));
        }
        let id = self.ledger.next_vault();
        let mut config = VaultConfig::new(id, name, asset, policy)?;
        let priority_window = self.ledger.next_window();
        let standard_window = self.ledger.next_window();
        config.add_window(UnlockWindow::new(
            priority_window,
            WindowSpec::new(
                WindowKind::Priority,
                EpochDay::new(0),
                EpochDay::new(30),
                policy.priority().priority_unlock_delay(),
                EpochDay::new(0),
            )?,
        ))?;
        config.add_window(UnlockWindow::new(
            standard_window,
            WindowSpec::new(
                WindowKind::Standard,
                EpochDay::new(0),
                EpochDay::new(30),
                policy.priority().standard_unlock_delay(),
                EpochDay::new(0),
            )?,
        ))?;
        let state = VaultState::new(config);
        self.vaults.insert(id, state);
        let event = self
            .ledger
            .event(self.day(), EventKind::VaultOpened)
            .vault(id)
            .asset(asset)
            .build();
        self.ledger.record(event);
        Ok(id)
    }

    pub fn mint_position(
        &mut self,
        account: AccountId,
        vault: VaultId,
        shares: Amount,
    ) -> CrownResult<()> {
        shares.non_zero("mint shares")?;
        let asset = self.vault(vault)?.asset();
        self.vault_mut(vault)?.deposit_assets(shares)?;
        self.vault_mut(vault)?.mint_shares(shares)?;
        self.accounts
            .get_mut(account)?
            .portfolio_mut()
            .credit_shares(vault, shares)?;
        let event = self
            .ledger
            .event(self.day(), EventKind::SharesMinted)
            .account(account)
            .vault(vault)
            .asset(asset)
            .amount(shares)
            .build();
        self.ledger.record(event);
        self.check_invariants()
    }

    pub fn request_redemption(
        &mut self,
        account: AccountId,
        vault: VaultId,
        shares: Amount,
        kind: RedemptionKind,
    ) -> CrownResult<RequestReceipt> {
        let day = self.day();
        let account_view = self.accounts.get(account)?.clone();
        let tier = account_view.tier();
        let policy = self.vault(vault)?.policy();
        let asset = self.vault(vault)?.asset();
        policy.validate_amount(shares)?;
        if kind == RedemptionKind::Priority && !tier.may_use_priority() {
            return Err(CrownError::InvalidLane(format!(
                "account {account} cannot use priority lane"
            )));
        }
        if account_view.portfolio().share_balance(vault) < shares {
            return Err(CrownError::InsufficientShares(format!(
                "account {account} shares below {shares}"
            )));
        }
        if self.queue.depth(vault) >= policy.priority().max_queue_depth() {
            return Err(CrownError::QueueCapacity(format!(
                "vault {vault} queue is full"
            )));
        }
        let projected_daily = self
            .limits
            .active_for(account, vault, day)?
            .checked_add(shares)?;
        let daily_limit = policy.daily_limits().for_tier(tier);
        if projected_daily > daily_limit {
            return Err(CrownError::LimitExceeded(format!(
                "daily limit {daily_limit} exceeded by projected {projected_daily}"
            )));
        }
        if kind == RedemptionKind::Priority {
            let projected_priority = projected_daily;
            let per_user = policy.priority().per_user_priority_limit();
            if projected_priority > per_user {
                return Err(CrownError::LimitExceeded(format!(
                    "priority limit {per_user} exceeded by projected {projected_priority}"
                )));
            }
            let available = self.capacity.available(vault, day, policy.priority())?;
            if available < shares {
                return Err(CrownError::QueueCapacity(format!(
                    "priority capacity {available} below request {shares}"
                )));
            }
        }
        let quoted_assets = self.vault(vault)?.quote_redemption(shares)?;
        let window = self.select_window(vault, kind)?;
        if !window.accepts(day) {
            return Err(CrownError::WindowClosed(format!(
                "window {} is closed for day {}",
                window.id(),
                day.raw()
            )));
        }
        let unlock_day = window.unlock_day(day)?;
        let redemption_id = self.ledger.next_redemption();
        self.limits
            .consume(account, vault, tier, day, shares, policy.daily_limits())?;
        if kind == RedemptionKind::Priority {
            self.capacity
                .consume(vault, day, shares, policy.priority())?;
        }
        self.accounts
            .get_mut(account)?
            .portfolio_mut()
            .debit_shares(vault, shares)?;
        self.vault_mut(vault)?.burn_shares_for_redemption(shares)?;
        let item = self.queue.enqueue(
            QueueRequest {
                redemption_id,
                account,
                vault,
                amount: shares,
                lane: kind.lane(),
                tier,
                requested_day: day,
                window: window.id(),
            },
            policy.priority().max_queue_depth(),
        )?;
        let ticket = RedemptionTicket::from_draft(RedemptionTicketDraft {
            id: redemption_id,
            account,
            vault,
            asset,
            shares,
            quoted_assets,
            kind,
            requested_day: day,
            unlock_day,
            window: window.id(),
            sequence: item.sequence(),
        });
        self.vault_mut(vault)?.insert_ticket(ticket)?;
        if kind == RedemptionKind::Priority {
            let claim = PriorityClaim::from_draft(PriorityClaimDraft {
                redemption_id,
                account,
                vault,
                asset,
                amount: quoted_assets,
                unlock_day,
                window: window.id(),
                lane: kind.lane(),
            });
            self.claims.insert(claim)?;
        }
        let event = self
            .ledger
            .event(day, EventKind::RedemptionRequested)
            .account(account)
            .vault(vault)
            .asset(asset)
            .redemption(redemption_id)
            .window(window.id())
            .amount(shares)
            .note(kind.label())
            .build();
        self.ledger.record(event);
        self.check_invariants()?;
        Ok(RequestReceipt {
            redemption_id,
            account,
            vault,
            kind,
            shares,
            quoted_assets,
            unlock_day,
        })
    }

    pub fn cancel_redemption(
        &mut self,
        redemption_id: RedemptionId,
    ) -> CrownResult<RedemptionTicket> {
        let day = self.day();
        let (vault_id, account_id, shares, kind, requested_day) = {
            let ticket = self.find_ticket(redemption_id)?;
            (
                ticket.vault(),
                ticket.account(),
                ticket.shares(),
                ticket.kind(),
                ticket.requested_day(),
            )
        };
        let policy = self.vault(vault_id)?.policy();
        let removed_from_queue = self.queue.contains(vault_id, redemption_id);
        if removed_from_queue {
            let _ = self.queue.cancel(vault_id, redemption_id)?;
        }
        let ticket = self.vault_mut(vault_id)?.cancel_ticket(redemption_id)?;
        self.accounts
            .get_mut(account_id)?
            .portfolio_mut()
            .credit_shares(vault_id, shares)?;
        self.vault_mut(vault_id)?
            .restore_shares_from_redemption(shares)?;
        self.limits
            .release(account_id, vault_id, requested_day, shares)?;
        if kind == RedemptionKind::Priority {
            self.capacity.release(vault_id, requested_day, shares)?;
        } else if self.claims.pending_for_ticket(redemption_id) > Amount::ZERO {
            let _ = self.claims.release_claim(redemption_id)?;
        }
        let event = self
            .ledger
            .event(day, EventKind::RedemptionCancelled)
            .account(account_id)
            .vault(vault_id)
            .redemption(redemption_id)
            .amount(shares)
            .note(policy.priority().daily_capacity().to_string())
            .build();
        self.ledger.record(event);
        self.check_invariants()?;
        Ok(ticket)
    }

    pub fn process_vault_queue(
        &mut self,
        vault: VaultId,
        max_items: usize,
    ) -> CrownResult<Vec<RedemptionTicket>> {
        let day = self.day();
        let mut processed = Vec::new();
        let items = self.queue.pop_batch(vault, max_items);
        for item in items {
            let ticket = self
                .vault_mut(vault)?
                .schedule_ticket(item.redemption_id())?;
            if item.lane() == Lane::Standard {
                let claim = PriorityClaim::from_draft(PriorityClaimDraft {
                    redemption_id: ticket.id(),
                    account: ticket.account(),
                    vault: ticket.vault(),
                    asset: ticket.asset(),
                    amount: ticket.quoted_assets(),
                    unlock_day: ticket.unlock_day(),
                    window: ticket.window(),
                    lane: item.lane(),
                });
                self.claims.insert(claim)?;
            }
            let event = self
                .ledger
                .event(day, EventKind::RedemptionScheduled)
                .account(ticket.account())
                .vault(ticket.vault())
                .asset(ticket.asset())
                .redemption(ticket.id())
                .amount(ticket.shares())
                .note(item.lane().label())
                .build();
            self.ledger.record(event);
            processed.push(ticket);
        }
        let event = self
            .ledger
            .event(day, EventKind::WindowProcessed)
            .vault(vault)
            .amount(Amount::from(processed.len() as u64))
            .build();
        self.ledger.record(event);
        self.check_invariants()?;
        Ok(processed)
    }

    pub fn withdraw_ticket(
        &mut self,
        redemption_id: RedemptionId,
    ) -> CrownResult<WithdrawalReceipt> {
        let day = self.day();
        let ticket = self.find_ticket(redemption_id)?.clone();
        if ticket.status() != RedemptionStatus::PendingUnlock {
            return Err(CrownError::InvalidStatus(format!(
                "ticket {} is {}",
                redemption_id,
                ticket.status().label()
            )));
        }
        let claim = self.claims.consume_claim(redemption_id, day)?;
        let withdrawn = self
            .vault_mut(ticket.vault())?
            .withdraw_ticket(redemption_id, day)?;
        self.accounts
            .get_mut(withdrawn.account())?
            .portfolio_mut()
            .credit_asset(withdrawn.asset(), claim.amount())?;
        self.limits.complete(
            withdrawn.account(),
            withdrawn.vault(),
            withdrawn.requested_day(),
            withdrawn.shares(),
        )?;
        let event = self
            .ledger
            .event(day, EventKind::WithdrawalCompleted)
            .account(withdrawn.account())
            .vault(withdrawn.vault())
            .asset(withdrawn.asset())
            .redemption(withdrawn.id())
            .amount(claim.amount())
            .build();
        self.ledger.record(event);
        self.check_invariants()?;
        Ok(WithdrawalReceipt {
            account: withdrawn.account(),
            vault: withdrawn.vault(),
            asset: withdrawn.asset(),
            amount: claim.amount(),
            redemptions: vec![withdrawn.id()],
        })
    }

    pub fn withdraw_available(
        &mut self,
        account: AccountId,
        vault: VaultId,
    ) -> CrownResult<WithdrawalReceipt> {
        let day = self.day();
        let asset = self.vault(vault)?.asset();
        let claims = self
            .claims
            .consume_mature_for_account(account, vault, day)?;
        let mut total = Amount::ZERO;
        let mut ids = Vec::new();
        for claim in claims {
            self.vault_mut(vault)?
                .settle_external_claim(claim.amount())?;
            let _ = self
                .vault_mut(vault)?
                .complete_ticket_from_claim(claim.redemption_id())?;
            self.accounts
                .get_mut(account)?
                .portfolio_mut()
                .credit_asset(asset, claim.amount())?;
            if let Ok(ticket) = self.vault(vault)?.ticket(claim.redemption_id()) {
                if ticket.status() == RedemptionStatus::Withdrawn {
                    self.limits.complete(
                        ticket.account(),
                        ticket.vault(),
                        ticket.requested_day(),
                        ticket.shares(),
                    )?;
                }
            }
            total = total.checked_add(claim.amount())?;
            ids.push(claim.redemption_id());
            let event = self
                .ledger
                .event(day, EventKind::WithdrawalCompleted)
                .account(account)
                .vault(vault)
                .asset(asset)
                .redemption(claim.redemption_id())
                .amount(claim.amount())
                .note("account sweep")
                .build();
            self.ledger.record(event);
        }
        self.check_invariants()?;
        Ok(WithdrawalReceipt {
            account,
            vault,
            asset,
            amount: total,
            redemptions: ids,
        })
    }

    pub fn queue_order(&self, vault: VaultId) -> Vec<RedemptionId> {
        self.queue.peek_order(vault)
    }

    pub fn ticket(&self, redemption_id: RedemptionId) -> CrownResult<&RedemptionTicket> {
        self.find_ticket(redemption_id)
    }

    pub fn daily_active(
        &self,
        account: AccountId,
        vault: VaultId,
        day: EpochDay,
    ) -> CrownResult<Amount> {
        self.limits.active_for(account, vault, day)
    }

    pub fn priority_available(&self, vault: VaultId, day: EpochDay) -> CrownResult<Amount> {
        let policy = self.vault(vault)?.policy();
        self.capacity.available(vault, day, policy.priority())
    }

    pub fn check_invariants(&mut self) -> CrownResult<()> {
        if !self.config.invariant_checks() {
            return Ok(());
        }
        let totals = self.collect_totals()?;
        let day = self.day();
        self.ledger.assert_share_conservation(day, &totals)?;
        self.ledger.assert_claim_coverage(day, &totals)?;
        Ok(())
    }

    fn select_window(&self, vault: VaultId, kind: RedemptionKind) -> CrownResult<UnlockWindow> {
        let state = self.vault(vault)?;
        for window in state.config().windows() {
            match (kind, window.kind()) {
                (RedemptionKind::Priority, WindowKind::Priority) => return Ok(window),
                (RedemptionKind::Standard, WindowKind::Standard) => return Ok(window),
                _ => {}
            }
        }
        state.config().first_window()
    }

    fn find_ticket(&self, redemption_id: RedemptionId) -> CrownResult<&RedemptionTicket> {
        for vault in self.vaults.values() {
            if let Ok(ticket) = vault.ticket(redemption_id) {
                return Ok(ticket);
            }
        }
        Err(CrownError::MissingTicket(redemption_id.to_string()))
    }

    fn collect_totals(&self) -> CrownResult<LedgerTotals> {
        let mut totals = LedgerTotals::default();
        for vault in self.vaults.values() {
            totals.set_vault_reserve(vault.id(), vault.reserve_assets());
            totals.set_vault_shares(vault.id(), vault.total_shares());
        }
        for account in self.accounts.all() {
            for (vault, amount) in account.portfolio().share_positions() {
                totals.add_account_shares(vault, amount)?;
            }
            for (asset, amount) in account.portfolio().asset_positions() {
                totals.add_account_asset(asset, amount)?;
            }
        }
        for claim in self.claims.open_claims() {
            totals.add_open_claim(claim.vault(), claim.amount())?;
        }
        Ok(totals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_requests_require_eligible_account() {
        let mut engine = CrownEngine::fixture();
        let vault = engine.first_vault_id().unwrap();
        let alice = engine.account_id("alice").unwrap();
        let err = engine
            .request_redemption(
                alice,
                vault,
                Amount::from(100_u64),
                RedemptionKind::Priority,
            )
            .unwrap_err();
        assert_eq!(err.code(), "invalid_lane");
    }

    #[test]
    fn priority_order_precedes_standard_order() {
        let mut engine = CrownEngine::fixture();
        let vault = engine.first_vault_id().unwrap();
        let alice = engine.account_id("alice").unwrap();
        let bob = engine.account_id("bob").unwrap();
        let standard = engine
            .request_redemption(
                alice,
                vault,
                Amount::from(100_u64),
                RedemptionKind::Standard,
            )
            .unwrap();
        let priority = engine
            .request_redemption(bob, vault, Amount::from(100_u64), RedemptionKind::Priority)
            .unwrap();
        let order = engine.queue_order(vault);
        assert_eq!(order, vec![priority.redemption_id, standard.redemption_id]);
    }

    #[test]
    fn cancelled_standard_ticket_releases_daily_limit() {
        let mut engine = CrownEngine::fixture();
        let vault = engine.first_vault_id().unwrap();
        let alice = engine.account_id("alice").unwrap();
        let receipt = engine
            .request_redemption(
                alice,
                vault,
                Amount::from(700_u64),
                RedemptionKind::Standard,
            )
            .unwrap();
        engine.cancel_redemption(receipt.redemption_id).unwrap();
        let second = engine.request_redemption(
            alice,
            vault,
            Amount::from(700_u64),
            RedemptionKind::Standard,
        );
        assert!(second.is_ok());
    }
}
