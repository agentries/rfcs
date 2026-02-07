use std::collections::{HashMap, HashSet};

pub const FWD_V1: u64 = 1;
pub const RECEIPT_V1: u64 = 1;
pub const COMMIT_V1: u64 = 1;
pub const DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_HANDOFF_MAX_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayError {
    pub code: u16,
    pub name: &'static str,
    pub detail: String,
}

impl RelayError {
    pub fn invalid_message(detail: impl Into<String>) -> Self {
        Self {
            code: 1001,
            name: "INVALID_MESSAGE",
            detail: detail.into(),
        }
    }

    pub fn unsupported_version(detail: impl Into<String>) -> Self {
        Self {
            code: 1004,
            name: "UNSUPPORTED_VERSION",
            detail: detail.into(),
        }
    }

    pub fn recipient_not_found(detail: impl Into<String>) -> Self {
        Self {
            code: 2001,
            name: "RECIPIENT_NOT_FOUND",
            detail: detail.into(),
        }
    }

    pub fn endpoint_unavailable(detail: impl Into<String>) -> Self {
        Self {
            code: 2002,
            name: "ENDPOINT_UNAVAILABLE",
            detail: detail.into(),
        }
    }

    pub fn relay_rejected(detail: impl Into<String>) -> Self {
        Self {
            code: 2003,
            name: "RELAY_REJECTED",
            detail: detail.into(),
        }
    }

