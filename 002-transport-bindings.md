# RFC 002: AMP Transport Bindings

**Status**: Draft  
**Authors**: Ryan Cooper  
**Created**: 2026-02-05  
**Updated**: 2026-02-05  
**Version**: 0.3

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Enables:**
- RFC 003: Relay & Store-and-Forward

---

## Abstract

This specification defines how AMP messages (RFC 001) are transmitted over common transport protocols. It covers WebSocket, TCP, and HTTP bindings, including connection establishment, message framing, authentication, and error handling at the transport layer.

AMP is transport-agnostic by design. This document provides concrete bindings to enable interoperable implementations.

---

## 1. Introduction

### 1.1 Scope

This RFC defines:
- **WebSocket Binding**: Persistent bidirectional connection for real-time agent communication
- **HTTP Binding**: Request-response pattern for simple interactions and webhooks
- **TCP Binding**: Low-overhead persistent connection for high-performance scenarios

### 1.2 Non-Goals

- Relay discovery and federation (→ RFC 003)
- Message persistence semantics (→ RFC 003)
- QUIC binding (future RFC)
- UDP binding (unreliable transport not suitable for AMP)

### 1.3 Terminology

| Term | Definition |
|------|------------|
| **Endpoint** | A network address that accepts AMP messages |
| **Client** | The party initiating a connection |
| **Server** | The party accepting connections (agent or relay) |
| **Frame** | A transport-layer unit containing one AMP message |
| **Binding** | A specification mapping AMP semantics to a transport protocol |

### 1.4 Requirements Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

---

## 2. Common Concepts

### 2.1 Transport vs Application Layer

AMP distinguishes two confirmation levels (RFC 001 §R6):

| Level | Meaning | Who Sends |
|-------|---------|-----------|
| **Transport ACK** | Message received by next hop (relay or recipient endpoint) | Transport layer |
| **Application ACK** | Message processed by recipient agent | Application layer (AMP `ACK` message) |

Transport bindings define **transport ACK** behavior. Application ACK is an AMP message type defined in RFC 001.

### 2.2 Message Framing

AMP messages are CBOR-encoded binary. Transport bindings MUST:
1. Preserve message boundaries (one frame = one AMP message)
2. Support messages of at least **1 MiB** (mandatory minimum)
3. Handle fragmentation transparently if needed

### 2.2.1 Message Size Negotiation

Not all implementations support the same maximum message size:

| Role | Mandatory Minimum | Typical | Notes |
|------|-------------------|---------|-------|
| Agent | 1 MiB | 1-16 MiB | Constrained by memory |
| Relay | 1 MiB | 16-64 MiB | Must handle diverse agents |

**Negotiation mechanism:**

1. **During handshake**: Both parties declare `max_msg_size` they can receive
2. **Effective limit**: `min(client_max, server_max)`
3. **Pre-connection hint** (optional): DID Document service metadata

```json
{
  "id": "did:web:example.com:agent:xxx#amp",
  "type": "AgentMessaging",
  "serviceEndpoint": "wss://agent.example.com/amp/v1/ws",
  "maxMessageSize": 16777216
}
```

**Behavior when limit exceeded:**
- Sender SHOULD check limit before sending
- Receiver MUST reject with appropriate error (WebSocket 1009, TCP ERROR frame, HTTP 413)
- Receiver SHOULD NOT crash or hang on oversized messages

### 2.3 Transport-Layer Authentication

Before exchanging AMP messages, parties MAY authenticate at the transport layer:

| Method | Use Case |
|--------|----------|
| **TLS Client Cert** | Mutual TLS with DID-bound certificate |
| **Bearer Token** | Short-lived tokens for relay access |
| **DID Auth Challenge** | Prove DID ownership via signed challenge |

Transport auth is OPTIONAL. AMP messages are self-authenticating via signatures (RFC 001 §8).

### 2.4 Connection Lifecycle

