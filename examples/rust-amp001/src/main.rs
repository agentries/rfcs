use amp001_example::{
    build_authcrypt_signed, build_plain_signed, cbor_map_string_pairs, hex_encode, make_message_id, now_ms,
    receive_and_verify, select_compatible_version, validate_ack_semantics, AckBody, AckSource, AgentKeys,
    DidResolver, HelloBody, MessageMeta, Recipients, TextMessageBody, TYPE_ACK, TYPE_HELLO, TYPE_HELLO_ACK,
    TYPE_MESSAGE,
};
use serde_cbor::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alice = AgentKeys::from_sign_seed("did:web:example.com:agent:alice", [1_u8; 32]);
    let bob = AgentKeys::from_sign_seed("did:web:example.com:agent:bob", [2_u8; 32]);

    let mut resolver = DidResolver::default();
    resolver.add_agent(&alice);
    resolver.add_agent(&bob);

    let mut counter = 1_u64;
    let base_ts = now_ms();

    // 1) HELLO: Alice -> Bob
    let hello_body = HelloBody {
        versions: vec!["0.30.0".to_string(), "1.0.0".to_string()],
    };
    let hello_meta = MessageMeta {
        v: 1,
        id: make_message_id(base_ts, counter),
        typ: TYPE_HELLO,
        ts_ms: base_ts,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(bob.did.clone()),
        reply_to: None,
        thread_id: None,
    };
    counter += 1;

    let hello_wire = build_plain_signed(&alice, hello_meta, &hello_body)?;
    let hello_rx = receive_and_verify(&bob, &hello_wire, &resolver, base_ts + 50)?;
    let hello_decoded: HelloBody = hello_rx.decode_body()?;

    // 2) HELLO_ACK: Bob -> Alice
    let selected = select_compatible_version(
        &["0.30.0".to_string(), "1.0.0".to_string()],
        &hello_decoded.versions,
    )
    .ok_or("no compatible version")?;

    let hello_ack_body = cbor_map_string_pairs(&[("selected", Value::Text(selected.clone()))]);
    let hello_ack_meta = MessageMeta {
        v: 1,
        id: make_message_id(base_ts + 10, counter),
        typ: TYPE_HELLO_ACK,
        ts_ms: base_ts + 10,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(alice.did.clone()),
        reply_to: Some(hello_rx.meta.id),
        thread_id: None,
    };
    counter += 1;

    let hello_ack_wire = build_plain_signed(&bob, hello_ack_meta, &hello_ack_body)?;
    let _hello_ack_rx = receive_and_verify(&alice, &hello_ack_wire, &resolver, base_ts + 60)?;

    // 3) Encrypted MESSAGE: Alice -> Bob (authcrypt)
    let secret_body = TextMessageBody {
        msg: "pay invoice #42".to_string(),
    };
    let secret_meta = MessageMeta {
        v: 1,
        id: make_message_id(base_ts + 20, counter),
        typ: TYPE_MESSAGE,
        ts_ms: base_ts + 20,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(bob.did.clone()),
        reply_to: None,
        thread_id: None,
    };
    counter += 1;

    let secret_wire = build_authcrypt_signed(&alice, &bob.did, secret_meta, &secret_body, &resolver)?;
    let secret_rx = receive_and_verify(&bob, &secret_wire, &resolver, base_ts + 70)?;
    let secret_decoded: TextMessageBody = secret_rx.decode_body()?;

    // 4) ACK: Bob -> Alice
    let ack_body = AckBody {
        ack_source: AckSource::Recipient,
        received_at: base_ts + 80,
        ack_target: None,
    };
    let ack_meta = MessageMeta {
        v: 1,
        id: make_message_id(base_ts + 30, counter),
        typ: TYPE_ACK,
        ts_ms: base_ts + 30,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(alice.did.clone()),
        reply_to: Some(secret_rx.meta.id),
        thread_id: None,
    };

    let ack_wire = build_plain_signed(&bob, ack_meta, &ack_body)?;
    let ack_rx = receive_and_verify(&alice, &ack_wire, &resolver, base_ts + 90)?;
    validate_ack_semantics(&ack_rx, &[bob.did.clone()], &resolver)?;

    println!("HELLO versions from Alice: {:?}", hello_decoded.versions);
    println!("Selected version: {selected}");
    println!("Encrypted MESSAGE body decoded by Bob: {}", secret_decoded.msg);
    println!("HELLO wire bytes: {}", hello_wire.len());
    println!("Encrypted wire bytes: {}", secret_wire.len());
    println!(
        "Encrypted wire prefix (hex): {}",
        hex_encode(&secret_wire[..secret_wire.len().min(48)])
    );
    println!("ACK verification: passed");

    Ok(())
}
