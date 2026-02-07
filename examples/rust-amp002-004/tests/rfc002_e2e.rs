use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use amp001_example::{
    build_authcrypt_signed, build_plain_signed, demo_agents, make_message_id, now_ms, read_frame,
    receive_and_verify, write_frame, HelloBody, MessageMeta, Recipients, TextMessageBody,
    TYPE_HELLO, TYPE_MESSAGE,
};
use amp002_004_tests::{
    decode_relay_commit_report, decode_relay_forward, validate_relay_commit_principal_binding,
    validate_relay_forward_principal_binding, PollResponse, RelayCommitReport, RelayForward,
    TransferMode,
};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForwardResponse {
    accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt: Option<ByteBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommitResponse {
    accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;

    let mut buf = Vec::new();
    let mut temp = [0_u8; 1024];
    let header_end;
    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed before header",
            ));
        }
        buf.extend_from_slice(&temp[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
    }

    let header_text = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = header_text.split("\r\n").filter(|v| !v.is_empty());
    let request_line = lines
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "missing request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let content_len = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = buf[header_end..].to_vec();
    while body.len() < content_len {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }
    body.truncate(content_len);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
) -> std::io::Result<()> {
    let mut resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status,
        reason,
        body.len()
    );
    for (k, v) in extra_headers {
        resp.push_str(k);
        resp.push_str(": ");
        resp.push_str(v);
        resp.push_str("\r\n");
    }
    resp.push_str("\r\n");

    stream.write_all(resp.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn send_http_request(
    addr: &str,
    method: &str,
    path: &str,
    headers: &[(&str, String)],
    body: &[u8],
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let mut req = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        method,
        path,
        addr,
        body.len()
    );
    for (k, v) in headers {
        req.push_str(k);
        req.push_str(": ");
        req.push_str(v);
        req.push_str("\r\n");
    }
    req.push_str("\r\n");

    stream.write_all(req.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let header_end = find_subslice(&buf, b"\r\n\r\n").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid http response")
    })? + 4;

    let header_text = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = header_text.split("\r\n").filter(|v| !v.is_empty());
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(0);

    let mut parsed_headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            parsed_headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let content_len = parsed_headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body_bytes = buf[header_end..].to_vec();
    body_bytes.truncate(content_len);

    Ok(HttpResponse {
        status,
        headers: parsed_headers,
        body: body_bytes,
    })
}

#[test]
fn rfc002_e2e_tcp_forward_between_two_clients() {
    let demo = demo_agents();
    let resolver = demo.resolver();

    let (tx, rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind tcp relay");
        let addr = listener.local_addr().expect("local addr").to_string();
        tx.send(addr).expect("send addr");

        let (mut bob_stream, _) = listener.accept().expect("accept bob");
        let bob_reg = read_frame(&mut bob_stream).expect("bob registration frame");
        let bob_routing = amp001_example::peek_routing(&bob_reg).expect("bob routing");
        assert!(bob_routing.from.contains(":bob"));

        let (mut alice_stream, _) = listener.accept().expect("accept alice");
        let alice_reg = read_frame(&mut alice_stream).expect("alice registration frame");
        let alice_routing = amp001_example::peek_routing(&alice_reg).expect("alice routing");
        assert!(alice_routing.from.contains(":alice"));

        let msg_frame = read_frame(&mut alice_stream).expect("message frame from alice");
        let msg_routing = amp001_example::peek_routing(&msg_frame).expect("msg routing");
        assert!(msg_routing.to.iter().any(|v| v.contains(":bob")));

        write_frame(&mut bob_stream, &msg_frame).expect("forward to bob");
    });

    let addr = rx.recv().expect("recv addr");

    let mut bob_conn = TcpStream::connect(&addr).expect("bob connect");
    bob_conn
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set timeout");

    let hello_bob = build_plain_signed(
        &demo.bob,
        MessageMeta {
            v: 1,
            id: make_message_id(now_ms(), 1),
            typ: TYPE_HELLO,
            ts_ms: now_ms(),
            ttl_ms: 60_000,
            from: String::new(),
            to: Recipients::One("did:web:example.com:relay:main".to_string()),
            reply_to: None,
            thread_id: None,
        },
        &HelloBody {
            versions: vec!["0.30.0".to_string()],
        },
    )
    .expect("build bob hello");
    write_frame(&mut bob_conn, &hello_bob).expect("send bob hello");

    let mut alice_conn = TcpStream::connect(&addr).expect("alice connect");
    let hello_alice = build_plain_signed(
        &demo.alice,
        MessageMeta {
            v: 1,
            id: make_message_id(now_ms(), 2),
            typ: TYPE_HELLO,
            ts_ms: now_ms(),
            ttl_ms: 60_000,
            from: String::new(),
            to: Recipients::One("did:web:example.com:relay:main".to_string()),
            reply_to: None,
            thread_id: None,
        },
        &HelloBody {
            versions: vec!["0.30.0".to_string()],
        },
    )
    .expect("build alice hello");
    write_frame(&mut alice_conn, &hello_alice).expect("send alice hello");

    let wire = build_authcrypt_signed(
        &demo.alice,
        &demo.bob.did,
        MessageMeta {
            v: 1,
            id: make_message_id(now_ms(), 3),
            typ: TYPE_MESSAGE,
            ts_ms: now_ms(),
            ttl_ms: 60_000,
            from: String::new(),
            to: Recipients::One(demo.bob.did.clone()),
            reply_to: None,
            thread_id: None,
        },
        &TextMessageBody {
            msg: "tcp-e2e".to_string(),
        },
        &resolver,
    )
    .expect("build message");
    write_frame(&mut alice_conn, &wire).expect("send message");

    let forwarded = read_frame(&mut bob_conn).expect("read forwarded");
    let received = receive_and_verify(&demo.bob, &forwarded, &resolver, now_ms()).expect("verify");
    let body: TextMessageBody = received.decode_body().expect("decode");
    assert_eq!(body.msg, "tcp-e2e");

    server.join().expect("server thread");
}

