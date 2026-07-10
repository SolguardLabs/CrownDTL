use crate::accounts::AccountTier;
use crate::amount::Amount;
use crate::engine::CrownEngine;
use crate::error::CrownResult;
use crate::ids::{AccountId, AssetId, RedemptionId, VaultId};
use crate::vault::RedemptionStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountReport {
    pub id: AccountId,
    pub label: String,
    pub tier: AccountTier,
    pub shares: Vec<(VaultId, Amount)>,
    pub assets: Vec<(AssetId, Amount)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultReport {
    pub id: VaultId,
    pub name: String,
    pub asset: AssetId,
    pub reserve_assets: Amount,
    pub total_shares: Amount,
    pub pending_assets: Amount,
    pub queue_depth: usize,
    pub open_claims: Amount,
    pub queued: usize,
    pub pending_unlock: usize,
    pub cancelled: usize,
    pub withdrawn: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolReport {
    pub day: u64,
    pub accounts: Vec<AccountReport>,
    pub vaults: Vec<VaultReport>,
    pub event_count: usize,
}

impl AccountReport {
    pub fn to_json(&self) -> String {
        let shares = self
            .shares
            .iter()
            .map(|(vault, amount)| {
                format!(
                    "{{\"vault\":\"{}\",\"amount\":{}}}",
                    json_escape(&vault.to_string()),
                    amount.raw()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let assets = self
            .assets
            .iter()
            .map(|(asset, amount)| {
                format!(
                    "{{\"asset\":\"{}\",\"amount\":{}}}",
                    json_escape(&asset.to_string()),
                    amount.raw()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\":\"{}\",\"label\":\"{}\",\"tier\":\"{}\",\"shares\":[{}],\"assets\":[{}]}}",
            json_escape(&self.id.to_string()),
            json_escape(&self.label),
            self.tier.label(),
            shares,
            assets
        )
    }
}

impl VaultReport {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"asset\":\"{}\",\"reserveAssets\":{},\"totalShares\":{},\"pendingAssets\":{},\"queueDepth\":{},\"openClaims\":{},\"tickets\":{{\"queued\":{},\"pendingUnlock\":{},\"cancelled\":{},\"withdrawn\":{}}}}}",
            json_escape(&self.id.to_string()),
            json_escape(&self.name),
            json_escape(&self.asset.to_string()),
            self.reserve_assets.raw(),
            self.total_shares.raw(),
            self.pending_assets.raw(),
            self.queue_depth,
            self.open_claims.raw(),
            self.queued,
            self.pending_unlock,
            self.cancelled,
            self.withdrawn
        )
    }
}

impl ProtocolReport {
    pub fn capture(engine: &CrownEngine) -> CrownResult<Self> {
        let mut accounts = Vec::new();
        for account in engine.accounts().all() {
            accounts.push(AccountReport {
                id: account.id(),
                label: account.label().to_owned(),
                tier: account.tier(),
                shares: account.portfolio().share_positions().collect(),
                assets: account.portfolio().asset_positions().collect(),
            });
        }
        let mut vaults = Vec::new();
        for vault in engine.vaults() {
            let snapshot = engine.queue_snapshot(vault.id())?;
            let mut queued = 0usize;
            let mut pending_unlock = 0usize;
            let mut cancelled = 0usize;
            let mut withdrawn = 0usize;
            for ticket in vault.tickets() {
                match ticket.status() {
                    RedemptionStatus::Queued => queued += 1,
                    RedemptionStatus::PendingUnlock => pending_unlock += 1,
                    RedemptionStatus::Cancelled => cancelled += 1,
                    RedemptionStatus::Withdrawn => withdrawn += 1,
                }
            }
            vaults.push(VaultReport {
                id: vault.id(),
                name: vault.name().to_owned(),
                asset: vault.asset(),
                reserve_assets: vault.reserve_assets(),
                total_shares: vault.total_shares(),
                pending_assets: vault.pending_assets(),
                queue_depth: snapshot.priority_depth() + snapshot.standard_depth(),
                open_claims: engine.claims().open_claims().try_fold(
                    Amount::ZERO,
                    |total, claim| {
                        if claim.vault() == vault.id() {
                            total.checked_add(claim.amount())
                        } else {
                            Ok(total)
                        }
                    },
                )?,
                queued,
                pending_unlock,
                cancelled,
                withdrawn,
            });
        }
        Ok(Self {
            day: engine.day().raw(),
            accounts,
            vaults,
            event_count: engine.ledger().journal().len(),
        })
    }

    pub fn to_json(&self) -> String {
        let accounts = self
            .accounts
            .iter()
            .map(AccountReport::to_json)
            .collect::<Vec<_>>()
            .join(",");
        let vaults = self
            .vaults
            .iter()
            .map(VaultReport::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"day\":{},\"eventCount\":{},\"accounts\":[{}],\"vaults\":[{}]}}",
            self.day, self.event_count, accounts, vaults
        )
    }
}

pub fn id_array_json(ids: &[RedemptionId]) -> String {
    let body = ids
        .iter()
        .map(|id| format!("\"{}\"", json_escape(&id.to_string())))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

pub fn string_array_json(values: &[String]) -> String {
    let body = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

pub fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