```
┌─────────────────────────────────────────────────────────┐
│                    Connection States                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────┐    success    ┌─────────────┐            │
│  │ CONNECTING├──────────────►│ AUTHENTICATED│           │
│  └─────┬────┘               └──────┬──────┘            │
│        │                           │                    │
│        │ failure                   │ ready              │
│        ▼                           ▼                    │
│  ┌──────────┐               ┌─────────────┐            │
│  │  FAILED  │               │    OPEN     │◄───┐       │
│  └──────────┘               └──────┬──────┘    │       │
│                                    │           │       │
│                              error │    reconnect      │
│                                    ▼           │       │
│                             ┌─────────────┐    │       │
│                             │   CLOSED    ├────┘       │
│                             └─────────────┘            │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 3. WebSocket Binding

### 3.1 Overview

WebSocket provides persistent, bidirectional communication ideal for:
- Real-time agent-to-agent messaging
- Agent-to-relay connections
- Long-running sessions

### 3.2 Connection Establishment

#### 3.2.1 Endpoint URL

WebSocket endpoints SHOULD follow this pattern:

```
wss://{host}:{port}/amp/v1/ws
```

- `wss://` REQUIRED for production (TLS)
- `/amp/v1/` indicates AMP protocol version 1.x
- `/ws` indicates WebSocket transport

#### 3.2.2 Subprotocol Negotiation

Clients MUST request the `amp.v1` subprotocol:

```http
GET /amp/v1/ws HTTP/1.1
Host: relay.example.com
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Protocol: amp.v1
Sec-WebSocket-Version: 13
X-AMP-Max-Message-Size: 16777216
```

Servers MUST respond with:

```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: ...
Sec-WebSocket-Protocol: amp.v1
X-AMP-Max-Message-Size: 67108864
```

The `X-AMP-Max-Message-Size` header declares the maximum message size (in bytes) each party can receive. If omitted, assume the mandatory minimum (1 MiB).

If `amp.v1` is not supported, server MUST reject with HTTP 400.

#### 3.2.3 Authentication During Handshake

Optional transport-layer auth via headers:

```http
GET /amp/v1/ws HTTP/1.1
Authorization: Bearer <token>
X-AMP-DID: did:web:agentries.xyz:agent:xxx
```

Or via query parameters (when headers unavailable):

```
wss://relay.example.com/amp/v1/ws?token=<token>&did=<did>
```

### 3.3 Message Framing

#### 3.3.1 Frame Type

AMP messages MUST be sent as **Binary frames** (opcode 0x02).

Text frames (opcode 0x01) MUST NOT be used for AMP messages.

#### 3.3.2 One Message Per Frame

Each WebSocket binary frame MUST contain exactly one complete AMP message (CBOR-encoded).

Implementations MUST NOT:
- Split one AMP message across multiple frames
- Combine multiple AMP messages in one frame

WebSocket fragmentation (continuation frames) MAY be used for large messages, but MUST reassemble to exactly one AMP message.

#### 3.3.3 Maximum Message Size

| Party | Minimum Support | Recommended |
|-------|-----------------|-------------|
| Agents | 1 MiB | 16 MiB |
| Relays | 16 MiB | 64 MiB |

Messages exceeding the recipient's limit MUST be rejected with WebSocket close code 1009 (Message Too Big).

### 3.4 Keepalive / Heartbeat

#### 3.4.1 WebSocket Ping/Pong

Both parties SHOULD send WebSocket Ping frames to detect connection health:

| Parameter | Recommendation |
|-----------|---------------|
| Ping interval | 30 seconds |
| Pong timeout | 10 seconds |

If Pong is not received within timeout, the connection SHOULD be considered dead.

#### 3.4.2 AMP-Level Ping (Optional)

For application-level health checks, agents MAY use AMP `PING` message (type 0x10):

```cddl
ping = {
  typ: 0x10,
  from: did,
  ts: uint,
  ttl: uint,
  ? echo: bytes  ; optional payload to echo back
}
```

Response is `PONG` (type 0x11):

```cddl
pong = {
  typ: 0x11,
  from: did,
  ts: uint,
  ttl: uint,
  reply_to: msg_id,
  ? echo: bytes  ; echoed payload
}
```

### 3.5 Transport ACK

#### 3.5.1 Implicit ACK

For WebSocket, transport-layer receipt is **implicit**:
- If the frame is accepted without WebSocket error → transport ACK
- No separate transport ACK message needed

#### 3.5.2 Relay Receipt Confirmation

Relays MAY send explicit receipt confirmation using AMP `ACK` message with `ack_source: "relay"` (RFC 001 §16).

### 3.6 Error Handling

#### 3.6.1 WebSocket Close Codes

| Code | Meaning | When to Use |
|------|---------|-------------|
| 1000 | Normal | Clean shutdown |
| 1002 | Protocol Error | Invalid frame type, malformed message |
| 1003 | Unsupported Data | Non-CBOR payload |
| 1008 | Policy Violation | Auth failure, rate limit |
| 1009 | Message Too Big | Exceeds size limit |
| 1011 | Internal Error | Server error |
| 4000-4999 | AMP-specific | Reserved for AMP errors |

