use std::io;
use std::io::Cursor;

use amp001_example::{
    build_authcrypt_signed, demo_agents, make_message_id, write_frame, MessageMeta, Recipients,
    TextMessageBody, TRANSPORT_WRAPPER_VERSION_V1, TYPE_MESSAGE, now_ms, read_frame,
};
use amp002_004_tests::{
    decode_poll_response, decode_relay_commit_report, decode_relay_forward,
    decode_ws_binary_message_unit, reject_ws_text_message, validate_relay_commit_principal_binding,
    validate_relay_forward_principal_binding, validate_strict_principal_binding, PollResponse,
    RelayCommitReport, RelayForward, TransferMode,
};
use serde_bytes::ByteBuf;

#[test]
fn rfc002_tcp_frame_boundary_checks() {
    let payload = b"amp-transport-002".to_vec();
    let mut framed = Vec::new();
    write_frame(&mut framed, &payload).expect("write frame");
    framed.pop();

    let mut cursor = Cursor::new(framed);
    let err = read_frame(&mut cursor).expect_err("truncated frame must fail");
    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn rfc002_websocket_mapping_rules() {
    let demo = demo_agents();
    let resolver = demo.resolver();
    let ts = now_ms();
    let body = TextMessageBody {
        msg: "ws-binary".to_string(),
    };
    let meta = MessageMeta {
        v: 1,
        id: make_message_id(ts, 6),
        typ: TYPE_MESSAGE,
        ts_ms: ts,
        ttl_ms: 86_400_000,
        from: String::new(),
        to: Recipients::One(demo.bob.did.clone()),
        reply_to: None,
        thread_id: None,
    };

    let wire = build_authcrypt_signed(&demo.alice, &demo.bob.did, meta, &body, &resolver)
        .expect("build authcrypt");
    let routing = decode_ws_binary_message_unit(&wire).expect("ws binary mapping");
    assert_eq!(routing.from, demo.alice.did);
    assert_eq!(routing.to, vec![demo.bob.did]);

    let err = reject_ws_text_message();
    assert_eq!(err.code, 1001);
}

#[test]
fn rfc002_http_polling_and_relay_forward_wrapper_validation() {
    let demo = demo_agents();
    let resolver = demo.resolver();
    let ts = now_ms();
    let body = TextMessageBody {
        msg: "http-wrapper".to_string(),
    };
    let meta = MessageMeta {
        v: 1,
        id: make_message_id(ts, 7),
        typ: TYPE_MESSAGE,
        ts_ms: ts,
        ttl_ms: 86_400_000,
        from: String::new(),
        to: Recipients::One(demo.bob.did.clone()),
        reply_to: None,
        thread_id: None,
    };
    let wire = build_authcrypt_signed(&demo.alice, &demo.bob.did, meta, &body, &resolver)
        .expect("build authcrypt");

    let poll = PollResponse {
        messages: vec![ByteBuf::from(wire.clone())],
        next_cursor: Some("cur-1".to_string()),
        has_more: false,
    };
    let poll_bytes = serde_cbor::to_vec(&poll).expect("encode poll wrapper");
    let decoded_poll = decode_poll_response(&poll_bytes).expect("decode poll wrapper");
    assert_eq!(decoded_poll.messages.len(), 1);

    let relay_forward = RelayForward {
        fwd_v: TRANSPORT_WRAPPER_VERSION_V1,
        message: ByteBuf::from(wire.clone()),
        from_did: demo.alice.did.clone(),
        recipient_did: demo.bob.did.clone(),
        relay_path: vec!["did:web:example.com:relay:a".to_string()],
        hop_limit: 8,
        upstream_relay: "did:web:example.com:relay:a".to_string(),
        transfer_mode: TransferMode::Single,
    };
    let rf_bytes = serde_cbor::to_vec(&relay_forward).expect("encode relay-forward");
    let parsed = decode_relay_forward(&rf_bytes).expect("decode relay-forward");
    assert_eq!(parsed.fwd_v, TRANSPORT_WRAPPER_VERSION_V1);
    assert_eq!(parsed.recipient_did, demo.bob.did);

    let mut unsupported = relay_forward;
    unsupported.fwd_v = 2;
    let unsupported_bytes = serde_cbor::to_vec(&unsupported).expect("encode unsupported");
    let err = decode_relay_forward(&unsupported_bytes).expect_err("fwd_v must be rejected");
    assert_eq!(err.code, 1004);
}

#[test]
fn rfc002_http_relay_commit_wrapper_validation() {
    let commit = RelayCommitReport {
        commit_v: TRANSPORT_WRAPPER_VERSION_V1,
        commit_receipt: ByteBuf::from(vec![0xa1, 0x01, 0x02]),
    };
    let encoded = serde_cbor::to_vec(&commit).expect("encode commit wrapper");
    let parsed = decode_relay_commit_report(&encoded).expect("decode commit wrapper");
    assert_eq!(parsed.commit_v, TRANSPORT_WRAPPER_VERSION_V1);

    let mut unsupported = commit.clone();
    unsupported.commit_v = 3;
    let unsupported_bytes = serde_cbor::to_vec(&unsupported).expect("encode unsupported");
    let err = decode_relay_commit_report(&unsupported_bytes).expect_err("unsupported commit_v");
    assert_eq!(err.code, 1004);

    let empty_receipt = RelayCommitReport {
        commit_v: TRANSPORT_WRAPPER_VERSION_V1,
        commit_receipt: ByteBuf::from(vec![]),
    };
    let empty_bytes = serde_cbor::to_vec(&empty_receipt).expect("encode empty");
    let err = decode_relay_commit_report(&empty_bytes).expect_err("empty commit_receipt");
    assert_eq!(err.code, 1001);
}

#[test]
fn rfc002_principal_binding_rules() {
    validate_strict_principal_binding(
        "did:web:example.com:agent:alice",
        "did:web:example.com:agent:alice",
    )
    .expect("strict binding should pass");
    let err = validate_strict_principal_binding(
        "did:web:example.com:agent:alice",
        "did:web:example.com:agent:bob",
    )
    .expect_err("strict binding mismatch");
    assert_eq!(err.code, 3001);

    let wrapper = RelayForward {
        fwd_v: TRANSPORT_WRAPPER_VERSION_V1,
        message: ByteBuf::from(vec![0xa1, 0x61, 0x76, 0x01]), // placeholder bytes for binding-only check
        from_did: "did:web:example.com:agent:alice".to_string(),
        recipient_did: "did:web:example.com:agent:bob".to_string(),
        relay_path: vec![],
        hop_limit: 8,
        upstream_relay: "did:web:example.com:relay:a".to_string(),
        transfer_mode: TransferMode::Single,
    };
    validate_relay_forward_principal_binding("did:web:example.com:relay:a", &wrapper)
        .expect("relay-forward binding should pass");
    let err = validate_relay_forward_principal_binding("did:web:example.com:relay:x", &wrapper)
        .expect_err("relay-forward binding mismatch");
    assert_eq!(err.code, 3001);

    validate_relay_commit_principal_binding(
        "did:web:example.com:relay:b",
        "did:web:example.com:relay:b",
    )
    .expect("relay-commit binding should pass");
    let err = validate_relay_commit_principal_binding(
        "did:web:example.com:relay:b",
        "did:web:example.com:relay:c",
    )
    .expect_err("relay-commit binding mismatch");
    assert_eq!(err.code, 3001);
}
