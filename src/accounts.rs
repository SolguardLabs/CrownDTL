use crate::amount::Amount;
use crate::error::{CrownError, CrownResult};
use crate::ids::{AccountId, AssetId, VaultId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountTier {
    Standard,
    Vip,
    Institutional,
}

impl AccountTier {
    pub fn priority_weight(self) -> u32 {
        match self {
            AccountTier::Standard => 10,
            AccountTier::Vip => 60,
            AccountTier::Institutional => 90,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AccountTier::Standard => "standard",
            AccountTier::Vip => "vip",
            AccountTier::Institutional => "institutional",
        }
    }

    pub fn may_use_priority(self) -> bool {
        matches!(self, AccountTier::Vip | AccountTier::Institutional)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Portfolio {
    shares: BTreeMap<VaultId, Amount>,
    assets: BTreeMap<AssetId, Amount>,
    pending_assets: BTreeMap<AssetId, Amount>,
}

impl Portfolio {
    pub fn share_balance(&self, vault: VaultId) -> Amount {
        self.shares.get(&vault).copied().unwrap_or_default()
    }

    pub fn asset_balance(&self, asset: AssetId) -> Amount {
        self.assets.get(&asset).copied().unwrap_or_default()
    }

    pub fn pending_asset_balance(&self, asset: AssetId) -> Amount {
        self.pending_assets.get(&asset).copied().unwrap_or_default()
    }

    pub fn credit_shares(&mut self, vault: VaultId, amount: Amount) -> CrownResult<()> {
        let next = self.share_balance(vault).checked_add(amount)?;
        self.shares.insert(vault, next);
        Ok(())
    }

    pub fn debit_shares(&mut self, vault: VaultId, amount: Amount) -> CrownResult<()> {
        let current = self.share_balance(vault);
        if current < amount {
            return Err(CrownError::InsufficientShares(format!(
                "vault {vault} has {current}, needs {amount}"
            )));
        }
        let next = current.checked_sub(amount)?;
        if next.is_zero() {
            self.shares.remove(&vault);
        } else {
            self.shares.insert(vault, next);
        }
        Ok(())
    }

    pub fn credit_asset(&mut self, asset: AssetId, amount: Amount) -> CrownResult<()> {
        let next = self.asset_balance(asset).checked_add(amount)?;
        self.assets.insert(asset, next);
        Ok(())
    }

    pub fn debit_asset(&mut self, asset: AssetId, amount: Amount) -> CrownResult<()> {
        let current = self.asset_balance(asset);
        if current < amount {
            return Err(CrownError::InsufficientAssets(format!(
                "asset {asset} has {current}, needs {amount}"
            )));
        }
        let next = current.checked_sub(amount)?;
        if next.is_zero() {
            self.assets.remove(&asset);
        } else {
            self.assets.insert(asset, next);
        }
        Ok(())
    }

    pub fn reserve_pending_asset(&mut self, asset: AssetId, amount: Amount) -> CrownResult<()> {
        let next = self.pending_asset_balance(asset).checked_add(amount)?;
        self.pending_assets.insert(asset, next);
        Ok(())
    }

    pub fn release_pending_asset(&mut self, asset: AssetId, amount: Amount) -> CrownResult<()> {
        let current = self.pending_asset_balance(asset);
        if current < amount {
            return Err(CrownError::Invariant(format!(
                "pending asset {asset} has {current}, needs {amount}"
            )));
        }
        let next = current.checked_sub(amount)?;
        if next.is_zero() {
            self.pending_assets.remove(&asset);
        } else {
            self.pending_assets.insert(asset, next);
        }
        Ok(())
    }

    pub fn share_positions(&self) -> impl Iterator<Item = (VaultId, Amount)> + '_ {
        self.shares.iter().map(|(vault, amount)| (*vault, *amount))
    }

    pub fn asset_positions(&self) -> impl Iterator<Item = (AssetId, Amount)> + '_ {
        self.assets.iter().map(|(asset, amount)| (*asset, *amount))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    label: String,
    tier: AccountTier,
    portfolio: Portfolio,
}

impl Account {
    pub fn new(id: AccountId, label: impl Into<String>, tier: AccountTier) -> CrownResult<Self> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(CrownError::InvalidPolicy(
                "account label is empty".to_owned(),
            ));
        }
        Ok(Self {
            id,
            label,
            tier,
            portfolio: Portfolio::default(),
        })
    }

    pub fn id(&self) -> AccountId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn tier(&self) -> AccountTier {
        self.tier
    }

    pub fn portfolio(&self) -> &Portfolio {
        &self.portfolio
    }

    pub fn portfolio_mut(&mut self) -> &mut Portfolio {
        &mut self.portfolio
    }
}

#[derive(Debug, Clone, Default)]
pub struct AccountRegistry {
    accounts: BTreeMap<AccountId, Account>,
    by_label: BTreeMap<String, AccountId>,
}

impl AccountRegistry {
    pub fn insert(&mut self, account: Account) -> CrownResult<()> {
        if self.accounts.contains_key(&account.id()) {
            return Err(CrownError::DuplicateAccount(account.id().to_string()));
        }
        let key = account.label().to_ascii_lowercase();
        if self.by_label.contains_key(&key) {
            return Err(CrownError::DuplicateAccount(account.label().to_owned()));
        }
        self.by_label.insert(key, account.id());
        self.accounts.insert(account.id(), account);
        Ok(())
    }

    pub fn get(&self, id: AccountId) -> CrownResult<&Account> {
        self.accounts
            .get(&id)
            .ok_or_else(|| CrownError::MissingAccount(id.to_string()))
    }

    pub fn get_mut(&mut self, id: AccountId) -> CrownResult<&mut Account> {
        self.accounts
            .get_mut(&id)
            .ok_or_else(|| CrownError::MissingAccount(id.to_string()))
    }

    pub fn by_label(&self, label: &str) -> CrownResult<AccountId> {
        self.by_label
            .get(&label.to_ascii_lowercase())
            .copied()
            .ok_or_else(|| CrownError::MissingAccount(label.to_owned()))
    }

    pub fn all(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}
