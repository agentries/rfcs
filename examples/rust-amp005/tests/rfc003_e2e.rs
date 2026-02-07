use std::collections::HashMap;

use amp005_rfc003_tests::{
    compute_handoff_step, split_for_federation, CommitReceipt, CommitResult, Message, QueueStatus,
    RecipientState, Relay, TransferMode, TransferReceipt, TransferState, COMMIT_V1,
    DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS, FWD_V1, RECEIPT_V1,
};

const ALICE: &str = "did:web:example.com:agent:alice";
const BOB: &str = "did:web:example.com:agent:bob";
const CAROL: &str = "did:web:example.com:agent:carol";
const RELAY_A: &str = "did:web:example.com:relay:a";
const RELAY_B: &str = "did:web:example.com:relay:b";

fn message(msg_id: &str, ttl_ms: u64, recipients: &[&str]) -> Message {
    Message {
        from_did: ALICE.to_string(),
        msg_id: msg_id.to_string(),
        recipients: recipients.iter().map(|v| (*v).to_string()).collect(),
        ts_ms: 1_707_055_200_000,
        ttl_ms,
    }
}

fn transfer_receipt(forward: &amp005_rfc003_tests::RelayForward) -> TransferReceipt {
    TransferReceipt {
        receipt_v: RECEIPT_V1,
        msg_id: forward.msg_id.clone(),
        from_did: forward.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        accepted_at: 1_707_055_200_500,
        hop_limit_remaining: 7,
        accepted: true,
        alg: -8,
        kid: format!("{}#k1", forward.downstream_relay),
        key_purpose: "assertionMethod".to_string(),
    }
}

fn commit_receipt(forward: &amp005_rfc003_tests::RelayForward, result: CommitResult) -> CommitReceipt {
    CommitReceipt {
        commit_v: COMMIT_V1,
        msg_id: forward.msg_id.clone(),
        from_did: forward.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        result,
        committed_at: 1_707_055_201_000,
        alg: -8,
        kid: format!("{}#k1", forward.downstream_relay),
        key_purpose: "assertionMethod".to_string(),
    }
}

#[test]
fn rfc003_e2e_store_forward_poll_and_ack() {
    let mut relay = Relay::new(RELAY_A, 1_707_055_200_100);
    let msg = message("m-001", 60_000, &[BOB]);

    relay.ingress(&msg, &HashMap::new()).expect("ingress queued");

    let first = relay.poll(BOB);
    assert_eq!(first.len(), 1);
    let second = relay.poll(BOB);
    assert_eq!(second.len(), 1, "must redeliver before commit");

    relay
        .ack_recipient(ALICE, "m-001", BOB)
        .expect("recipient ack commit");

    assert_eq!(relay.poll(BOB).len(), 0, "delivered record must stop redelivery");
    assert_eq!(relay.record_status(ALICE, "m-001"), Some(QueueStatus::Done));
}

#[test]
fn rfc003_e2e_multi_recipient_independent_commit() {
    let mut relay = Relay::new(RELAY_A, 1_707_055_200_100);
    let msg = message("m-002", 60_000, &[BOB, CAROL]);
    relay.ingress(&msg, &HashMap::new()).expect("ingress queued");

    relay.ack_recipient(ALICE, "m-002", BOB).expect("bob ack");
    assert_eq!(relay.record_status(ALICE, "m-002"), Some(QueueStatus::Queued));

    relay
        .ack_recipient(ALICE, "m-002", CAROL)
        .expect("carol ack");
    assert_eq!(relay.record_status(ALICE, "m-002"), Some(QueueStatus::Done));
}

#[test]
fn rfc003_e2e_ttl0_requires_immediate_next_hop() {
    let mut relay = Relay::new(RELAY_A, 1_707_055_200_000);
    let msg = message("m-003", 0, &[BOB]);

    let mut online = HashMap::new();
    online.insert(BOB.to_string(), false);
    let err = relay
        .ingress(&msg, &online)
        .expect_err("ttl=0 + offline must reject");
    assert_eq!(err.code, 2003);

    online.insert(BOB.to_string(), true);
    relay.ingress(&msg, &online).expect("ttl=0 + online accepted");
    assert_eq!(relay.active_recipient_count(ALICE, "m-003"), None);
}

#[test]
fn rfc003_e2e_duplicate_suppression_and_expiry() {
    let mut relay = Relay::new(RELAY_A, 1_707_055_200_100);
    let msg = message("m-004", 2_000, &[BOB]);

    relay.ingress(&msg, &HashMap::new()).expect("first ingress");
    relay.ingress(&msg, &HashMap::new()).expect("duplicate ingress");
    assert_eq!(relay.active_recipient_count(ALICE, "m-004"), Some(1));

    relay.set_now(msg.ts_ms + msg.ttl_ms + 1);
    relay.expire();
    assert_eq!(
        relay.recipient_state(ALICE, "m-004", BOB),
        Some(RecipientState::Expired)
    );
    assert_eq!(relay.record_status(ALICE, "m-004"), Some(QueueStatus::Expired));
}

