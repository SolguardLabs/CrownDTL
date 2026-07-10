use std::fmt::{Display, Formatter};

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(u64);

        impl $name {
            pub fn new(value: u64) -> Self {
                Self(value)
            }

            pub fn raw(self) -> u64 {
                self.0
            }

            pub fn is_zero(self) -> bool {
                self.0 == 0
            }

            pub fn next_after(self) -> Self {
                Self(self.0 + 1)
            }

            pub fn label(self) -> String {
                format!("{}{}", $prefix, self.0)
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self::new(value)
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}{}", $prefix, self.0)
            }
        }
    };
}

id_type!(AccountId, "acct-");
id_type!(AssetId, "asset-");
id_type!(VaultId, "vault-");
id_type!(RedemptionId, "rdm-");
id_type!(WindowId, "win-");
id_type!(JournalId, "jrn-");
id_type!(RouteId, "route-");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Sequence(u64);

impl Sequence {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn raw(self) -> u64 {
        self.0
    }

    pub fn bump(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }

    pub fn peek_next(self) -> u64 {
        self.0 + 1
    }
}

#[derive(Debug, Clone)]
pub struct IdAllocator {
    next_account: Sequence,
    next_asset: Sequence,
    next_vault: Sequence,
    next_redemption: Sequence,
    next_window: Sequence,
    next_journal: Sequence,
    next_route: Sequence,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self {
            next_account: Sequence::new(0),
            next_asset: Sequence::new(0),
            next_vault: Sequence::new(0),
            next_redemption: Sequence::new(0),
            next_window: Sequence::new(0),
            next_journal: Sequence::new(0),
            next_route: Sequence::new(0),
        }
    }
}

impl IdAllocator {
    pub fn account(&mut self) -> AccountId {
        AccountId::new(self.next_account.bump())
    }

    pub fn asset(&mut self) -> AssetId {
        AssetId::new(self.next_asset.bump())
    }

    pub fn vault(&mut self) -> VaultId {
        VaultId::new(self.next_vault.bump())
    }

    pub fn redemption(&mut self) -> RedemptionId {
        RedemptionId::new(self.next_redemption.bump())
    }

    pub fn window(&mut self) -> WindowId {
        WindowId::new(self.next_window.bump())
    }

    pub fn journal(&mut self) -> JournalId {
        JournalId::new(self.next_journal.bump())
    }

    pub fn route(&mut self) -> RouteId {
        RouteId::new(self.next_route.bump())
    }

    pub fn reserve_account(&mut self, id: AccountId) {
        if id.raw() > self.next_account.raw() {
            self.next_account = Sequence::new(id.raw());
        }
    }

    pub fn reserve_asset(&mut self, id: AssetId) {
        if id.raw() > self.next_asset.raw() {
            self.next_asset = Sequence::new(id.raw());
        }
    }

    pub fn reserve_vault(&mut self, id: VaultId) {
        if id.raw() > self.next_vault.raw() {
            self.next_vault = Sequence::new(id.raw());
        }
    }

    pub fn reserve_redemption(&mut self, id: RedemptionId) {
        if id.raw() > self.next_redemption.raw() {
            self.next_redemption = Sequence::new(id.raw());
        }
    }
}
