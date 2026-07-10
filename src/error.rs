use std::error::Error;
use std::fmt::{Display, Formatter};

pub type CrownResult<T> = Result<T, CrownError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrownError {
    Arithmetic(String),
    DuplicateAccount(String),
    DuplicateAsset(String),
    DuplicateVault(String),
    MissingAccount(String),
    MissingAsset(String),
    MissingVault(String),
    MissingTicket(String),
    MissingWindow(String),
    MissingClaim(String),
    InsufficientShares(String),
    InsufficientAssets(String),
    LimitExceeded(String),
    QueueCapacity(String),
    WindowClosed(String),
    WindowNotReady(String),
    InvalidAmount(String),
    InvalidStatus(String),
    InvalidLane(String),
    InvalidPolicy(String),
    Invariant(String),
    Cli(String),
}

impl CrownError {
    pub fn code(&self) -> &'static str {
        match self {
            CrownError::Arithmetic(_) => "arithmetic",
            CrownError::DuplicateAccount(_) => "duplicate_account",
            CrownError::DuplicateAsset(_) => "duplicate_asset",
            CrownError::DuplicateVault(_) => "duplicate_vault",
            CrownError::MissingAccount(_) => "missing_account",
            CrownError::MissingAsset(_) => "missing_asset",
            CrownError::MissingVault(_) => "missing_vault",
            CrownError::MissingTicket(_) => "missing_ticket",
            CrownError::MissingWindow(_) => "missing_window",
            CrownError::MissingClaim(_) => "missing_claim",
            CrownError::InsufficientShares(_) => "insufficient_shares",
            CrownError::InsufficientAssets(_) => "insufficient_assets",
            CrownError::LimitExceeded(_) => "limit_exceeded",
            CrownError::QueueCapacity(_) => "queue_capacity",
            CrownError::WindowClosed(_) => "window_closed",
            CrownError::WindowNotReady(_) => "window_not_ready",
            CrownError::InvalidAmount(_) => "invalid_amount",
            CrownError::InvalidStatus(_) => "invalid_status",
            CrownError::InvalidLane(_) => "invalid_lane",
            CrownError::InvalidPolicy(_) => "invalid_policy",
            CrownError::Invariant(_) => "invariant",
            CrownError::Cli(_) => "cli",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            CrownError::Arithmetic(value)
            | CrownError::DuplicateAccount(value)
            | CrownError::DuplicateAsset(value)
            | CrownError::DuplicateVault(value)
            | CrownError::MissingAccount(value)
            | CrownError::MissingAsset(value)
            | CrownError::MissingVault(value)
            | CrownError::MissingTicket(value)
            | CrownError::MissingWindow(value)
            | CrownError::MissingClaim(value)
            | CrownError::InsufficientShares(value)
            | CrownError::InsufficientAssets(value)
            | CrownError::LimitExceeded(value)
            | CrownError::QueueCapacity(value)
            | CrownError::WindowClosed(value)
            | CrownError::WindowNotReady(value)
            | CrownError::InvalidAmount(value)
            | CrownError::InvalidStatus(value)
            | CrownError::InvalidLane(value)
            | CrownError::InvalidPolicy(value)
            | CrownError::Invariant(value)
            | CrownError::Cli(value) => value,
        }
    }

    pub fn arithmetic(message: impl Into<String>) -> Self {
        CrownError::Arithmetic(message.into())
    }

    pub fn invalid_amount(message: impl Into<String>) -> Self {
        CrownError::InvalidAmount(message.into())
    }

    pub fn invariant(message: impl Into<String>) -> Self {
        CrownError::Invariant(message.into())
    }
}

impl Display for CrownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.detail())
    }
}

impl Error for CrownError {}
