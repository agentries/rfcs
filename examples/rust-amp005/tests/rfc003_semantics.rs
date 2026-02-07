use std::collections::HashMap;

use amp005_rfc003_tests::{
    compute_handoff_step, split_for_federation, CommitReceipt, CommitResult, QueueStatus, Relay,
    TransferMode, TransferReceipt, TransferState, COMMIT_V1, FWD_V1, RECEIPT_V1,
    DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS, DEFAULT_HANDOFF_MAX_ATTEMPTS,
};

fn sample_message(ttl_ms: u64, recipients: Vec<&str>) -> amp005_rfc003_tests::Message {
    amp005_rfc003_tests::Message {
        from_did: "did:web:example.com:agent:alice".to_string(),
        msg_id: "0000019c3520e44c0000000000000004".to_string(),
        recipients: recipients.into_iter().map(|v| v.to_string()).collect(),
        ts_ms: 1_707_055_200_000,
        ttl_ms,
    }
}

#[test]
fn rfc003_a1_ttl0_offline_rejection() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_000);
    let msg = sample_message(0, vec!["did:web:example.com:agent:bob"]);
    let mut online = HashMap::new();
    online.insert("did:web:example.com:agent:bob".to_string(), false);

    let err = relay.ingress(&msg, &online).expect_err("ttl=0 offline must reject");
    assert_eq!(err.code, 2003);
    assert_eq!(
        relay.active_recipient_count(&msg.from_did, &msg.msg_id),
        None
    );
}

#[test]
fn rfc003_a2_polling_redelivery_until_commit() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay
        .ingress(&msg, &HashMap::new())
        .expect("ingress should queue");

    let first = relay.poll("did:web:example.com:agent:bob");
    let second = relay.poll("did:web:example.com:agent:bob");
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(
        relay
            .recipient_state(&msg.from_did, &msg.msg_id, "did:web:example.com:agent:bob")
            .expect("recipient state"),
        amp005_rfc003_tests::RecipientState::Inflight
    );
}

#[test]
fn rfc003_a3_recipient_ack_commit() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay.ingress(&msg, &HashMap::new()).expect("ingress");
    relay
        .ack_recipient(&msg.from_did, &msg.msg_id, "did:web:example.com:agent:bob")
        .expect("ack");
    assert_eq!(
        relay.record_status(&msg.from_did, &msg.msg_id),
        Some(QueueStatus::Done)
    );
}

#[test]
fn rfc003_a4_multi_recipient_partial_commit() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(
        60_000,
        vec![
            "did:web:example.com:agent:bob",
            "did:web:example.com:agent:carol",
        ],
    );
    relay.ingress(&msg, &HashMap::new()).expect("ingress");
    relay
        .ack_recipient(&msg.from_did, &msg.msg_id, "did:web:example.com:agent:bob")
        .expect("ack A");

    assert_ne!(
        relay.record_status(&msg.from_did, &msg.msg_id),
        Some(QueueStatus::Done)
    );
}

#[test]
fn rfc003_a5_loop_rejection() {
    let err = compute_handoff_step(
        "did:web:relay:a",
        &["did:web:relay:x".to_string(), "did:web:relay:a".to_string()],
        8,
    )
    .expect_err("loop must reject");
    assert_eq!(err.code, 2003);
}

#[test]
fn rfc003_a6_hop_limit_exhaustion() {
    let err = compute_handoff_step("did:web:relay:a", &[], 0).expect_err("hop 0 must reject");
    assert_eq!(err.code, 2003);
}

#[test]
fn rfc003_a7_duplicate_suppression() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay.ingress(&msg, &HashMap::new()).expect("first ingress");
    relay.ingress(&msg, &HashMap::new()).expect("duplicate ingress");

    assert_eq!(
        relay.active_recipient_count(&msg.from_did, &msg.msg_id),
        Some(1)
    );
}

#[test]
fn rfc003_a8_transfer_rollback_on_timeout() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay.ingress(&msg, &HashMap::new()).expect("ingress");
    relay
        .start_handoff(
            &msg.from_did,
            &msg.msg_id,
            "did:web:example.com:agent:bob",
            "did:web:relay:b",
            TransferMode::Single,
        )
        .expect("start handoff");

    relay.set_now(1_707_055_200_100 + DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS + 1);
    relay
        .handoff_timeout_rollback(&msg.from_did, &msg.msg_id, "did:web:example.com:agent:bob")
        .expect("rollback");
    assert_eq!(
        relay.transfer_state(&msg.from_did, &msg.msg_id, "did:web:example.com:agent:bob"),
        Some(TransferState::RolledBack)
    );
}

#[test]
fn rfc003_a9_single_custody_acceptance_removes_local_copy() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay.ingress(&msg, &HashMap::new()).expect("ingress");

    let forward = split_for_federation(
        &msg,
        "did:web:relay:a",
        "did:web:relay:b",
        &[],
        8,
        TransferMode::Single,
    )
    .expect("split")
    .pop()
    .expect("single recipient forward");
    relay
        .start_handoff(
            &msg.from_did,
            &msg.msg_id,
            &forward.recipient_did,
            &forward.downstream_relay,
            forward.transfer_mode,
        )
        .expect("start handoff");

    let receipt = TransferReceipt {
        receipt_v: RECEIPT_V1,
        msg_id: msg.msg_id.clone(),
        from_did: msg.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        accepted_at: relay.now_ms,
        hop_limit_remaining: 7,
        accepted: true,
        alg: -8,
        kid: "did:web:relay:b#k1".to_string(),
        key_purpose: "assertionMethod".to_string(),
    };
    relay
        .apply_transfer_receipt(&forward, &receipt, &[-8, -7])
        .expect("receipt accepted");

    assert_eq!(
        relay.retained_local_copy(&msg.from_did, &msg.msg_id, &forward.recipient_did),
        Some(false)
    );
}