#[test]
fn rfc002_e2e_http_submit_then_poll() {
    let demo = demo_agents();
    let resolver = demo.resolver();

    let (tx, rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind http relay");
        let addr = listener.local_addr().expect("local addr").to_string();
        tx.send(addr).expect("send addr");

        let mut queued: Vec<Vec<u8>> = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept http");
            let req = read_http_request(&mut stream).expect("read request");

            match (req.method.as_str(), req.path.as_str()) {
                ("POST", "/amp/v1/messages") => {
                    amp001_example::peek_routing(&req.body).expect("valid amp payload");
                    queued.push(req.body);
                    write_http_response(&mut stream, 202, "Accepted", &[], &[]).expect("write 202");
                }
                ("GET", "/amp/v1/messages?cursor=cur-0&limit=50") => {
                    let wrapper = PollResponse {
                        messages: queued.iter().cloned().map(ByteBuf::from).collect(),
                        next_cursor: None,
                        has_more: false,
                    };
                    let body = serde_cbor::to_vec(&wrapper).expect("encode poll wrapper");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &body,
                        &[("Content-Type", "application/cbor")],
                    )
                    .expect("write 200");
                }
                _ => {
                    write_http_response(&mut stream, 404, "Not Found", &[], &[]).expect("write 404");
                }
            }
        }
    });

    let addr = rx.recv().expect("recv addr");
    let wire = build_authcrypt_signed(
        &demo.alice,
        &demo.bob.did,
        MessageMeta {
            v: 1,
            id: make_message_id(now_ms(), 10),
            typ: TYPE_MESSAGE,
            ts_ms: now_ms(),
            ttl_ms: 60_000,
            from: String::new(),
            to: Recipients::One(demo.bob.did.clone()),
            reply_to: None,
            thread_id: None,
        },
        &TextMessageBody {
            msg: "http-e2e".to_string(),
        },
        &resolver,
    )
    .expect("build wire");

    let post_resp = send_http_request(
        &addr,
        "POST",
        "/amp/v1/messages",
        &[("Content-Type", "application/cbor".to_string())],
        &wire,
    )
    .expect("post");
    assert_eq!(post_resp.status, 202);
    assert!(post_resp.headers.contains_key("content-length"));

    let get_resp = send_http_request(
        &addr,
        "GET",
        "/amp/v1/messages?cursor=cur-0&limit=50",
        &[("Accept", "application/cbor".to_string())],
        &[],
    )
    .expect("get");
    assert_eq!(get_resp.status, 200);

    let poll = amp002_004_tests::decode_poll_response(&get_resp.body).expect("decode poll");
    assert_eq!(poll.messages.len(), 1);
    let received = receive_and_verify(&demo.bob, poll.messages[0].as_ref(), &resolver, now_ms())
        .expect("verify polled payload");
    let body: TextMessageBody = received.decode_body().expect("decode body");
    assert_eq!(body.msg, "http-e2e");

    server.join().expect("server thread");
}

