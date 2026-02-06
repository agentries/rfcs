# RFC 002: Transport Bindings (TCP-first, HTTP/WS mappings)

**Status**: Draft
**Authors**: Ryan Cooper, Nowa
**Created**: 2026-02-05
**Updated**: 2026-02-06
**Version**: 0.7

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 003: Relay and Store-and-Forward (persistence and queue policy)

---

## Abstract

This RFC defines AMP transport bindings with a TCP-first normative model. AMPS (TCP over TLS) is the canonical binding used to define framing, handshake, ordering, and transport error behavior. WebSocket and HTTP bindings are specified as normative mappings to the same canonical semantics. The goal is high-performance interoperability without transport-specific semantic drift.

---

## Table of Contents

1. Scope and Non-Goals
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Canonical Transport Semantics (TCP-first)
3.1 Connection and Handshake
3.2 Frame Model and Limits
3.3 AMP Version Negotiation Sequencing
3.4 ACK Boundary and Delivery Semantics
3.5 Endpoint Selection and Binding Priority
4. AMPS/TCP Binding (Canonical)
5. WebSocket Mapping to Canonical Semantics
6. HTTP Mapping to Canonical Semantics
6.1 Submit Endpoint
6.2 Polling Wrapper (Normative)
6.3 Webhook Wrapper (Normative)
6.4 HTTP Status Mapping
7. Transport Authentication and DID Binding
8. Error Handling and Retry
9. Versioning and Compatibility
10. Security Considerations
11. Implementation Checklist
12. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions

---

## 1. Scope and Non-Goals

### 1.1 Scope

This RFC defines:
- How a full CBOR `amp-message` (RFC 001) is carried over TCP, WebSocket, and HTTP.
- Transport handshake, frame boundaries, size negotiation, and keepalive behavior.
- Binding-specific error/status mapping to RFC 001 error categories.
- Conformance requirements for agents and relays.

### 1.2 Non-Goals

This RFC does not define:
- Relay federation topology, queue retention, or store-and-forward guarantees (RFC 003).
- Capability/session/discovery/presence semantics (RFC 004/006/008).
- New AMP message types (all message types remain in RFC 001).
- QUIC/UDP binding details (future RFC).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

A transport implementation is conformant only if it:
- Satisfies Section 3 common canonical semantics.
- Satisfies all requirements for each claimed binding section.
- Preserves RFC 001 message bytes and validation model at protocol boundaries.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Canonical binding | AMPS/TCP binding used as semantic reference model. |
| Transport ACK | Confirmation that the next hop accepted transport payload. |
| Application ACK | AMP `ACK`/`PROC_*` semantics in RFC 001 Section 16. |
| Transport principal | Identity established by transport auth (token subject, mTLS identity). |
| Effective max size | `min(sender_max, receiver_max)` bytes. |

### 2.2 Role Profiles and MTI Requirements

`Core Agent Profile`:
- MUST support HTTP `POST /amp/v1/messages` as sender MTI.
- MUST support at least one receive mode: HTTP polling client or WebSocket client.
- SHOULD support AMPS client mode when high-throughput/low-latency is required.

`Relay Profile`:
- MUST support AMPS server endpoint.
- MUST support HTTP `POST /amp/v1/messages` endpoint.
- MUST support HTTP polling endpoint with the normative wrapper in Section 6.2.
- SHOULD support WebSocket endpoint.
- SHOULD support webhook push with the normative wrapper in Section 6.3.

Rationale: HTTP MTI guarantees minimum cross-vendor interoperability; AMPS canonical semantics guarantee performance and precise transport behavior.

---

## 3. Canonical Transport Semantics (TCP-first)

### 3.1 Connection and Handshake

Canonical state sequence:

```
IDLE -> CONNECTED -> HANDSHAKE -> OPEN -> DRAINING -> CLOSED
```

Requirements:
- No AMP payload frame may be sent before handshake completion.
- Handshake timeout is implementation-defined; recommended default is 10 seconds.
- On handshake failure, endpoint MUST close cleanly.

### 3.2 Frame Model and Limits

Transport MUST preserve AMP boundaries:
- One transport message unit carries exactly one AMP payload.
- Sender MUST NOT coalesce multiple AMP payloads in one transport unit.
- Receiver MUST reject partial/truncated payloads.

All conformant implementations MUST accept at least 1 MiB inbound payload.

Recommended limits:
- Agent endpoint: 16 MiB
- Relay endpoint: 64 MiB

### 3.3 AMP Version Negotiation Sequencing

RFC 001 `HELLO`/`HELLO_ACK`/`HELLO_REJECT` (Section 13) governs AMP version negotiation.

