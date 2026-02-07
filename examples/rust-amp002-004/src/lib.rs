use amp001_example::{peek_routing, AmpError, RoutingEnvelope, TRANSPORT_WRAPPER_VERSION_V1};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PollResponse {
    pub messages: Vec<ByteBuf>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferMode {
    Single,
    Dual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayForward {
    pub fwd_v: u64,
    pub message: ByteBuf,
    pub from_did: String,
    pub recipient_did: String,
    pub relay_path: Vec<String>,
    pub hop_limit: u64,
    pub upstream_relay: String,
    pub transfer_mode: TransferMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayCommitReport {
    pub commit_v: u64,
    pub commit_receipt: ByteBuf,
}

pub fn decode_ws_binary_message_unit(payload: &[u8]) -> Result<RoutingEnvelope, AmpError> {
    if payload.is_empty() {
        return Err(AmpError::invalid_message(
            "websocket binary payload must not be empty",
        ));
    }
    peek_routing(payload)
}

pub fn reject_ws_text_message() -> AmpError {
    AmpError::invalid_message("websocket text messages are not supported for AMP")
}

pub fn decode_poll_response(bytes: &[u8]) -> Result<PollResponse, AmpError> {
    let wrapper: PollResponse = serde_cbor::from_slice(bytes)
        .map_err(|e| AmpError::invalid_message(format!("invalid poll wrapper: {e}")))?;

    for (idx, raw_msg) in wrapper.messages.iter().enumerate() {
        peek_routing(raw_msg.as_ref()).map_err(|e| {
            AmpError::invalid_message(format!(
                "poll wrapper messages[{idx}] is not a valid AMP message: {e}"
            ))
        })?;
    }

    Ok(wrapper)
}

pub fn decode_relay_forward(bytes: &[u8]) -> Result<RelayForward, AmpError> {
    let wrapper: RelayForward = serde_cbor::from_slice(bytes)
        .map_err(|e| AmpError::invalid_message(format!("invalid relay-forward wrapper: {e}")))?;

    if wrapper.fwd_v != TRANSPORT_WRAPPER_VERSION_V1 {
        return Err(AmpError::unsupported_version(format!(
            "unsupported relay-forward fwd_v={}, expected {}",
            wrapper.fwd_v, TRANSPORT_WRAPPER_VERSION_V1
        )));
    }
    if wrapper.hop_limit == 0 {
        return Err(AmpError::invalid_message(
            "relay-forward hop_limit must be > 0",
        ));
    }
    if wrapper.upstream_relay.is_empty() {
        return Err(AmpError::unauthorized(
            "relay-forward upstream_relay is required",
        ));
    }

    let routing = peek_routing(wrapper.message.as_ref())?;
    if routing.from != wrapper.from_did {
        return Err(AmpError::invalid_message(format!(
            "relay-forward from_did mismatch: wrapper={} message={}",
            wrapper.from_did, routing.from
        )));
    }
    if !routing.to.iter().any(|did| did == &wrapper.recipient_did) {
        return Err(AmpError::invalid_message(format!(
            "relay-forward recipient_did {} not found in message to field",
            wrapper.recipient_did
        )));
    }

    Ok(wrapper)
}

pub fn decode_relay_commit_report(bytes: &[u8]) -> Result<RelayCommitReport, AmpError> {
    let wrapper: RelayCommitReport = serde_cbor::from_slice(bytes)
        .map_err(|e| AmpError::invalid_message(format!("invalid relay-commit wrapper: {e}")))?;

    if wrapper.commit_v != TRANSPORT_WRAPPER_VERSION_V1 {
        return Err(AmpError::unsupported_version(format!(
            "unsupported relay-commit commit_v={}, expected {}",
            wrapper.commit_v, TRANSPORT_WRAPPER_VERSION_V1
        )));
    }
    if wrapper.commit_receipt.is_empty() {
        return Err(AmpError::invalid_message(
            "relay-commit commit_receipt must not be empty",
        ));
    }

    Ok(wrapper)
}

pub fn validate_strict_principal_binding(
    transport_principal_did: &str,
    amp_from_did: &str,
) -> Result<(), AmpError> {
    if transport_principal_did != amp_from_did {
        return Err(AmpError::unauthorized(format!(
            "strict binding failed: principal={} from={}",
            transport_principal_did, amp_from_did
        )));
    }
    Ok(())
}

pub fn validate_relay_forward_principal_binding(
    transport_principal_did: &str,
    wrapper: &RelayForward,
) -> Result<(), AmpError> {
    if transport_principal_did != wrapper.upstream_relay {
        return Err(AmpError::unauthorized(format!(
            "relay-forward binding failed: principal={} upstream_relay={}",
            transport_principal_did, wrapper.upstream_relay
        )));
    }
    Ok(())
}

pub fn validate_relay_commit_principal_binding(
    transport_principal_did: &str,
    commit_receipt_downstream_relay: &str,
) -> Result<(), AmpError> {
    if transport_principal_did != commit_receipt_downstream_relay {
        return Err(AmpError::unauthorized(format!(
            "relay-commit binding failed: principal={} downstream_relay={}",
            transport_principal_did, commit_receipt_downstream_relay
        )));
    }
    Ok(())
}