    pub fn message_expired(detail: impl Into<String>) -> Self {
        Self {
            code: 2004,
            name: "MESSAGE_EXPIRED",
            detail: detail.into(),
        }
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self {
            code: 3001,
            name: "UNAUTHORIZED",
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub from_did: String,
    pub msg_id: String,
    pub recipients: Vec<String>,
    pub ts_ms: u64,
    pub ttl_ms: u64,
}

impl Message {
    pub fn expires_at(&self) -> u64 {
        self.ts_ms.saturating_add(self.ttl_ms)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientState {
    Pending,
    Inflight,
    Delivered,
    Failed,
    Expired,
}

impl RecipientState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RecipientState::Delivered | RecipientState::Failed | RecipientState::Expired
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    Queued,
    Dispatching,
    Done,
    Expired,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Single,
    Dual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    None,
    Pending,
    Accepted,
    RolledBack,
    CommitReported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitResult {
    Delivered,
    Failed,
    Expired,
}

#[derive(Debug, Clone)]
pub struct RelayForward {
    pub fwd_v: u64,
    pub from_did: String,
    pub msg_id: String,
    pub recipient_did: String,
    pub relay_path: Vec<String>,
    pub hop_limit: u64,
    pub upstream_relay: String,
    pub downstream_relay: String,
    pub transfer_mode: TransferMode,
}

#[derive(Debug, Clone)]
pub struct TransferReceipt {
    pub receipt_v: u64,
    pub msg_id: String,
    pub from_did: String,
    pub recipient_did: String,
    pub upstream_relay: String,
    pub downstream_relay: String,
    pub accepted_at: u64,
    pub hop_limit_remaining: u64,
    pub accepted: bool,
    pub alg: i32,
    pub kid: String,
    pub key_purpose: String,
}

#[derive(Debug, Clone)]
pub struct CommitReceipt {
    pub commit_v: u64,
    pub msg_id: String,
    pub from_did: String,
    pub recipient_did: String,
    pub upstream_relay: String,
    pub downstream_relay: String,
    pub result: CommitResult,
    pub committed_at: u64,
    pub alg: i32,
    pub kid: String,
    pub key_purpose: String,
}

#[derive(Debug, Clone)]
pub struct RecipientEntry {
    pub state: RecipientState,
    pub retained_local_copy: bool,
    pub transfer_state: TransferState,
    pub transfer_mode: Option<TransferMode>,
    pub downstream_relay: Option<String>,
    pub last_transfer_change_ms: u64,
    pub handoff_attempts: u8,
}

#[derive(Debug, Clone)]
pub struct QueueRecord {
    pub from_did: String,
    pub msg_id: String,
    pub recipients: HashMap<String, RecipientEntry>,
    pub accepted_at: u64,
    pub expires_at: u64,
    pub status: QueueStatus,
}

#[derive(Debug, Clone)]
pub struct Relay {
    pub relay_id: String,
    pub now_ms: u64,
    dedupe_active: HashSet<(String, String, String)>,
    records: HashMap<(String, String), QueueRecord>,
}

impl Relay {
    pub fn new(relay_id: impl Into<String>, now_ms: u64) -> Self {
        Self {
            relay_id: relay_id.into(),
            now_ms,
            dedupe_active: HashSet::new(),
            records: HashMap::new(),
        }
    }

    pub fn set_now(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    pub fn ingress(
        &mut self,
        message: &Message,
        recipient_online: &HashMap<String, bool>,
    ) -> Result<(), RelayError> {
        if message.recipients.is_empty() {
            return Err(RelayError::recipient_not_found(
                "message recipients must not be empty",
            ));
        }
        if self.now_ms > message.expires_at() {
            return Err(RelayError::message_expired("ingress message already expired"));
        }

        if message.ttl_ms == 0 {
            let all_online = message
                .recipients
                .iter()
                .all(|r| recipient_online.get(r).copied().unwrap_or(false));
            if !all_online {
                return Err(RelayError::relay_rejected(
                    "ttl=0 requires immediate next-hop availability",
                ));
            }
            return Ok(());
        }

        let key = (message.from_did.clone(), message.msg_id.clone());
        let record = self.records.entry(key).or_insert_with(|| QueueRecord {
            from_did: message.from_did.clone(),
            msg_id: message.msg_id.clone(),
            recipients: HashMap::new(),
            accepted_at: self.now_ms,
            expires_at: message.expires_at(),
            status: QueueStatus::Queued,
        });

        for recipient in &message.recipients {
            let dedupe_key = (
                message.from_did.clone(),
                message.msg_id.clone(),
                recipient.clone(),
            );
            if self.dedupe_active.contains(&dedupe_key) {
                continue;
            }
            self.dedupe_active.insert(dedupe_key);
            record.recipients.insert(
                recipient.clone(),
                RecipientEntry {
                    state: RecipientState::Pending,
                    retained_local_copy: true,
                    transfer_state: TransferState::None,
                    transfer_mode: None,
                    downstream_relay: None,
                    last_transfer_change_ms: self.now_ms,
                    handoff_attempts: 0,
                },
            );
        }

        if record.recipients.is_empty() {
            return Ok(());
        }
        record.status = QueueStatus::Queued;
        Ok(())
    }

    pub fn poll(&mut self, recipient_did: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for ((from, msg), record) in &mut self.records {
            if let Some(entry) = record.recipients.get_mut(recipient_did) {
                if matches!(entry.state, RecipientState::Pending | RecipientState::Inflight) {
                    entry.state = RecipientState::Inflight;
                    out.push((from.clone(), msg.clone()));
                }
            }
        }
        out
    }

    pub fn ack_recipient(
        &mut self,
        from_did: &str,
        msg_id: &str,
        recipient_did: &str,
    ) -> Result<(), RelayError> {
        let record = self
            .records
            .get_mut(&(from_did.to_string(), msg_id.to_string()))
            .ok_or_else(|| RelayError::recipient_not_found("queue record not found"))?;
        let entry = record
            .recipients
            .get_mut(recipient_did)
            .ok_or_else(|| RelayError::recipient_not_found("recipient state not found"))?;

        entry.state = RecipientState::Delivered;
        Self::refresh_record_status(record);
        Ok(())
    }

    pub fn expire(&mut self) {
        for record in self.records.values_mut() {
            if self.now_ms > record.expires_at {
                for entry in record.recipients.values_mut() {
                    if !entry.state.is_terminal() {
                        entry.state = RecipientState::Expired;
                    }
                }
                record.status = QueueStatus::Expired;
                Self::refresh_record_status(record);
            }
        }
    }

    pub fn start_handoff(
        &mut self,
        from_did: &str,
        msg_id: &str,
        recipient_did: &str,
        downstream_relay: &str,
        mode: TransferMode,
    ) -> Result<(), RelayError> {
        let record = self
            .records
            .get_mut(&(from_did.to_string(), msg_id.to_string()))
            .ok_or_else(|| RelayError::recipient_not_found("queue record not found"))?;
        let entry = record
            .recipients
            .get_mut(recipient_did)
            .ok_or_else(|| RelayError::recipient_not_found("recipient state not found"))?;

        entry.transfer_state = TransferState::Pending;
        entry.transfer_mode = Some(mode);
        entry.downstream_relay = Some(downstream_relay.to_string());
        entry.last_transfer_change_ms = self.now_ms;
        entry.handoff_attempts = entry.handoff_attempts.saturating_add(1);
        Ok(())
    }

    pub fn apply_transfer_receipt(
        &mut self,
        forward: &RelayForward,
        receipt: &TransferReceipt,
        supported_algs: &[i32],
    ) -> Result<(), RelayError> {
        validate_transfer_receipt(forward, receipt, supported_algs)?;

        let record = self
            .records
            .get_mut(&(forward.from_did.clone(), forward.msg_id.clone()))
            .ok_or_else(|| RelayError::recipient_not_found("queue record not found"))?;
        let entry = record
            .recipients
            .get_mut(&forward.recipient_did)
            .ok_or_else(|| RelayError::recipient_not_found("recipient state not found"))?;

        entry.transfer_state = TransferState::Accepted;
        entry.last_transfer_change_ms = self.now_ms;
        if forward.transfer_mode == TransferMode::Single {
            entry.retained_local_copy = false;
        }
        Ok(())
    }

    pub fn apply_commit_receipt(
        &mut self,
        forward: &RelayForward,
        receipt: &CommitReceipt,
        supported_algs: &[i32],
    ) -> Result<(), RelayError> {
        validate_commit_receipt(forward, receipt, supported_algs)?;

        let record = self
            .records
            .get_mut(&(forward.from_did.clone(), forward.msg_id.clone()))
            .ok_or_else(|| RelayError::recipient_not_found("queue record not found"))?;
        let entry = record
            .recipients
            .get_mut(&forward.recipient_did)
            .ok_or_else(|| RelayError::recipient_not_found("recipient state not found"))?;

        entry.transfer_state = TransferState::CommitReported;
        entry.last_transfer_change_ms = self.now_ms;
        match receipt.result {
            CommitResult::Delivered => {
                entry.state = RecipientState::Delivered;
                entry.retained_local_copy = false;
            }
            CommitResult::Failed => {
                entry.state = RecipientState::Failed;
            }
            CommitResult::Expired => {
                entry.state = RecipientState::Expired;
            }
        }

        Self::refresh_record_status(record);
        Ok(())
    }

    pub fn handoff_timeout_rollback(
        &mut self,
        from_did: &str,
        msg_id: &str,
        recipient_did: &str,
    ) -> Result<(), RelayError> {
        let record = self
            .records
            .get_mut(&(from_did.to_string(), msg_id.to_string()))
            .ok_or_else(|| RelayError::recipient_not_found("queue record not found"))?;
        let entry = record
            .recipients
            .get_mut(recipient_did)
            .ok_or_else(|| RelayError::recipient_not_found("recipient state not found"))?;

        if entry.transfer_state != TransferState::Pending {
            return Ok(());
        }

        let elapsed = self.now_ms.saturating_sub(entry.last_transfer_change_ms);
        if elapsed >= DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS {
            entry.transfer_state = TransferState::RolledBack;
            entry.last_transfer_change_ms = self.now_ms;
        }
        Ok(())
    }

    pub fn recipient_state(
        &self,
        from_did: &str,
        msg_id: &str,
        recipient_did: &str,
    ) -> Option<RecipientState> {
        self.records
            .get(&(from_did.to_string(), msg_id.to_string()))
            .and_then(|r| r.recipients.get(recipient_did))
            .map(|e| e.state)
    }

    pub fn transfer_state(
        &self,
        from_did: &str,
        msg_id: &str,
        recipient_did: &str,
    ) -> Option<TransferState> {
        self.records
            .get(&(from_did.to_string(), msg_id.to_string()))
            .and_then(|r| r.recipients.get(recipient_did))
            .map(|e| e.transfer_state)
    }

    pub fn retained_local_copy(
        &self,
        from_did: &str,
        msg_id: &str,
        recipient_did: &str,
    ) -> Option<bool> {
        self.records
            .get(&(from_did.to_string(), msg_id.to_string()))
            .and_then(|r| r.recipients.get(recipient_did))
            .map(|e| e.retained_local_copy)
    }

    pub fn record_status(&self, from_did: &str, msg_id: &str) -> Option<QueueStatus> {
        self.records
            .get(&(from_did.to_string(), msg_id.to_string()))
            .map(|r| r.status)
    }

    pub fn active_recipient_count(&self, from_did: &str, msg_id: &str) -> Option<usize> {
        self.records
            .get(&(from_did.to_string(), msg_id.to_string()))
            .map(|r| r.recipients.len())
    }

    fn refresh_record_status(record: &mut QueueRecord) {
        if !record.recipients.values().all(|e| e.state.is_terminal()) {
            return;
        }

        let any_expired = record
            .recipients
            .values()
            .any(|e| e.state == RecipientState::Expired);
        record.status = if any_expired {
            QueueStatus::Expired
        } else {
            QueueStatus::Done
        };
    }
}

pub fn split_for_federation(
    message: &Message,
    upstream_relay: &str,
    downstream_relay: &str,
    relay_path: &[String],
    hop_limit: u64,
    mode: TransferMode,
) -> Result<Vec<RelayForward>, RelayError> {
    if message.recipients.is_empty() {
        return Err(RelayError::recipient_not_found("message has no recipient"));
    }
    let mut out = Vec::with_capacity(message.recipients.len());
    for recipient in &message.recipients {
        out.push(RelayForward {
            fwd_v: FWD_V1,
            from_did: message.from_did.clone(),
            msg_id: message.msg_id.clone(),
            recipient_did: recipient.clone(),
            relay_path: relay_path.to_vec(),
            hop_limit,
            upstream_relay: upstream_relay.to_string(),
            downstream_relay: downstream_relay.to_string(),
            transfer_mode: mode,
        });
    }
    Ok(out)
}

pub fn compute_handoff_step(
    local_relay_id: &str,
    relay_path: &[String],
    hop_limit: u64,
) -> Result<(Vec<String>, u64), RelayError> {
    if relay_path.iter().any(|r| r == local_relay_id) {
        return Err(RelayError::relay_rejected("relay loop detected"));
    }
    if hop_limit == 0 {
        return Err(RelayError::relay_rejected("hop limit exhausted"));
    }

    let hop_limit_next = hop_limit - 1;
    let mut relay_path_next = relay_path.to_vec();
    relay_path_next.push(local_relay_id.to_string());
    Ok((relay_path_next, hop_limit_next))
}

pub fn validate_transfer_receipt(
    forward: &RelayForward,
    receipt: &TransferReceipt,
    supported_algs: &[i32],
) -> Result<(), RelayError> {
    if forward.fwd_v != FWD_V1 {
        return Err(RelayError::unsupported_version("unsupported relay-forward version"));
    }
    if receipt.receipt_v != RECEIPT_V1 {
        return Err(RelayError::unsupported_version("unsupported transfer receipt version"));
    }
    if !supported_algs.iter().any(|a| *a == receipt.alg) {
        return Err(RelayError::unauthorized("unsupported transfer receipt algorithm"));
    }
    if receipt.key_purpose != "assertionMethod" {
        return Err(RelayError::unauthorized(
            "transfer receipt key purpose must be assertionMethod",
        ));
    }
    if receipt.kid.is_empty() {
        return Err(RelayError::unauthorized("transfer receipt kid is required"));
    }
    if !receipt.accepted {
        return Err(RelayError::unauthorized("transfer receipt must indicate accepted=true"));
    }
    if receipt.msg_id != forward.msg_id
        || receipt.from_did != forward.from_did
        || receipt.recipient_did != forward.recipient_did
        || receipt.upstream_relay != forward.upstream_relay
        || receipt.downstream_relay != forward.downstream_relay
    {
        return Err(RelayError::unauthorized(
            "transfer receipt tuple mismatch against handoff context",
        ));
    }

    Ok(())
}

pub fn validate_commit_receipt(
    forward: &RelayForward,
    receipt: &CommitReceipt,
    supported_algs: &[i32],
) -> Result<(), RelayError> {
    if forward.fwd_v != FWD_V1 {
        return Err(RelayError::unsupported_version("unsupported relay-forward version"));
    }
    if receipt.commit_v != COMMIT_V1 {
        return Err(RelayError::unsupported_version("unsupported commit receipt version"));
    }
    if !supported_algs.iter().any(|a| *a == receipt.alg) {
        return Err(RelayError::unauthorized("unsupported commit receipt algorithm"));
    }
    if receipt.key_purpose != "assertionMethod" {
        return Err(RelayError::unauthorized(
            "commit receipt key purpose must be assertionMethod",
        ));
    }
    if receipt.kid.is_empty() {
        return Err(RelayError::unauthorized("commit receipt kid is required"));
    }
    if receipt.msg_id != forward.msg_id
        || receipt.from_did != forward.from_did
        || receipt.recipient_did != forward.recipient_did
        || receipt.upstream_relay != forward.upstream_relay
        || receipt.downstream_relay != forward.downstream_relay
    {
        return Err(RelayError::unauthorized(
            "commit receipt tuple mismatch against handoff context",
        ));
    }

    Ok(())
}