Sequencing rules:
- For persistent channels (AMPS/WS), after transport handshake completes, non-handshake AMP messages MUST NOT be sent until AMP version is negotiated.
- For request/response HTTP mode, sender MAY skip explicit HELLO exchange only when using a preconfigured supported AMP major version; recipient MUST still reject unsupported `v` with `1004`.

### 3.4 ACK Boundary and Delivery Semantics

Transport success is not application success.

- Transport success indicates next-hop acceptance only.
- Application confirmation requires RFC 001 `ACK`/`PROC_OK`/`PROC_FAIL`.
- Relay-emitted `ACK` MUST follow RFC 001 `ack_source` and signature validation rules.

### 3.5 Endpoint Selection and Binding Priority

Endpoint discovery and service typing are defined in RFC 008 (`AgentMessaging`, `AgentMessagingRelay`, `AgentMessagingGated`).

When multiple routable endpoints are available, sender/relay MUST apply this binding priority:

```
amps > wss > https
```

Selection rules:
- Only endpoints supported by local implementation and policy are eligible.
- Within the same binding class, preserve DID Document service order.
- `AgentMessagingGated` endpoints MUST follow RFC 008 contact policy before message submission.
- If no eligible endpoint exists, sender/relay SHOULD map failure to `2001` (recipient not found/unsupported) or `2002` (endpoint unreachable) based on local evidence.

---

## 4. AMPS/TCP Binding (Canonical)

### 4.1 Endpoint and TLS

Endpoint URI:

```
amps://{host}:{port}
```

Plain `amp://` MAY be used only in trusted development/private environments.
Production deployments MUST use TLS 1.2+.

### 4.2 Frame Format

```
0                   1                   2                   3
0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-------------------------------+---------------+-----------------
| length (uint32, big-endian)   | frame_type(1) | payload (N-1)   |
+-------------------------------+---------------+-----------------
```

Rules:
- `length` includes `frame_type + payload` bytes.
- `length` MUST be >= 1.
- `length` exceeding effective max MUST be rejected.

### 4.3 Frame Types

| Type | Name | Payload |
|------|------|---------|
| 0x01 | AMP_MESSAGE | Raw CBOR `amp-message` bytes |
| 0x02 | HANDSHAKE | CBOR handshake request/response |
| 0x03 | PING | Opaque bytes |
| 0x04 | PONG | Echoed bytes |
| 0x05 | GOAWAY | CBOR goaway object |
| 0x06 | ERROR | CBOR transport error object |

### 4.4 Handshake Payload (CDDL)

```cddl
handshake-request = {
  "version": uint,
  "max_msg_size": uint,
  ? "did": tstr,
  ? "token": bstr,
  ? "extensions": [* tstr]
}

handshake-response = {
  "version": uint,
  "accepted": bool,
  "max_msg_size": uint,
  ? "error": tstr,
  ? "extensions": [* tstr]
}
```

Rules:
- Client MUST send `HANDSHAKE` first.
- Server MUST reply with `HANDSHAKE` before `AMP_MESSAGE`.
- If `accepted = false`, server SHOULD include `error` and close.

### 4.5 Control Frames (CDDL)

```cddl
goaway = {
  "reason": uint,
  ? "message": tstr,
  ? "last_id": bstr .size 16
}

transport-error = {
  "code": uint,
  "message": tstr,
  ? "msg_id": bstr .size 16
}
```

### 4.6 Graceful Shutdown

1. Send `GOAWAY`.
2. Stop accepting new work on this connection.
3. Drain in-flight work until timeout.
4. Close connection.

---

## 5. WebSocket Mapping to Canonical Semantics

### 5.1 Endpoint and Subprotocol

```
wss://{host}/amp/v1/ws
```

Client MUST request `Sec-WebSocket-Protocol: amp.v1`.
Server MUST select `amp.v1` or reject.

### 5.2 Mapping Rules

- One WebSocket binary message maps to one canonical `AMP_MESSAGE` frame payload.
- Text messages MUST be rejected.
- WebSocket continuation frames MAY be used, but reassembled payload MUST represent exactly one AMP message.

### 5.3 Size and Keepalive

- `X-AMP-Max-Message-Size` headers map to canonical size negotiation.
- WebSocket Ping/Pong maps to canonical keepalive.

### 5.4 Close/Error Mapping

| Condition | WS Code | Canonical / AMP Hint |
|----------|---------|----------------------|
| Normal close | 1000 | clean close |
| Framing violation | 1002 | transport error -> 1001 |
| Non-binary payload | 1003 | transport error -> 1001 |
| Policy/auth failure | 1008 | 3001 / 2003 |
| Oversize payload | 1009 | size violation -> 1001 |
| Internal server error | 1011 | 5001 |