#[test]
fn rfc003_a10_and_a11_invalid_or_unsupported_receipt_rejected() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay.ingress(&msg, &HashMap::new()).expect("ingress");

    let forward = split_for_federation(
        &msg,
        "did:web:relay:a",
        "did:web:relay:b",
        &[],
        8,
        TransferMode::Single,
    )
    .expect("split")
    .pop()
    .expect("single recipient forward");

    let invalid_tuple = TransferReceipt {
        receipt_v: RECEIPT_V1,
        msg_id: "wrong".to_string(),
        from_did: msg.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        accepted_at: relay.now_ms,
        hop_limit_remaining: 7,
        accepted: true,
        alg: -8,
        kid: "did:web:relay:b#k1".to_string(),
        key_purpose: "assertionMethod".to_string(),
    };
    let err = relay
        .apply_transfer_receipt(&forward, &invalid_tuple, &[-8])
        .expect_err("tuple mismatch must reject");
    assert_eq!(err.code, 3001);

    let unsupported_alg = TransferReceipt {
        receipt_v: RECEIPT_V1,
        msg_id: msg.msg_id.clone(),
        from_did: msg.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        accepted_at: relay.now_ms,
        hop_limit_remaining: 7,
        accepted: true,
        alg: -35,
        kid: "did:web:relay:b#k1".to_string(),
        key_purpose: "assertionMethod".to_string(),
    };
    let err = relay
        .apply_transfer_receipt(&forward, &unsupported_alg, &[-8])
        .expect_err("unsupported alg must reject");
    assert_eq!(err.code, 3001);
}

#[test]
fn rfc003_a12_dual_custody_commit_receipt_positive() {
    let mut relay = Relay::new("did:web:relay:a", 1_707_055_200_100);
    let msg = sample_message(60_000, vec!["did:web:example.com:agent:bob"]);
    relay.ingress(&msg, &HashMap::new()).expect("ingress");

    let forward = split_for_federation(
        &msg,
        "did:web:relay:a",
        "did:web:relay:b",
        &[],
        8,
        TransferMode::Dual,
    )
    .expect("split")
    .pop()
    .expect("single recipient forward");
    relay
        .start_handoff(
            &msg.from_did,
            &msg.msg_id,
            &forward.recipient_did,
            &forward.downstream_relay,
            forward.transfer_mode,
        )
        .expect("start handoff");

    let receipt = TransferReceipt {
        receipt_v: RECEIPT_V1,
        msg_id: msg.msg_id.clone(),
        from_did: msg.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        accepted_at: relay.now_ms,
        hop_limit_remaining: 7,
        accepted: true,
        alg: -8,
        kid: "did:web:relay:b#k1".to_string(),
        key_purpose: "assertionMethod".to_string(),
    };
    relay
        .apply_transfer_receipt(&forward, &receipt, &[-8, -7])
        .expect("receipt accepted");

    let commit = CommitReceipt {
        commit_v: COMMIT_V1,
        msg_id: msg.msg_id.clone(),
        from_did: msg.from_did.clone(),
        recipient_did: forward.recipient_did.clone(),
        upstream_relay: forward.upstream_relay.clone(),
        downstream_relay: forward.downstream_relay.clone(),
        result: CommitResult::Delivered,
        committed_at: relay.now_ms + 1000,
        alg: -8,
        kid: "did:web:relay:b#k1".to_string(),
        key_purpose: "assertionMethod".to_string(),
    };
    relay
        .apply_commit_receipt(&forward, &commit, &[-8, -7])
        .expect("commit receipt");

    assert_eq!(
        relay.transfer_state(&msg.from_did, &msg.msg_id, &forward.recipient_did),
        Some(TransferState::CommitReported)
    );
    assert_eq!(
        relay.retained_local_copy(&msg.from_did, &msg.msg_id, &forward.recipient_did),
        Some(false)
    );
}

#[test]
fn rfc003_a13_multi_recipient_split() {
    let msg = sample_message(
        60_000,
        vec![
            "did:web:example.com:agent:bob",
            "did:web:example.com:agent:carol",
        ],
    );
    let forwards = split_for_federation(
        &msg,
        "did:web:relay:a",
        "did:web:relay:b",
        &[],
        8,
        TransferMode::Single,
    )
    .expect("split");

    assert_eq!(forwards.len(), 2);
    assert!(forwards.iter().all(|f| f.fwd_v == FWD_V1));
    assert_ne!(forwards[0].recipient_did, forwards[1].recipient_did);
}

#[test]
fn rfc003_mti_defaults_present() {
    assert_eq!(DEFAULT_HANDOFF_MAX_ATTEMPTS, 3);
    assert_eq!(DEFAULT_HANDOFF_ACCEPT_TIMEOUT_MS, 5_000);
}