#### 3.6.2 AMP-Specific Close Codes

| Code | Meaning |
|------|---------|
| 4001 | Invalid AMP message (CBOR decode failed) |
| 4002 | Signature verification failed |
| 4003 | DID resolution failed |
| 4004 | Message expired (ts + ttl) |
| 4005 | Unsupported AMP version |

### 3.7 Reconnection

#### 3.7.1 Backoff Strategy

On unexpected disconnect, clients SHOULD reconnect with exponential backoff:

```
delay = min(initial * 2^attempt, max_delay) + random_jitter

initial = 1 second
max_delay = 60 seconds
jitter = 0-1 second (random)
```

#### 3.7.2 Message Recovery

Unacknowledged messages SHOULD be retransmitted after reconnection:
- Use AMP message `id` for idempotency (RFC 001 §16)
- Recipients MUST handle duplicate messages gracefully

---

## 4. TCP Binding

### 4.1 Overview

TCP binding provides the lowest overhead for high-performance scenarios:
- Relay-to-relay communication
- High-frequency agent messaging
- Environments requiring maximum throughput

Unlike WebSocket, TCP binding requires custom framing but avoids HTTP overhead.

### 4.2 Connection Establishment

#### 4.2.1 Endpoint

TCP endpoints use a dedicated port:

```
amp://{host}:{port}
amps://{host}:{port}  (with TLS)
```

Default port: **5710** (AMP = 0x414D50 → 5710 in decimal... or just a memorable number)

#### 4.2.2 TLS Requirement

Production deployments MUST use TLS 1.2+. The `amps://` scheme indicates TLS.

Plain `amp://` MAY be used for:
- Local development
- Already-encrypted tunnels (VPN, WireGuard)

#### 4.2.3 Handshake

After TCP/TLS connection, client sends a handshake frame:

```cddl
handshake_request = {
  magic: 0x414D5031,    ; "AMP1" in hex
  version: uint,        ; Protocol version (1)
  max_msg_size: uint,   ; Max message size client can receive (bytes)
  ? did: text,          ; Client DID (optional)
  ? token: bytes,       ; Auth token (optional)
  ? extensions: [text]  ; Requested extensions
}
```

Server responds:

```cddl
handshake_response = {
  magic: 0x414D5031,
  version: uint,
  accepted: bool,
  max_msg_size: uint,   ; Max message size server can receive (bytes)
  ? error: text,
  ? extensions: [text]  ; Accepted extensions
}
```

If `accepted` is false, server closes the connection after sending response.

### 4.3 Message Framing

TCP is a stream protocol, so we need explicit framing:

#### 4.3.1 Frame Format

```
┌────────────────┬────────────────┬─────────────────────┐
│  Length (4B)   │  Type (1B)     │  Payload (N bytes)  │
│  big-endian    │                │                     │
└────────────────┴────────────────┴─────────────────────┘
```

- **Length**: 4 bytes, big-endian, includes type byte (so payload = length - 1)
- **Type**: 1 byte frame type
- **Payload**: CBOR-encoded data

#### 4.3.2 Frame Types

| Type | Name | Payload |
|------|------|---------|
| 0x01 | MESSAGE | AMP message (CBOR) |
| 0x02 | HANDSHAKE | Handshake request/response |
| 0x03 | PING | Optional echo data |
| 0x04 | PONG | Echoed data |
| 0x05 | GOAWAY | Graceful shutdown notice |
| 0x06 | ERROR | Error details |

#### 4.3.3 Maximum Frame Size

| Party | Minimum Support | Recommended |
|-------|-----------------|-------------|
| Agents | 1 MiB | 16 MiB |
| Relays | 16 MiB | 64 MiB |

Frames exceeding limit MUST trigger connection close with ERROR frame.

### 4.4 Keepalive

#### 4.4.1 PING/PONG Frames

Either party MAY send PING frames:

| Parameter | Recommendation |
|-----------|---------------|
| Ping interval | 30 seconds |
| Pong timeout | 10 seconds |

PONG MUST echo the PING payload exactly.

#### 4.4.2 Idle Timeout

Connections with no activity for 5 minutes SHOULD be closed.

### 4.5 Graceful Shutdown

#### 4.5.1 GOAWAY Frame