#[test]
fn rfc002_e2e_http_relay_forward_and_commit_with_principal_binding() {
    let demo = demo_agents();
    let resolver = demo.resolver();

    let (tx, rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind http relay");
        let addr = listener.local_addr().expect("local addr").to_string();
        tx.send(addr).expect("send addr");

        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept http");
            let req = read_http_request(&mut stream).expect("read request");
            let principal = req
                .headers
                .get("x-principal-did")
                .cloned()
                .unwrap_or_default();

            match (req.method.as_str(), req.path.as_str()) {
                ("POST", "/amp/v1/relay/forward") => {
                    let forward = decode_relay_forward(&req.body).expect("decode forward");
                    validate_relay_forward_principal_binding(&principal, &forward)
                        .expect("forward principal binding");
                    let body = serde_cbor::to_vec(&ForwardResponse {
                        accepted: true,
                        receipt: Some(ByteBuf::from(vec![0xa1, 0x01, 0x02])),
                        error_code: None,
                        error_message: None,
                    })
                    .expect("encode forward response");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &body,
                        &[("Content-Type", "application/cbor")],
                    )
                    .expect("write forward response");
                }
                ("POST", "/amp/v1/relay/commit") => {
                    let report = decode_relay_commit_report(&req.body).expect("decode commit");
                    let parsed: serde_cbor::Value =
                        serde_cbor::from_slice(report.commit_receipt.as_ref()).expect("decode receipt map");
                    let downstream = extract_map_text(&parsed, "downstream_relay")
                        .expect("downstream_relay in commit receipt");
                    validate_relay_commit_principal_binding(&principal, &downstream)
                        .expect("commit principal binding");

                    let body = serde_cbor::to_vec(&CommitResponse {
                        accepted: true,
                        error_code: None,
                        error_message: None,
                    })
                    .expect("encode commit response");
                    write_http_response(
                        &mut stream,
                        200,
                        "OK",
                        &body,
                        &[("Content-Type", "application/cbor")],
                    )
                    .expect("write commit response");
                }
                _ => {
                    write_http_response(&mut stream, 404, "Not Found", &[], &[]).expect("write 404");
                }
            }
        }
    });

    let addr = rx.recv().expect("recv addr");
    let wire = build_authcrypt_signed(
        &demo.alice,
        &demo.bob.did,
        MessageMeta {
            v: 1,
            id: make_message_id(now_ms(), 20),
            typ: TYPE_MESSAGE,
            ts_ms: now_ms(),
            ttl_ms: 60_000,
            from: String::new(),
            to: Recipients::One(demo.bob.did.clone()),
            reply_to: None,
            thread_id: None,
        },
        &TextMessageBody {
            msg: "relay-forward-e2e".to_string(),
        },
        &resolver,
    )
    .expect("build wire");

    let forward = RelayForward {
        fwd_v: amp001_example::TRANSPORT_WRAPPER_VERSION_V1,
        message: ByteBuf::from(wire),
        from_did: demo.alice.did.clone(),
        recipient_did: demo.bob.did.clone(),
        relay_path: vec![],
        hop_limit: 8,
        upstream_relay: "did:web:example.com:relay:a".to_string(),
        transfer_mode: TransferMode::Dual,
    };
    let forward_body = serde_cbor::to_vec(&forward).expect("encode forward");
    let forward_resp = send_http_request(
        &addr,
        "POST",
        "/amp/v1/relay/forward",
        &[
            ("Content-Type", "application/cbor".to_string()),
            ("X-Principal-Did", "did:web:example.com:relay:a".to_string()),
        ],
        &forward_body,
    )
    .expect("forward request");
    assert_eq!(forward_resp.status, 200);
    let parsed_forward: ForwardResponse =
        serde_cbor::from_slice(&forward_resp.body).expect("decode forward response");
    assert!(parsed_forward.accepted);
    assert!(parsed_forward.receipt.is_some());

    let commit_receipt = serde_cbor::to_vec(&serde_cbor::value::Value::Map(
        vec![(
            serde_cbor::value::Value::Text("downstream_relay".to_string()),
            serde_cbor::value::Value::Text("did:web:example.com:relay:b".to_string()),
        )]
        .into_iter()
        .collect(),
    ))
    .expect("encode commit receipt");
    let commit_report = RelayCommitReport {
        commit_v: amp001_example::TRANSPORT_WRAPPER_VERSION_V1,
        commit_receipt: ByteBuf::from(commit_receipt),
    };
    let commit_body = serde_cbor::to_vec(&commit_report).expect("encode commit report");
    let commit_resp = send_http_request(
        &addr,
        "POST",
        "/amp/v1/relay/commit",
        &[
            ("Content-Type", "application/cbor".to_string()),
            ("X-Principal-Did", "did:web:example.com:relay:b".to_string()),
        ],
        &commit_body,
    )
    .expect("commit request");
    assert_eq!(commit_resp.status, 200);
    let parsed_commit: CommitResponse =
        serde_cbor::from_slice(&commit_resp.body).expect("decode commit response");
    assert!(parsed_commit.accepted);

    server.join().expect("server thread");
}

fn extract_map_text(value: &serde_cbor::Value, key: &str) -> Option<String> {
    match value {
        serde_cbor::Value::Map(map) => map.iter().find_map(|(k, v)| {
            if matches!(k, serde_cbor::Value::Text(t) if t == key) {
                if let serde_cbor::Value::Text(s) = v {
                    return Some(s.clone());
                }
            }
            None
        }),
        _ => None,
    }
}