#[test]
fn rfc003_e2e_federation_single_custody_transfer() {
    let mut upstream = Relay::new(RELAY_A, 1_707_055_200_100);
    let mut downstream = Relay::new(RELAY_B, 1_707_055_200_200);
    let msg = message("m-005", 60_000, &[BOB]);

    upstream.ingress(&msg, &HashMap::new()).expect("upstream ingress");

    let mut forwards = split_for_federation(&msg, RELAY_A, RELAY_B, &[], 8, TransferMode::Single)
        .expect("split for federation");
    let forward = forwards.pop().expect("single recipient forward");
    assert_eq!(forward.fwd_v, FWD_V1);

    let (path_next, hop_next) = compute_handoff_step(RELAY_A, &forward.relay_path, forward.hop_limit)
        .expect("compute handoff step");
    assert_eq!(path_next, vec![RELAY_A.to_string()]);
    assert_eq!(hop_next, 7);

    upstream
        .start_handoff(ALICE, "m-005", BOB, RELAY_B, TransferMode::Single)
        .expect("start handoff");

    downstream
        .ingress(&message("m-005", 60_000, &[BOB]), &HashMap::new())
        .expect("downstream ingress");

    let receipt = transfer_receipt(&forward);
    upstream
        .apply_transfer_receipt(&forward, &receipt, &[-8, -7])
        .expect("transfer receipt accepted");

    assert_eq!(
        upstream.transfer_state(ALICE, "m-005", BOB),
        Some(TransferState::Accepted)
    );
    assert_eq!(upstream.retained_local_copy(ALICE, "m-005", BOB), Some(false));
}

#[test]
fn rfc003_e2e_federation_dual_custody_commit_feedback() {
    let mut upstream = Relay::new(RELAY_A, 1_707_055_200_100);
    let mut downstream = Relay::new(RELAY_B, 1_707_055_200_200);
    let msg = message("m-006", 60_000, &[BOB]);

    upstream.ingress(&msg, &HashMap::new()).expect("upstream ingress");
    downstream.ingress(&msg, &HashMap::new()).expect("downstream ingress");

    let mut forwards = split_for_federation(&msg, RELAY_A, RELAY_B, &[], 8, TransferMode::Dual)
        .expect("split dual");
    let forward = forwards.pop().expect("single recipient forward");

    upstream
        .start_handoff(ALICE, "m-006", BOB, RELAY_B, TransferMode::Dual)
        .expect("start handoff");

    let receipt = transfer_receipt(&forward);
    upstream
        .apply_transfer_receipt(&forward, &receipt, &[-8, -7])
        .expect("transfer receipt accepted");

    assert_eq!(upstream.retained_local_copy(ALICE, "m-006", BOB), Some(true));

    downstream
        .ack_recipient(ALICE, "m-006", BOB)
        .expect("downstream recipient ack");

    let commit = commit_receipt(&forward, CommitResult::Delivered);
    upstream
        .apply_commit_receipt(&forward, &commit, &[-8, -7])
        .expect("commit receipt accepted");

    assert_eq!(
        upstream.transfer_state(ALICE, "m-006", BOB),
        Some(TransferState::CommitReported)
    );
    assert_eq!(upstream.retained_local_copy(ALICE, "m-006", BOB), Some(false));
    assert_eq!(upstream.record_status(ALICE, "m-006"), Some(QueueStatus::Done));
}

#[test]
fn rfc003_e2e_federation_rollback_and_negative_paths() {
    let mut upstream = Relay::new(RELAY_A, 1_707_055_200_100);
    let msg = message("m-007", 60_000, &[BOB]);
    upstream.ingress(&msg, &HashMap::new()).expect("ingress");

    let forward = split_for_federation(&msg, RELAY_A, RELAY_B, &[], 8, TransferMode::Single)
        .expect("split")
        .pop()
        .expect("forward");

    upstream
        .start_handoff(ALICE, "m-007", BOB, RELAY_B, TransferMode::Single)
        .expect("start handoff");

    upstream.set_now(1_707_055_200_100 + DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS + 1);
    upstream
        .handoff_timeout_rollback(ALICE, "m-007", BOB)
        .expect("rollback");
    assert_eq!(
        upstream.transfer_state(ALICE, "m-007", BOB),
        Some(TransferState::RolledBack)
    );

    let loop_err = compute_handoff_step(RELAY_A, &[RELAY_A.to_string()], 8)
        .expect_err("loop must reject");
    assert_eq!(loop_err.code, 2003);

    let hop_err = compute_handoff_step(RELAY_A, &[], 0).expect_err("hop=0 must reject");
    assert_eq!(hop_err.code, 2003);

    let mut invalid = transfer_receipt(&forward);
    invalid.msg_id = "wrong-msg-id".to_string();
    let tuple_err = upstream
        .apply_transfer_receipt(&forward, &invalid, &[-8])
        .expect_err("tuple mismatch must reject");
    assert_eq!(tuple_err.code, 3001);

    let mut unsupported_alg = transfer_receipt(&forward);
    unsupported_alg.alg = -35;
    let alg_err = upstream
        .apply_transfer_receipt(&forward, &unsupported_alg, &[-8])
        .expect_err("unsupported alg must reject");
    assert_eq!(alg_err.code, 3001);

    let mut unsupported_version = transfer_receipt(&forward);
    unsupported_version.receipt_v = 9;
    let ver_err = upstream
        .apply_transfer_receipt(&forward, &unsupported_version, &[-8, -7])
        .expect_err("unsupported version must reject");
    assert_eq!(ver_err.code, 1004);
}

#[test]
fn rfc003_e2e_multi_recipient_federation_split() {
    let msg = message("m-008", 60_000, &[BOB, CAROL]);
    let forwards = split_for_federation(&msg, RELAY_A, RELAY_B, &[], 8, TransferMode::Single)
        .expect("split federation forwards");

    assert_eq!(forwards.len(), 2);
    assert!(forwards.iter().all(|f| f.fwd_v == FWD_V1));
    assert!(forwards.iter().any(|f| f.recipient_did == BOB));
    assert!(forwards.iter().any(|f| f.recipient_did == CAROL));
}