Before closing, sender SHOULD send GOAWAY:

```cddl
goaway = {
  reason: uint,       ; Reason code
  ? message: text,    ; Human-readable message
  ? last_id: bytes    ; Last processed message ID
}
```

Reason codes:
| Code | Meaning |
|------|---------|
| 0 | Normal shutdown |
| 1 | Protocol error |
| 2 | Internal error |
| 3 | Overloaded |
| 4 | Maintenance |

#### 4.5.2 Shutdown Sequence

1. Sender sends GOAWAY
2. Sender stops sending new messages
3. Sender waits for in-flight responses (with timeout)
4. Sender closes TCP connection

### 4.6 Error Handling

ERROR frame for non-fatal errors:

```cddl
error_frame = {
  code: uint,         ; Error code
  message: text,      ; Description
  ? msg_id: bytes     ; Related message ID
}
```

Fatal errors → GOAWAY + close.

### 4.7 Multiplexing (Optional Extension)

For high-throughput scenarios, the `multiplex` extension allows multiple logical streams:

```
┌────────────────┬────────────────┬────────────────┬─────────────────────┐
│  Length (4B)   │  Type (1B)     │  Stream (2B)   │  Payload (N bytes)  │
└────────────────┴────────────────┴────────────────┴─────────────────────┘
```

Stream 0 = control stream (handshake, ping/pong, goaway).
Streams 1-65535 = message streams.

This is OPTIONAL and requires negotiation during handshake.

---

## 5. HTTP Binding

### 4.1 Overview

HTTP binding supports:
- Simple request-response interactions
- Webhook-style push notifications
- Environments where WebSocket is unavailable

### 4.2 Endpoint URL

HTTP endpoints SHOULD follow this pattern:

```
https://{host}:{port}/amp/v1/messages
```

### 4.3 Sending Messages (POST)

#### 4.3.1 Request

```http
POST /amp/v1/messages HTTP/1.1
Host: agent.example.com
Content-Type: application/cbor
Content-Length: <length>
X-AMP-Message-ID: <msg_id>

<CBOR-encoded AMP message>
```

#### 4.3.2 Response

**Success (message accepted):**

```http
HTTP/1.1 202 Accepted
Content-Type: application/cbor
X-AMP-Message-ID: <msg_id>

<optional: CBOR-encoded AMP ACK or response>
```

**Synchronous response available:**

```http
HTTP/1.1 200 OK
Content-Type: application/cbor

<CBOR-encoded AMP response message>
```

#### 4.3.3 Status Code Mapping

| HTTP Status | Meaning | AMP Error Code |
|-------------|---------|----------------|
| 200 | Sync response included | - |
| 202 | Accepted for processing | - |
| 400 | Malformed message | 1001 |
| 401 | Auth required | 3001 |
| 403 | Forbidden | 3002 |
| 404 | Unknown recipient | 2001 |
| 413 | Message too large | 1003 |
| 429 | Rate limited | 2003 |
| 500 | Server error | 4001 |
| 503 | Temporarily unavailable | 4002 |

### 4.4 Polling for Messages (GET)

Agents without persistent connections MAY poll for pending messages:

```http
GET /amp/v1/messages?since=<timestamp>&limit=50 HTTP/1.1
Host: relay.example.com
Authorization: Bearer <token>
Accept: application/cbor
```

Response:

```http
HTTP/1.1 200 OK
Content-Type: application/cbor

{
  "messages": [<array of AMP messages>],
  "next_cursor": "<cursor for pagination>",
  "has_more": true
}
```

### 4.5 Webhook Push

#### 4.5.1 Registration

Agents register webhook endpoints via DID Document service:

```json
{
  "id": "did:web:example.com:agent:xxx#amp-webhook",
  "type": "AgentMessagingWebhook",
  "serviceEndpoint": "https://agent.example.com/amp/v1/webhook"
}
```

#### 4.5.2 Delivery

Relays POST messages to registered webhooks:

```http
POST /amp/v1/webhook HTTP/1.1
Host: agent.example.com
Content-Type: application/cbor
X-AMP-Relay: did:web:relay.example.com
X-AMP-Signature: <relay signature over body>
X-AMP-Timestamp: <unix timestamp>

<CBOR-encoded AMP message>
```

#### 4.5.3 Webhook Verification

Recipients MUST verify:
1. `X-AMP-Timestamp` is recent (within 5 minutes)
2. `X-AMP-Signature` is valid for the relay's DID
3. AMP message signature is valid (per RFC 001)

