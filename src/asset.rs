use crate::amount::Amount;
use crate::error::{CrownError, CrownResult};
use crate::ids::AssetId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMetadata {
    id: AssetId,
    symbol: String,
    name: String,
    decimals: u8,
    minimum_redemption: Amount,
}

impl AssetMetadata {
    pub fn new(
        id: AssetId,
        symbol: impl Into<String>,
        name: impl Into<String>,
        decimals: u8,
        minimum_redemption: Amount,
    ) -> CrownResult<Self> {
        let symbol = symbol.into();
        if symbol.trim().is_empty() {
            return Err(CrownError::InvalidPolicy(
                "asset symbol is empty".to_owned(),
            ));
        }
        if decimals > 18 {
            return Err(CrownError::InvalidPolicy(
                "asset decimals exceed protocol range".to_owned(),
            ));
        }
        Ok(Self {
            id,
            symbol,
            name: name.into(),
            decimals,
            minimum_redemption,
        })
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn minimum_redemption(&self) -> Amount {
        self.minimum_redemption
    }

    pub fn format_amount(&self, amount: Amount) -> String {
        format!("{} {}", amount.format_units(self.decimals), self.symbol)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AssetRegistry {
    assets: BTreeMap<AssetId, AssetMetadata>,
    by_symbol: BTreeMap<String, AssetId>,
}

impl AssetRegistry {
    pub fn insert(&mut self, metadata: AssetMetadata) -> CrownResult<()> {
        if self.assets.contains_key(&metadata.id()) {
            return Err(CrownError::DuplicateAsset(metadata.id().to_string()));
        }
        let key = metadata.symbol().to_ascii_uppercase();
        if self.by_symbol.contains_key(&key) {
            return Err(CrownError::DuplicateAsset(key));
        }
        self.by_symbol.insert(key, metadata.id());
        self.assets.insert(metadata.id(), metadata);
        Ok(())
    }

    pub fn get(&self, id: AssetId) -> CrownResult<&AssetMetadata> {
        self.assets
            .get(&id)
            .ok_or_else(|| CrownError::MissingAsset(id.to_string()))
    }

    pub fn get_by_symbol(&self, symbol: &str) -> CrownResult<&AssetMetadata> {
        let key = symbol.to_ascii_uppercase();
        let id = self
            .by_symbol
            .get(&key)
            .copied()
            .ok_or_else(|| CrownError::MissingAsset(symbol.to_owned()))?;
        self.get(id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.assets.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &AssetMetadata> {
        self.assets.values()
    }
}
