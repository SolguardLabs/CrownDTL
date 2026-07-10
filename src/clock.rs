use crate::error::{CrownError, CrownResult};
use crate::ids::WindowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EpochDay(u64);

impl EpochDay {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn raw(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn checked_add(self, days: u64) -> CrownResult<Self> {
        self.0
            .checked_add(days)
            .map(Self)
            .ok_or_else(|| CrownError::arithmetic("epoch day overflow"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Priority,
    Standard,
    Maintenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSpec {
    kind: WindowKind,
    open_day: EpochDay,
    close_day: EpochDay,
    unlock_delay_days: u64,
    capacity_epoch: EpochDay,
}

impl WindowSpec {
    pub fn new(
        kind: WindowKind,
        open_day: EpochDay,
        close_day: EpochDay,
        unlock_delay_days: u64,
        capacity_epoch: EpochDay,
    ) -> CrownResult<Self> {
        if close_day < open_day {
            return Err(CrownError::InvalidPolicy(
                "window closes before it opens".to_owned(),
            ));
        }
        Ok(Self {
            kind,
            open_day,
            close_day,
            unlock_delay_days,
            capacity_epoch,
        })
    }

    pub fn priority(open_day: EpochDay, close_day: EpochDay, delay: u64) -> CrownResult<Self> {
        Self::new(WindowKind::Priority, open_day, close_day, delay, open_day)
    }

    pub fn standard(open_day: EpochDay, close_day: EpochDay, delay: u64) -> CrownResult<Self> {
        Self::new(WindowKind::Standard, open_day, close_day, delay, open_day)
    }

    pub fn kind(self) -> WindowKind {
        self.kind
    }

    pub fn open_day(self) -> EpochDay {
        self.open_day
    }

    pub fn close_day(self) -> EpochDay {
        self.close_day
    }

    pub fn unlock_day(self, requested_on: EpochDay) -> CrownResult<EpochDay> {
        requested_on.checked_add(self.unlock_delay_days)
    }

    pub fn capacity_epoch(self) -> EpochDay {
        self.capacity_epoch
    }

    pub fn contains(self, day: EpochDay) -> bool {
        day >= self.open_day && day <= self.close_day
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockWindow {
    id: WindowId,
    spec: WindowSpec,
}

impl UnlockWindow {
    pub fn new(id: WindowId, spec: WindowSpec) -> Self {
        Self { id, spec }
    }

    pub fn id(self) -> WindowId {
        self.id
    }

    pub fn spec(self) -> WindowSpec {
        self.spec
    }

    pub fn kind(self) -> WindowKind {
        self.spec.kind()
    }

    pub fn accepts(self, day: EpochDay) -> bool {
        self.spec.contains(day)
    }

    pub fn unlock_day(self, requested_on: EpochDay) -> CrownResult<EpochDay> {
        self.spec.unlock_day(requested_on)
    }
}

#[derive(Debug, Clone)]
pub struct Clock {
    day: EpochDay,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            day: EpochDay::new(0),
        }
    }
}

impl Clock {
    pub fn new(day: EpochDay) -> Self {
        Self { day }
    }

    pub fn day(&self) -> EpochDay {
        self.day
    }

    pub fn advance_days(&mut self, days: u64) -> CrownResult<EpochDay> {
        self.day = self.day.checked_add(days)?;
        Ok(self.day)
    }

    pub fn set_day(&mut self, day: EpochDay) {
        self.day = day;
    }
}