### 4.6 Long Polling (Optional)

For near-real-time without WebSocket:

```http
GET /amp/v1/messages/stream?timeout=30 HTTP/1.1
Host: relay.example.com
Authorization: Bearer <token>
```

Server holds connection open until:
- Message available → return immediately
- Timeout reached → return empty response
- Connection closed → client reconnects

---

## 6. Security Considerations

### 5.1 Transport Security

All AMP transport bindings MUST use TLS 1.2 or higher in production.

Plain HTTP/WS (`http://`, `ws://`) MAY be used only for local development.

### 5.2 Authentication Layers

| Layer | Mechanism | Purpose |
|-------|-----------|---------|
| Transport | TLS, Bearer tokens | Connection authorization |
| Message | AMP signatures | Message authenticity |

Both layers provide defense in depth:
- Transport auth enables rate limiting, access control
- Message auth provides end-to-end verification

### 5.3 Replay Protection

Transport layer SHOULD implement:
- Nonce/timestamp checking for webhook signatures
- Connection-level sequence numbers (optional)

AMP-level replay protection (RFC 001 §8.4) remains the primary defense.

### 5.4 DoS Mitigation

Implementations SHOULD:
- Enforce message size limits
- Implement rate limiting per DID
- Timeout slow connections
- Reject malformed messages early

---

## 7. Implementation Considerations

### 7.1 Choosing a Binding

| Use Case | Recommended Binding |
|----------|---------------------|
| Real-time agent chat | WebSocket |
| High-frequency messaging | WebSocket or TCP |
| Relay-to-relay backbone | TCP |
| Maximum throughput | TCP |
| Simple RPC-style calls | HTTP POST |
| Serverless environments | HTTP + Webhook |
| Mobile/constrained devices | HTTP (battery-friendly) |
| Firewall-restricted | WebSocket (port 443) |

### 7.2 Hybrid Approach

Agents MAY support multiple bindings simultaneously:
- WebSocket for active connections
- TCP for high-throughput relay links
- HTTP webhook for offline delivery
- HTTP polling as fallback

### 7.3 Testing Interoperability

Implementations SHOULD test:
- [ ] WebSocket handshake with `amp.v1` subprotocol
- [ ] Binary frame encoding/decoding
- [ ] Large message handling (>1 MiB)
- [ ] Reconnection with message recovery
- [ ] HTTP POST with CBOR body
- [ ] Webhook signature verification
- [ ] TCP handshake and framing
- [ ] TCP graceful shutdown (GOAWAY)

---

## 8. IANA Considerations

### 7.1 WebSocket Subprotocol

This document registers the `amp.v1` WebSocket subprotocol:

| Field | Value |
|-------|-------|
| Subprotocol Identifier | `amp.v1` |
| Reference | This document |

### 7.2 Media Type

AMP uses existing `application/cbor` media type (RFC 8949).

---

## 9. References

### 8.1 Normative References

- [RFC 001] Agent Messaging Protocol (Core)
- [RFC 2119] Key words for RFCs
- [RFC 6455] The WebSocket Protocol
- [RFC 8949] CBOR
- [RFC 8446] TLS 1.3

### 8.2 Informative References

- [RFC 7231] HTTP/1.1 Semantics
- [RFC 9110] HTTP Semantics
- [DIDComm Transports] https://identity.foundation/didcomm-messaging/spec/#transports

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-05 | 0.1 | Ryan Cooper | Initial draft |
| 2026-02-05 | 0.2 | Ryan Cooper | Added TCP binding (Section 4): length-prefixed framing, handshake, GOAWAY, optional multiplexing |
| 2026-02-05 | 0.3 | Ryan Cooper | Added message size negotiation (§2.2.1): mandatory 1 MiB minimum, handshake declares max_msg_size, DID Document hint |

---

## Open Questions

1. **AMP-level Ping/Pong**: Should PING (0x10) and PONG (0x11) be added to RFC 001 message types, or kept transport-specific?

2. **Batch HTTP**: Should we support sending multiple AMP messages in one HTTP request (array of CBOR)?

3. **Server-Sent Events**: Is SSE a useful alternative to WebSocket for read-heavy scenarios?

4. **HTTP/2 Streams**: Should we define HTTP/2 stream semantics for multiplexed AMP channels?

5. **mTLS DID Binding**: How exactly should DID be bound to TLS client certificates?
