use crate::error::{CrownError, CrownResult};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationSpec {
    pub domain: String,
    pub network: String,
    pub target: String,
    pub method: String,
    pub payload_hash: String,
    pub salt: String,
    pub execute_after: u64,
    pub expires_at: u64,
    pub predecessor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Scheduled,
    Ready,
    Executed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRecord {
    pub id: String,
    pub spec: OperationSpec,
    pub approvals: BTreeSet<String>,
    pub status: OperationStatus,
    pub scheduled_at: u64,
    pub executed_at: Option<u64>,
    pub cancelled_at: Option<u64>,
    pub cancel_reason: Option<String>,
}

impl OperationSpec {
    pub fn validate(&self) -> CrownResult<()> {
        for (name, value) in [
            ("domain", &self.domain),
            ("network", &self.network),
            ("target", &self.target),
            ("method", &self.method),
            ("payload hash", &self.payload_hash),
            ("salt", &self.salt),
        ] {
            if value.trim().is_empty() || value != value.trim() {
                return Err(CrownError::Invariant(format!("{name} must be normalized")));
            }
        }
        if !is_digest(&self.payload_hash) {
            return Err(CrownError::Invariant(
                "payload hash must be lowercase SHA-256".to_owned(),
            ));
        }
        if let Some(predecessor) = &self.predecessor {
            if !is_digest(predecessor) {
                return Err(CrownError::Invariant(
                    "predecessor must be a lowercase operation id".to_owned(),
                ));
            }
        }
        if self.expires_at <= self.execute_after {
            return Err(CrownError::Invariant(
                "expiry must follow execution start".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> CrownResult<Vec<u8>> {
        self.validate()?;
        let mut output = Vec::new();
        for value in [
            self.domain.as_str(),
            self.network.as_str(),
            self.target.as_str(),
            self.method.as_str(),
            self.payload_hash.as_str(),
            self.salt.as_str(),
            self.predecessor.as_deref().unwrap_or(""),
        ] {
            output.extend_from_slice(&(value.len() as u64).to_be_bytes());
            output.extend_from_slice(value.as_bytes());
        }
        output.extend_from_slice(&self.execute_after.to_be_bytes());
        output.extend_from_slice(&self.expires_at.to_be_bytes());
        Ok(output)
    }

    pub fn id(&self) -> CrownResult<String> {
        Ok(hex_digest(&Sha256::digest(self.canonical_bytes()?)))
    }
}

pub fn payload_hash(payload: &[u8]) -> String {
    hex_digest(&Sha256::digest(payload))
}

#[derive(Debug, Clone)]
pub struct Council {
    governors: BTreeSet<String>,
    guardian: String,
    quorum: usize,
    operations: BTreeMap<String, OperationRecord>,
}

impl Council {
    pub fn new(
        governors: impl IntoIterator<Item = String>,
        quorum: usize,
        guardian: String,
    ) -> CrownResult<Self> {
        let governors = governors.into_iter().collect::<BTreeSet<_>>();
        if governors.is_empty() || quorum == 0 || quorum > governors.len() {
            return Err(CrownError::Invariant("invalid governor quorum".to_owned()));
        }
        if governors
            .iter()
            .any(|value| value.trim().is_empty() || value != value.trim())
            || guardian.trim().is_empty()
            || guardian != guardian.trim()
            || governors.contains(&guardian)
        {
            return Err(CrownError::Invariant(
                "governance identity is required".to_owned(),
            ));
        }
        Ok(Self {
            governors,
            guardian,
            quorum,
            operations: BTreeMap::new(),
        })
    }

    pub fn schedule(
        &mut self,
        spec: OperationSpec,
        proposer: &str,
        now: u64,
    ) -> CrownResult<OperationRecord> {
        self.require_governor(proposer)?;
        if spec.execute_after <= now {
            return Err(CrownError::Invariant(
                "timelock must start in the future".to_owned(),
            ));
        }
        let id = spec.id()?;
        if self.operations.contains_key(&id) {
            return Err(CrownError::Invariant(
                "operation already scheduled".to_owned(),
            ));
        }
        let mut approvals = BTreeSet::new();
        approvals.insert(proposer.to_owned());
        let record = OperationRecord {
            id: id.clone(),
            spec,
            approvals,
            status: OperationStatus::Scheduled,
            scheduled_at: now,
            executed_at: None,
            cancelled_at: None,
            cancel_reason: None,
        };
        self.operations.insert(id, record.clone());
        Ok(record)
    }

    pub fn approve(&mut self, id: &str, governor: &str, now: u64) -> CrownResult<OperationRecord> {
        self.require_governor(governor)?;
        let record = self
            .operations
            .get_mut(id)
            .ok_or_else(|| CrownError::Invariant("operation not found".to_owned()))?;
        ensure_live(record, now)?;
        record.approvals.insert(governor.to_owned());
        Ok(snapshot(record, self.quorum, now))
    }

    pub fn cancel(
        &mut self,
        id: &str,
        guardian: &str,
        reason: &str,
        now: u64,
    ) -> CrownResult<OperationRecord> {
        if guardian != self.guardian || reason.trim().is_empty() {
            return Err(CrownError::Invariant(
                "guardian and reason are required".to_owned(),
            ));
        }
        let record = self
            .operations
            .get_mut(id)
            .ok_or_else(|| CrownError::Invariant("operation not found".to_owned()))?;
        ensure_live(record, now)?;
        record.status = OperationStatus::Cancelled;
        record.cancelled_at = Some(now);
        record.cancel_reason = Some(reason.to_owned());
        Ok(record.clone())
    }

    pub fn execute(&mut self, id: &str, now: u64) -> CrownResult<OperationRecord> {
        let predecessor = self
            .operations
            .get(id)
            .and_then(|record| record.spec.predecessor.clone());
        if let Some(predecessor) = predecessor {
            if self
                .operations
                .get(&predecessor)
                .map(|record| record.status)
                != Some(OperationStatus::Executed)
            {
                return Err(CrownError::Invariant(
                    "predecessor is not executed".to_owned(),
                ));
            }
        }
        let record = self
            .operations
            .get_mut(id)
            .ok_or_else(|| CrownError::Invariant("operation not found".to_owned()))?;
        ensure_live(record, now)?;
        if now < record.spec.execute_after || record.approvals.len() < self.quorum {
            return Err(CrownError::Invariant("operation is not ready".to_owned()));
        }
        record.status = OperationStatus::Executed;
        record.executed_at = Some(now);
        Ok(record.clone())
    }

    pub fn get(&self, id: &str, now: u64) -> Option<OperationRecord> {
        self.operations
            .get(id)
            .map(|record| snapshot(record, self.quorum, now))
    }

    fn require_governor(&self, governor: &str) -> CrownResult<()> {
        if self.governors.contains(governor) {
            Ok(())
        } else {
            Err(CrownError::Invariant("caller is not a governor".to_owned()))
        }
    }
}

fn ensure_live(record: &mut OperationRecord, now: u64) -> CrownResult<()> {
    if now >= record.spec.expires_at {
        record.status = OperationStatus::Expired;
    }
    if matches!(
        record.status,
        OperationStatus::Executed | OperationStatus::Cancelled | OperationStatus::Expired
    ) {
        return Err(CrownError::Invariant("operation is terminal".to_owned()));
    }
    Ok(())
}

fn snapshot(record: &OperationRecord, quorum: usize, now: u64) -> OperationRecord {
    let mut output = record.clone();
    if !matches!(
        output.status,
        OperationStatus::Executed | OperationStatus::Cancelled
    ) {
        output.status = if now >= output.spec.expires_at {
            OperationStatus::Expired
        } else if now >= output.spec.execute_after && output.approvals.len() >= quorum {
            OperationStatus::Ready
        } else {
            OperationStatus::Scheduled
        };
    }
    output
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> OperationSpec {
        OperationSpec {
            domain: "crown.governance.v1".to_owned(),
            network: "settlement-eu-1".to_owned(),
            target: "vault:senior".to_owned(),
            method: "set_priority_capacity".to_owned(),
            payload_hash: payload_hash(b"capacity=1500000"),
            salt: "capacity-2026-08".to_owned(),
            execute_after: 10,
            expires_at: 20,
            predecessor: None,
        }
    }

    #[test]
    fn operation_id_is_deterministic_and_domain_separated() {
        let original = spec();
        assert_eq!(original.id().unwrap(), original.id().unwrap());
        let mut changed = original.clone();
        changed.network = "settlement-us-1".to_owned();
        assert_ne!(original.id().unwrap(), changed.id().unwrap());
    }

    #[test]
    fn quorum_and_timelock_are_required() {
        let mut council = Council::new(
            ["alice", "bob", "carol"].map(str::to_owned),
            2,
            "guardian".to_owned(),
        )
        .unwrap();
        let record = council.schedule(spec(), "alice", 1).unwrap();
        assert!(council.execute(&record.id, 10).is_err());
        council.approve(&record.id, "bob", 5).unwrap();
        assert!(council.execute(&record.id, 9).is_err());
        assert_eq!(
            council.execute(&record.id, 10).unwrap().status,
            OperationStatus::Executed
        );
    }

    #[test]
    fn guardian_cancellation_is_terminal() {
        let mut council = Council::new(["alice".to_owned()], 1, "guardian".to_owned()).unwrap();
        let record = council.schedule(spec(), "alice", 1).unwrap();
        assert!(council.cancel(&record.id, "other", "stop", 2).is_err());
        assert_eq!(
            council
                .cancel(&record.id, "guardian", "capacity review", 2)
                .unwrap()
                .status,
            OperationStatus::Cancelled
        );
        assert!(council.execute(&record.id, 10).is_err());
    }

    #[test]
    fn guardian_is_separate_from_governors() {
        assert!(Council::new(["alice".to_owned()], 1, "alice".to_owned()).is_err());
        assert!(Council::new([" alice".to_owned()], 1, "guardian".to_owned()).is_err());
    }
}