---

## 6. HTTP Mapping to Canonical Semantics

### 6.1 Submit Endpoint

```http
POST /amp/v1/messages HTTP/1.1
Host: relay.example.com
Content-Type: application/cbor
Accept: application/cbor
X-AMP-Transport-Version: 1
Authorization: Bearer <token>

<raw amp-message bytes>
```

Rules:
- Body MUST contain exactly one AMP payload.
- Server MUST validate message envelope-level constraints before acceptance.
- Success SHOULD be `202 Accepted` (async) or `200 OK` (sync response body).

### 6.2 Polling Wrapper (Normative)

Relay polling endpoint:

```http
GET /amp/v1/messages?cursor=<opaque>&limit=50 HTTP/1.1
Accept: application/cbor
Authorization: Bearer <token>
```

Polling response CDDL (fixed wrapper):

```cddl
poll-response = {
  "messages": [* bstr],       ; each bstr is full raw CBOR amp-message bytes
  "next_cursor": tstr / null,
  "has_more": bool
}
```

Rules:
- `messages[i]` MUST be raw AMP bytes (no semantic re-encoding).
- `next_cursor = null` indicates no further page.

Minimal polling semantics in RFC 002 (interoperability baseline):
- Cursor progression MUST be monotonic for a given consumer identity.
- Polling MAY redeliver previously seen messages (at-least-once).
- Polling response MUST preserve message byte integrity for each `messages[i]`.
- Servers SHOULD define a finite replay window and document it.

Definitive store-and-forward consumption semantics (commit/ack-on-read/redelivery policy) are specified in RFC 003.

### 6.3 Webhook Wrapper (Normative)

Relay push:

```http
POST /amp/v1/webhook HTTP/1.1
Content-Type: application/cbor
X-AMP-Relay: did:web:relay.example.com
X-AMP-Timestamp: 1707055200
X-AMP-Signature: <relay-signature>

<CBOR webhook-delivery>
```

Webhook payload CDDL (fixed wrapper):

```cddl
webhook-delivery = {
  "message": bstr,            ; raw full CBOR amp-message bytes
  "relay": tstr,
  "sent_at": uint
}
```

Receiver MUST verify:
- `X-AMP-Timestamp` freshness.
- `X-AMP-Signature` over timestamp + body.
- `X-AMP-Relay` header equals `webhook-delivery.relay`.
- `webhook-delivery.message` as valid AMP payload.

### 6.4 HTTP Status Mapping

| HTTP | Meaning | AMP Hint |
|------|---------|----------|
| 200 | Accepted with sync response | none |
| 202 | Accepted for async path | none |
| 400 | Malformed payload/wrapper | 1001 |
| 401 | Missing/invalid auth | 3001 |
| 403 | Authenticated but not allowed | 3001 |
| 404 | Unknown route/recipient endpoint | 2001 or 2002 |
| 413 | Payload too large | 1001 |
| 429 | Rate/policy rejection | 2003 |
| 500 | Internal failure | 5001 |
| 503 | Temporarily unavailable | 2003 |

---

## 7. Transport Authentication and DID Binding

### 7.1 Principal Binding Rule

Transport auth establishes `transport principal`.

Default rule (`strict` mode, MUST be default):
- `transport principal DID` MUST equal AMP `from` DID.

Optional delegated rule (`act_as` mode):
- Principal MAY send with a different `from` DID only if auth material explicitly authorizes that DID (for example, token `act_as` claim).
- `act_as` mode MUST be explicitly enabled by policy and MUST be auditable.

### 7.2 Enforcement Requirements

Relays MUST:
- Support configurable strict/delegated mode, with strict as default.
- Reject unauthorized principal/from combinations.
- Apply quotas and rate limits at least by `transport principal`, and SHOULD additionally track `from` DID.
- Emit auditable tuple: `(principal_id, from_did, message_id)`.

Failure mapping:
- Unauthorized principal/from binding -> `3001` (HTTP 401/403, WS 1008, TCP ERROR then close).

---

## 8. Error Handling and Retry

### 8.1 Canonical Categories

| Category | Typical Cause | AMP Hint |
|----------|---------------|----------|
| Framing/parse | Invalid WS/TCP/HTTP wrapper | 1001 |
| Size violation | Over effective max | 1001 |
| Version mismatch | Unsupported AMP `v` | 1004 |
| Auth failure | Invalid token/mTLS/signature | 3001 |
| Policy rejection | Rate limit / relay policy / TTL=0 offline | 2003 |
| Internal failure | Unexpected server error | 5001 |

### 8.2 TTL=0 Mapping (from RFC 001)

If recipient is offline/unreachable and message `ttl = 0`:
- WebSocket path: reject at relay policy layer (for example, close 1008 with policy reason).
- HTTP path: reject with 429 or 503 (policy dependent) and AMP hint `2003`.
- TCP path: send `ERROR` frame (`code=2003`) then close or continue by policy.

### 8.3 Retry Guidance

- If transport disconnect occurs before next-hop acceptance, sender SHOULD retry.
- If delivery state is uncertain, sender MAY retry same AMP `id`.
- Receivers MUST enforce idempotency by RFC 001 message ID semantics.

Recommended reconnect backoff:
- initial 1 second, multiplier 2, max 60 seconds, jitter 0 to 1 second.

---

## 9. Versioning and Compatibility

Binding version is independent from AMP message version:
- Binding version: transport behavior (`amp.v1`, `X-AMP-Transport-Version`, handshake.version).
- AMP version: RFC 001 message header `v` plus HELLO negotiation.

Implementations MUST validate both dimensions.

---

## 10. Security Considerations

- Production transport MUST use TLS (WSS/HTTPS/AMPS).
- Transport credentials SHOULD be short-lived and revocable.
- Transport auth complements, but does not replace, RFC 001 signature/encryption checks.
- Receivers SHOULD fail fast on malformed length/framing/wrappers.
- Webhook signature verification MUST bind timestamp + payload to mitigate replay.
- Error responses SHOULD avoid creating decrypt-oracle or recipient-existence oracle behavior.

---

## 11. Implementation Checklist

- [ ] Meets role MTI requirements in Section 2.2.
- [ ] Preserves one transport unit = one AMP payload.
- [ ] Supports inbound payload at least 1 MiB.
- [ ] Negotiates/enforces effective max size.
- [ ] Enforces HELLO sequencing rules for persistent channels.
- [ ] Enforces transport principal vs `from` DID policy.
- [ ] Implements fixed polling/webhook wrappers if supported.
- [ ] Maps errors to canonical categories in Section 8.
- [ ] Passes Appendix A vectors for claimed bindings.

---

## 12. References

### 12.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 6455: The WebSocket Protocol
- RFC 8446: TLS 1.3
- RFC 8949: CBOR
- RFC 9110: HTTP Semantics

### 12.2 Informative References

- RFC 7231: HTTP/1.1 Semantics and Content (historical)
- RFC 5246: TLS 1.2 (legacy interoperability)

---

## Appendix A. Minimal Test Vectors

### A.1 TCP Frame Positive

Given payload hex `a1617801` and frame type `0x01`:
- length = `00000005`
- full frame hex: `0000000501a1617801`

Receiver MUST parse one `AMP_MESSAGE` payload `a1617801`.

### A.2 TCP Frame Negative

Frame hex `0000000401a1617801` MUST be rejected (declared length mismatch).

### A.3 WebSocket Handshake

Request MUST include `Sec-WebSocket-Protocol: amp.v1`.
Response MUST select `amp.v1` with status `101`.
If absent, handshake MUST be rejected.

### A.4 HTTP Polling Wrapper

Valid CBOR object with fields:
- `messages`: array of bstr
- `next_cursor`: tstr or null
- `has_more`: bool

Any type mismatch MUST be treated as malformed wrapper (`400` / hint `1001`).

### A.5 Size-Limit Violation

Given effective max 1 MiB and payload 1 MiB + 1 byte:
- WS: close 1009 or policy-equivalent reject.
- HTTP: 413.
- TCP: transport ERROR then close (or policy reject).

### A.6 Strict Principal Binding Negative

Given strict mode enabled:
- transport principal DID = `did:web:example.com:agent:alice`
- AMP `from` DID = `did:web:example.com:agent:bob`

Expected:
- Reject with auth failure mapping (`3001` hint).
- MUST NOT forward payload to next hop.
- SHOULD emit audit tuple with mismatched principal/from.

### A.7 Persistent Channel Pre-HELLO Negative

Given AMPS or WebSocket connection where transport handshake succeeded but AMP HELLO negotiation not completed:
- Sender transmits non-handshake AMP message (e.g., `typ=0x10 MESSAGE`).

Expected:
- Receiver rejects message as protocol violation.
- Connection MAY be closed by policy.
- Error mapping SHOULD indicate unsupported/invalid protocol state (recommended `1004` or binding-local protocol error mapped to `1001`).

---

## Appendix B. Open Questions

1. Should Relay Profile additionally require WebSocket (`MUST`) once interop test coverage matures?
2. Should AMPS define mandatory congestion-control and backpressure signaling in this RFC or a follow-up?
3. Should webhook wrapper include optional batch delivery in this RFC or defer to RFC 003?
