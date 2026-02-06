# RFC 001: Agent Messaging Protocol (AMP)

**Status**: Draft  
**Authors**: Ryan Cooper, Jason Apple Huang  
**Created**: 2026-02-04  
**Updated**: 2026-02-06  
**Version**: 0.30

---

## Abstract

AMP (Agent Messaging Protocol) is a native communication protocol designed for the AI Agent ecosystem.

**Core Positioning**:
- 🎯 **Goal**: Native messaging protocol for AI Agent ecosystem
- ⚡ **Features**: Binary, efficient, agent-to-agent communication, capability invocation, document/credential exchange
- 🔗 **Position**: Standalone protocol (not a DIDComm profile)

## Table of Contents

1. Problem Statement  
2. Requirements  
3. Protocol Layers  
4. Message Format (CBOR)  
5. Capability Invocation  
6. Document Exchange  
7. Credential Exchange  
8. Security Considerations  
9. Agentries Integration (AMP Discovery)  
10. Presence & Status  
11. Provisional Responses  
12. Capability Namespacing & Versioning  
13. Protocol Version Negotiation  
14. Interoperability  
15. Error Codes  
16. Acknowledgment Semantics  
17. Registry Governance  
18. Open Questions  
19. References  
Appendix A. Test Vectors  
Appendix B. Implementation Notes  
Changelog  

---

## 1. Problem Statement

### 1.1 Current State

Existing communication infrastructure was designed for humans:

| Infrastructure | Era | Agent Problems |
|----------------|-----|----------------|
| Email (IMAP/SMTP) | 1986 | Eventual consistency, no native identity, human-centric |
| HTTP APIs | 1990s | Point-to-point, no standard identity, requires pre-arrangement |
| WebSocket | 2011 | Connection-centric, no identity verification standard |
| DIDComm | 2022 | Designed for "humans/institutions", not agent-optimized |

### 1.2 Unique Requirements of Agents

| Human Messages | Agent Messages |
|----------------|----------------|
| Primarily natural language | Primarily structured data |
| Can be ignored/delayed | Must process or return error |
| "Read" status sufficient | Needs "processed success/failure" |
| Low frequency (seconds/minutes) | Potentially high frequency (milliseconds) |
| JSON/text sufficient | Needs binary efficiency |

### 1.3 Problems AMP Solves

- ❌ No standard protocol for agent-to-agent communication
- ❌ No native capability invocation mechanism
- ❌ No standard delegation/authorization passing
- ❌ No efficient binary format
- ❌ No unified document/credential exchange

**Scope Note**: This protocol supports:
- Autonomous agent-to-agent communication
- Human-delegated agent messages
- Agent capability invocation (RPC)
- Document and credential exchange

### 1.4 Normative Language

The key words "**MUST**", "**MUST NOT**", "**REQUIRED**", "**SHALL**", "**SHALL NOT**", "**SHOULD**", "**SHOULD NOT**", "**RECOMMENDED**", "**MAY**", and "**OPTIONAL**" in this document are to be interpreted as described in RFC 2119 and RFC 8174.

### 1.5 Conformance

An implementation conforms to AMP Core if it satisfies all **MUST/REQUIRED** statements in this RFC and, at minimum:
- Implements the message envelope and validation rules in §4 and §8 (deterministic CBOR, Sig_Input, TTL checks, replay protection).
- Implements HELLO/HELLO_ACK/HELLO_REJECT negotiation in §13.
- Implements ERROR and ACK semantics in §15 and §16.
- Rejects unsupported message types with ERROR 1005 (UNKNOWN_TYPE) or a more specific error when applicable.

Support for capabilities, sessions, discovery, and other extensions is optional for AMP Core conformance; if implemented, those features MUST follow their respective RFCs (RFC 004/006/008).

Conformance profiles:
- `AMP Core`: Envelope/security/handshake/error/ack behavior defined in this RFC.
- `AMP Full`: AMP Core plus document/credential/delegation application flows in this RFC and companion RFCs.

### 1.6 Terminology

- **Agent**: An autonomous or human-delegated software entity identified by a DID and capable of sending/receiving AMP messages.
- **Sender**: The agent that creates and signs an AMP message.
- **Recipient**: The intended agent(s) listed in `to`.
- **Relay**: A store-and-forward intermediary that accepts AMP messages for delivery to recipients.
- **Delivery Confirmation**: Acknowledgment that a message has been accepted for delivery (ACK from relay or recipient).
- **Processing Confirmation**: Acknowledgment that a recipient has completed handling a message (PROC_OK/PROC_FAIL).
- **Endpoint**: A network address where an AMP service receives messages.
- **Contactable**: An agent that can receive AMP messages without prior approval.

---

## 2. Requirements

The requirements below describe the full AMP target. For strict `AMP Core` conformance, Section 1.5 takes precedence on minimum required behavior.

### 2.1 Identity
- **R1**: Agents MUST be identified by DID
- **R2**: Identity MUST be platform-independent and self-sovereign

### 2.2 Security
- **R3**: All messages MUST be signed by sender's private key
- **R4**: Message payloads SHOULD support end-to-end encryption
- **R5**: Recipients MUST be able to verify message authenticity

### 2.3 Delivery
- **R6a**: MUST confirm transport-layer delivery (relay received)
- **R6b**: SHOULD support application-layer confirmation (agent processing result)
- **R6c**: Receipts MUST clearly distinguish delivery vs processing
- **R7**: Messages MUST be persisted until confirmed (see §16.5)
- **R8**: Protocol MUST support asynchronous communication

### 2.4 Efficiency
- **R9**: Message format MUST be binary (CBOR)
- **R10**: Support batch message transmission (see §4.5)
- **R11**: Support streaming (large documents)

### 2.5 Capability
- **R12**: Support capability query and declaration
- **R13**: Support capability invocation (RPC semantics)
- **R14**: Support capability version negotiation

### 2.6 Delegation
- **R15**: Support delegation credential passing
- **R16**: Support delegation chain verification
- **R17**: Support delegation revocation

### 2.7 Interoperability
- **R18**: Transport-layer agnostic (HTTP, WebSocket, TCP, UDP...)
- **R19**: Support document exchange (any MIME type)
- **R20**: Support credential exchange (Verifiable Credentials)

---

## 3. Protocol Layers

AMP adopts a three-layer architecture (inspired by MTProto):

```
┌─────────────────────────────────────────┐
│  Layer 3: Application                   │
│  (Capability invocation, document       │
│   exchange, credential exchange)        │
├─────────────────────────────────────────┤
│  Layer 2: Security                      │
│  (Signing, encryption, authentication)  │
├─────────────────────────────────────────┤
│  Layer 1: Transport                     │
│  (HTTP, WebSocket, TCP, Relay...)       │
└─────────────────────────────────────────┘
```

### 3.1 Transport Layer

Transport-agnostic design:

| Transport | Use Case |
|-----------|----------|
| HTTP POST | Simple, firewall-friendly |
| WebSocket | Bidirectional, real-time |
| TCP | High frequency, low latency |
| Message Queue | Decoupled, reliable |
| Relay Network | Offline support, routing |

### 3.2 Security Layer

**Signing (Required)**:
- Algorithm: Ed25519
- Input: Sig_Input structure (see §8.1 for exact definition)
- Verification: Obtain public key via DID Document, reconstruct Sig_Input, verify
- **Note**: For encrypted messages (`enc` present), recipients MUST decrypt first, then verify signature (see §8.6)

**Encryption (Optional)**:
- Algorithm: X25519-XSalsa20-Poly1305 (NaCl box)
- Profile:
  - `authcrypt`: Authenticated encryption using sender static key agreement key

### 3.3 Application Layer

Message type categories:

| Category | Types | Purpose |
|----------|-------|---------|
| **Control** | PING, PONG, ACK, ERROR | Protocol control |
| **Message** | MESSAGE, REQUEST, RESPONSE | General messaging/RPC |
| **Capability** | CAP_QUERY, CAP_DECLARE | Capability negotiation |
| **Document** | DOC_SEND, DOC_REQUEST | Document exchange |
| **Credential** | CRED_ISSUE, CRED_REQUEST, CRED_VERIFY | Credential exchange |
| **Delegation** | DELEG_GRANT, DELEG_REVOKE | Delegation management |

---

## 4. Message Format (CBOR)

### 4.1 Base Message Structure

```cddl
amp-message = amp-plaintext / amp-encrypted

amp-plaintext = {
  common-fields,
  body: any,                  ; Message body (required; use null for no payload)
  ? ext: {* tstr => any},     ; Extension fields (NOT signed, see §8.7)
}

amp-encrypted = {
  common-fields,
  enc: encrypted-payload,     ; Encrypted payload (replaces body)
  ? ext: {* tstr => any},     ; Extension fields (NOT signed, see §8.7)
}

common-fields = (
  v: uint,                    ; Protocol major version (see §13)
  id: message-id,             ; 16-byte message ID (see §4.2)
  typ: uint,                  ; Message type
  ts: uint,                   ; Unix timestamp (milliseconds) - when created
  ttl: uint,                  ; Time-to-live (milliseconds) - REQUIRED (see §8.1, §8.3)

  ; Routing
  from: did,                  ; Sender DID
  to: did / [+ did],          ; Recipient DID(s)
  ? reply_to: bstr,           ; Message ID being replied to
  ? thread_id: bstr,          ; Conversation/thread ID

  ; Security
  sig: bstr                   ; Ed25519 signature (see §8.1 for Sig_Input)
)

message-id = bstr .size 16

encrypted-payload = {
  alg: "X25519-XSalsa20-Poly1305",
  mode: "authcrypt",
  nonce: bstr,                ; Nonce (XSalsa20-Poly1305)
  ciphertext: bstr            ; Encrypted deterministic_cbor(body)
}

did = tstr  ; DID string or DID URL (optional key fragment)
```

**Field Notes**:
- `v` is the protocol **major** version; see §13 for negotiation and mapping.
- `ts` + `ttl` determine message validity window (see §8.3).
- `sig` covers ALL semantically critical fields (see §8.1 Sig_Input):
  - Includes: id, typ, ts, ttl, from, to, reply_to, thread_id.
  - Always signs **plaintext** body (for encrypted messages, decrypt first then verify; see §8.6).
- `body` is REQUIRED for unencrypted messages. If there is no payload, use CBOR `null` (`0xF6`).
- `enc` and `body` are mutually exclusive. Encrypted messages MUST omit `body`; `enc.ciphertext` MUST encrypt `deterministic_cbor(body)` (see §8.6).
- `enc.mode` is fixed to `"authcrypt"` in AMP 001.
- `body` MUST be encoded using **deterministic CBOR** (RFC 8949 §4.2) for signing; for unencrypted messages, verifiers MUST re-encode body deterministically before verification; for encrypted messages, verifiers use decrypted bytes directly (see §8.2).
- `ext` is NOT signed — treat as untrusted (see §8.7 for security implications).

**Note on Examples**: Code examples throughout this document may omit some required fields (e.g., `ttl`, `sig`) for brevity. All required fields listed above MUST be present in actual implementations.

### 4.2 Message ID Design

Inspired by MTProto, message IDs contain time information:

```
Message ID (16 bytes, big-endian):
┌────────────────────┬────────────────────┐
│  Timestamp (8B)    │  Random (8B)       │
│  uint64_be(ts_ms)  │  uint64_be(rand)   │
└────────────────────┴────────────────────┘

Encoding Rules:
- `ts_ms` MUST equal the message `ts` field (milliseconds since Unix epoch).
- `uint64_be` is an unsigned 64-bit big-endian integer.
- `rand` MUST be generated from a cryptographically secure RNG.

Properties:
- Natural time ordering
- Timestamp MUST match message `ts` field within ±1 second (see §8.3)
- Expiration driven by TTL: reject if now > ts + ttl
- Reject messages where ts > now + MAX_CLOCK_SKEW (clock attack protection)
```

### 4.3 Message Type Codes

**Registry**: Message type codes are allocated in ranges. New types MUST be registered. See Section 17 for registry governance.

```
; ═══════════════════════════════════════════════════════════════
; CONTROL (0x00-0x0F) — Protocol control messages
; ═══════════════════════════════════════════════════════════════
PING              = 0x01    ; Keepalive request
PONG              = 0x02    ; Keepalive response
ACK               = 0x03    ; Delivery confirmation (relay received)
PROC_OK           = 0x04    ; Processing success (agent handled)
PROC_FAIL         = 0x05    ; Processing failure (agent error)
CONTACT_REQUEST   = 0x06    ; Request to establish contact (DISCOVERABLE)
CONTACT_RESPONSE  = 0x07    ; Approve/deny contact request
CONTACT_REVOKE    = 0x08    ; Revoke previously granted contact
PROCESSING        = 0x09    ; Long operation in progress
PROGRESS          = 0x0A    ; Progress update (percentage, ETA)
INPUT_REQUIRED    = 0x0B    ; Blocked, need additional input
; 0x0C-0x0E reserved
ERROR             = 0x0F    ; Protocol error

; ═══════════════════════════════════════════════════════════════
; MESSAGE (0x10-0x1F) — General messaging and streaming
; ═══════════════════════════════════════════════════════════════
MESSAGE           = 0x10    ; General message
REQUEST           = 0x11    ; RPC request
RESPONSE          = 0x12    ; RPC response
STREAM_START      = 0x13    ; Begin streaming transfer
STREAM_DATA       = 0x14    ; Stream data chunk
STREAM_END        = 0x15    ; End streaming transfer
BATCH             = 0x16    ; Batch container (multiple messages)
; 0x17-0x1F reserved

; ═══════════════════════════════════════════════════════════════
; CAPABILITY (0x20-0x2F) — Capability discovery and invocation
; ═══════════════════════════════════════════════════════════════
CAP_QUERY         = 0x20    ; Query capabilities
CAP_DECLARE       = 0x21    ; Declare capabilities
CAP_INVOKE        = 0x22    ; Invoke capability
CAP_RESULT        = 0x23    ; Invocation result
; 0x24-0x2F reserved

; ═══════════════════════════════════════════════════════════════
; DOCUMENT (0x30-0x3F) — Document exchange
; ═══════════════════════════════════════════════════════════════
DOC_SEND          = 0x30    ; Send small document (inline)
DOC_REQUEST       = 0x31    ; Request document
; NOTE: Large documents use STREAM_START/DATA/END with doc metadata
; 0x32-0x3F reserved

; ═══════════════════════════════════════════════════════════════
; CREDENTIAL (0x40-0x4F) — Verifiable credential exchange
; ═══════════════════════════════════════════════════════════════
CRED_ISSUE        = 0x40    ; Issue credential
CRED_REQUEST      = 0x41    ; Request credential
CRED_PRESENT      = 0x42    ; Present credential
CRED_VERIFY       = 0x43    ; Verify credential
; 0x44-0x4F reserved

; ═══════════════════════════════════════════════════════════════
; DELEGATION (0x50-0x5F) — Authorization delegation
; ═══════════════════════════════════════════════════════════════
DELEG_GRANT       = 0x50    ; Grant delegation
DELEG_REVOKE      = 0x51    ; Revoke delegation
DELEG_QUERY       = 0x52    ; Query delegation status
; 0x53-0x5F reserved

; ═══════════════════════════════════════════════════════════════
; PRESENCE (0x60-0x6F) — Agent presence and capacity
; ═══════════════════════════════════════════════════════════════
PRESENCE          = 0x60    ; Presence announcement (capacity data)
PRESENCE_QUERY    = 0x61    ; Query agent presence
PRESENCE_SUB      = 0x62    ; Subscribe to presence updates
PRESENCE_UNSUB    = 0x63    ; Unsubscribe from presence
; 0x64-0x6F reserved

; ═══════════════════════════════════════════════════════════════
; HANDSHAKE (0x70-0x7F) — Protocol version negotiation
; ═══════════════════════════════════════════════════════════════
HELLO             = 0x70    ; Initiate with supported versions
HELLO_ACK         = 0x71    ; Confirm selected version
HELLO_REJECT      = 0x72    ; No compatible version found
; 0x73-0x7F reserved

; ═══════════════════════════════════════════════════════════════
; EXTENSION (0xF0-0xFF) — Vendor/experimental extensions
; ═══════════════════════════════════════════════════════════════
EXTENSION         = 0xF0    ; Extension message (see ext field)
; 0xF1-0xFF reserved for future extension types
```

**Type Semantics**:
- Capability message semantics are defined in **RFC 004**.
- Contact, discovery, and presence message semantics are defined in **RFC 008**.
- Provisional response semantics are defined in **RFC 006**.

### 4.4 Message Body Schemas (CDDL)

The following CDDL defines the expected `body` structure for **core** AMP message types. Additional bodies are defined in other RFCs:
- **Capability bodies (CAP_\*)**: RFC 004
- **Contact & discovery bodies**: RFC 008
- **Presence bodies**: RFC 008
- **Provisional responses (PROCESSING/PROGRESS/INPUT_REQUIRED)**: RFC 006

Capability identifiers and namespace governance are defined in **RFC 004** and are REQUIRED for interoperable capability naming.

```cddl
; Common scalars
semver = tstr
mime-type = tstr
utc-timestamp = tstr  ; RFC 3339 UTC timestamp

; Control bodies
ping-body = null
pong-body = null

ack-body = {
  "ack_source": "relay" / "recipient",
  "received_at": uint,
  ? "ack_target": did,
  ? "stream_id": tstr,
  ? "chunks_received": uint,
  ? "verified": bool
}

proc-ok-body = {
  ? "details": any
}

proc-fail-body = {
  ? "error": any
}

error-body = {
  "code": uint,
  "category": tstr,
  "message": tstr,
  ? "details": any,
  ? "retry": bool,
  ? "batch_index": uint
}

; Document bodies
doc-send-body = {
  "content_type": mime-type,
  "filename": tstr,
  "size": uint,
  "hash": bstr,
  "data": bstr
}

doc-request-body = {
  ? "doc_id": tstr,
  ? "hash": bstr,
  ? "accept": mime-type
}

stream-start-body = {
  "stream_id": tstr,
  "content_type": mime-type,
  "filename": tstr,
  "total_size": uint,
  "total_chunks": uint,
  "chunk_size": uint,
  "hash_algo": tstr
}

stream-data-body = {
  "stream_id": tstr,
  "index": uint,
  "data": bstr
}

stream-end-body = {
  "stream_id": tstr,
  "hash": bstr
}

; Credential bodies
cred-issue-body = {
  "format": tstr,
  "credential": any,
  ? "purpose": tstr
}

cred-request-body = {
  "format": tstr,
  ? "request": any,
  ? "purpose": tstr
}

cred-present-body = {
  "format": tstr,
  "credential": any,
  ? "purpose": tstr
}

cred-verify-body = {
  "format": tstr,
  "credential": any
}

; Delegation bodies
deleg-grant-body = {
  "credential": any,
  ? "scope": any,
  ? "expires": utc-timestamp
}

deleg-revoke-body = {
  "delegation_id": tstr
}

deleg-query-body = {
  "delegation_id": tstr
}

; Handshake bodies
hello-body = {
  "versions": [+ semver],
  ? "extensions": [* tstr],
  ? "agent_info": {
    "name": tstr,
    ? "implementation": tstr
  }
}

hello-ack-body = {
  "selected": semver
}

hello-reject-body = {
  ? "reason": tstr
}

; Batch body
batch-body = {
  "items": [+ bstr]  ; CBOR-encoded amp-message bytes
}
```

**Schema Notes**:
- `doc-request-body` MUST include `doc_id` or `hash`.
- `batch_index` is zero-based index into `batch-body.items`.

### 4.5 Batch Messages

**Purpose**: Reduce transport overhead by grouping multiple AMP messages into a single container.

**Rules**:
- The `BATCH` message body is `batch-body`.
- Each item in `items` MUST be a complete CBOR-encoded `amp-message` with its own signature.
- Recipients MUST process each item as if it arrived independently.
- Recipients SHOULD preserve item order unless application semantics allow parallelization.
- Recipients SHOULD send ACK/PROC/ERROR for each inner message (using the inner message `id`).
- A recipient MAY ACK the batch container itself to indicate receipt of the batch.
- If an inner item is malformed and cannot be decoded, recipients MAY return ERROR referencing the batch container with `batch_index`.

**Streaming Clarification**: Large documents use the generic streaming mechanism (STREAM_START/DATA/END) with document metadata in the body. DOC_SEND is for small inline documents only.

---

## 5. Capability Invocation

Capability discovery, invocation semantics, and compatibility rules are defined in **RFC 004**.
This RFC only reserves message types (`CAP_QUERY`, `CAP_DECLARE`, `CAP_INVOKE`, `CAP_RESULT`) and their numeric codes.

See: `004-capability-schema-registry.md`

---

## 6. Document Exchange

### 6.1 Small Documents (Inline)

```cbor
{
  "typ": 0x30,  ; DOC_SEND
  "body": {
    "content_type": "application/pdf",
    "filename": "report.pdf",
    "size": 102400,
    "hash": h'sha256...',
    "data": h'binary...'
  }
}
```

### 6.2 Large Documents (Streamed)

Large documents use the generic streaming mechanism:

```cbor
; STREAM_START — begin transfer with document metadata
{
  "typ": 0x13,  ; STREAM_START
  "body": {
    "stream_id": "doc-abc123",
    "content_type": "application/pdf",
    "filename": "large-report.pdf",
    "total_size": 10485760,
    "total_chunks": 10,
    "chunk_size": 1048576,    ; bytes per chunk (except possibly last)
    "hash_algo": "sha256"
  }
}

; STREAM_DATA — send chunks
{ "typ": 0x14, "body": { "stream_id": "doc-abc123", "index": 0, "data": h'...' } }
{ "typ": 0x14, "body": { "stream_id": "doc-abc123", "index": 1, "data": h'...' } }
...

; STREAM_END — complete transfer with verification hash (sent by SENDER)
{
  "typ": 0x15,  ; STREAM_END
  "body": {
    "stream_id": "doc-abc123",
    "hash": h'sha256...'       ; expected hash for verification
  }
}

; Receiver confirms via ACK with stream completion status
; ACK body: {
;   "ack_source": "recipient",
;   "received_at": 1707055200000,
;   "stream_id": "doc-abc123",
;   "chunks_received": 10,
;   "verified": true
; }
```

**Streaming Specification**:

| Field | Requirement |
|-------|-------------|
| `index` | 0-based, sequential integers (0, 1, 2, ...) |
| `total_chunks` | Declared in STREAM_START, MUST match actual count |
| `chunk_size` | Uniform size except last chunk MAY be smaller |
| Ordering | Chunks SHOULD arrive in order but MAY arrive out-of-order |

**Hash Computation**:

The final `hash` in STREAM_END is computed as:

```
hash = SHA256(chunk[0] || chunk[1] || ... || chunk[n-1])
```

Where:
- `||` denotes byte concatenation
- Chunks are concatenated in index order (0, 1, 2, ...)
- Hash is computed over raw chunk data, not CBOR encoding
- NO length prefixes or separators between chunks

**Reassembly Algorithm**:

```
1. Receive STREAM_START, allocate buffer of total_size
2. For each STREAM_DATA:
   a. Validate: 0 <= index < total_chunks
   b. Write data at offset: index * chunk_size
   c. Track received chunk bitmap
3. On STREAM_END:
   a. Verify all chunks received (bitmap complete)
   b. Compute hash over concatenated data
   c. Compare with provided hash
   d. REJECT if mismatch
```

**Error Handling**:
- Missing chunks after timeout → request retransmission or fail
- Hash mismatch → reject entire transfer
- Duplicate chunk → ignore (idempotent)

### 6.3 Streaming State Machine

**Sender**:
```
IDLE
  └─ STREAM_START → SENDING
SENDING
  ├─ STREAM_DATA (0..n-1) → SENDING
  └─ STREAM_END → AWAIT_ACK
AWAIT_ACK
  ├─ ACK(verified=true) → DONE
  └─ ERROR/timeout → DONE
```

**Receiver**:
```
IDLE
  └─ STREAM_START → RECEIVING
RECEIVING
  ├─ STREAM_DATA → RECEIVING
  └─ STREAM_END → VERIFY
VERIFY
  ├─ hash ok → ACK(verified=true) → DONE
  └─ hash fail → ERROR → DONE
```

**Rules**:
- Receiver MUST send a single ACK with `stream_id` after VERIFY.
- Sender MUST treat missing ACK as transfer failure and MAY retry with a new `stream_id`.

**Note**: The streaming mechanism is generic and can be used for any large payload, not just documents.

---

## 7. Credential Exchange

Compatible with W3C Verifiable Credentials:

```cbor
{
  "typ": 0x42,  ; CRED_PRESENT
  "body": {
    "format": "jwt_vc",  ; or "cbor_vc", "json_ld"
    "credential": "eyJ...",
    "purpose": "age_verification"
  }
}
```

---

## 8. Security Considerations

### 8.1 Deterministic CBOR Encoding for Signatures

To ensure signature verification succeeds across implementations, signed fields MUST be encoded using **deterministic CBOR** (RFC 8949 §4.2) before signing.

**Rules**:
1. Map keys MUST be sorted by byte-wise lexicographic order
2. Integers MUST use smallest encoding
3. No duplicate map keys
4. No indefinite-length encoding

**Signature Structure** (COSE Sign1 inspired):

```
; Inner signature covers ALL semantically critical fields
Sig_Input = [
  "AMP-v1",                    ; context string
  h'',                         ; protected header (reserved)
  {                            ; signed headers
    "id": message_id,          ; 16-byte message ID
    "typ": typ,                ; message type
    "ts": ts,                  ; creation timestamp
    "ttl": ttl,                ; time-to-live
    "from": from,              ; sender DID
    "to": to,                  ; recipient DID(s)
    ? "reply_to": reply_to,    ; if present
    ? "thread_id": thread_id   ; if present
  },
  deterministic_cbor(body)     ; canonicalized PLAINTEXT body (always)
]

sig = Ed25519_Sign(sender_private_key, CBOR_Encode(Sig_Input))
```

**What is signed**:
- `id` - message identity
- `typ` - message type (prevents type confusion attacks)
- `ts`, `ttl` - temporal validity (prevents relay manipulation)
- `from`, `to` - routing (prevents re-routing attacks)
- `reply_to`, `thread_id` - conversation context (if present)
- `body` - **plaintext** payload content (see §8.6 for encrypted messages). If there is no payload, `body` MUST be CBOR `null` before signing.

**What is NOT signed**:
- `sig` field itself
- `enc` field (ciphertext; binding via decrypt-then-verify, see §8.6)
- `ext` field (see §8.7 for security implications)
- Relay transport metadata (out of scope; see §8.1 note on relay metadata)

**Relay Metadata (Out of Scope)**:

Relay hop-by-hop metadata (routing hints, timestamps, hop lists) is **out of scope** for the AMP message format. Such metadata:
- Is transport-layer specific (HTTP headers, WebSocket frames, MQ attributes)
- MUST NOT be used for security decisions
- Is stripped before AMP message processing

Relay transport bindings are defined in separate specifications (e.g., AMP-over-HTTP, AMP-over-WebSocket).

### 8.2 Signature Verification

**For Unencrypted Messages** (body field present; use `null` for no payload):

Verification steps:
1. Extract `from` DID from message
2. Resolve DID Document to obtain public key (see §8.9 key selection policy)
3. Encode `body` using deterministic CBOR (RFC 8949 §4.2) → body_bytes
4. Reconstruct Sig_Input using body_bytes
5. Verify `sig` using Ed25519_Verify(public_key, CBOR_Encode(Sig_Input), sig)
6. MUST reject if verification fails

**For Encrypted Messages** (enc field present):

1. Extract `from` DID from message
2. Resolve DID Document to obtain public key (see §8.9 key selection policy)
3. Decrypt `enc.ciphertext` → body_bytes
4. Reconstruct Sig_Input using body_bytes directly (no re-encoding)
5. Verify `sig` using Ed25519_Verify(public_key, CBOR_Encode(Sig_Input), sig)
6. Decode body_bytes as CBOR → plaintext body
7. MUST reject if any step fails (see §8.6 for detailed failure handling)

**Important (Unencrypted Messages Only)**: The sender MUST encode `body` using deterministic CBOR before signing. The verifier MUST re-encode `body` using the same deterministic rules. Non-deterministic CBOR encoding will cause signature verification to fail. (For encrypted messages, raw decrypted bytes are used directly; see above.)

**Failure Handling (Unencrypted)**: Recipients MUST reject and return the appropriate error (see §15.3 for retry semantics):

| Failure | Error Code |
|---------|------------|
| Missing required fields | 1001 INVALID_MESSAGE |
| Signature verification fails | 1002 INVALID_SIGNATURE |
| Body is not valid CBOR | 1001 INVALID_MESSAGE |

### 8.3 Timestamp Validation and Offline Messages

**The Problem**: Strict timestamp rejection breaks offline/store-and-forward scenarios.

**Solution**: TTL-driven expiration with clock skew tolerance.

```cbor
{
  "ts": 1707055200000,    ; When message was created (ms)
  "ttl": 86400000,        ; How long message remains valid (ms, default 24h)
  ...
}
```

**Validation Rules** (unified for all delivery paths):

| Check | Rule | Rationale |
|-------|------|-----------|
| **Expiration** | Reject if `now > ts + ttl` | Message has expired |
| **Future protection** | Reject if `ts > now + MAX_CLOCK_SKEW` | Clock attack protection |
| **Immediate expiry** | If TTL = 0, message requires immediate delivery | Real-time only |

**Note**: `ttl` is a REQUIRED field (see §4.1). Senders SHOULD use 86400000 (24h) as a reasonable default when no specific TTL is needed.

**TTL = 0 Semantics**:
- Relays MUST NOT store TTL=0 messages; attempt immediate forward only
- If recipient is offline or unreachable, relay MUST reject with error 2003 (RELAY_REJECTED) with details indicating "TTL=0 requires immediate delivery"
- Use case: real-time signaling where stale messages are harmful (e.g., PING, time-sensitive coordination)
- Senders using TTL=0 SHOULD expect higher failure rates

**MAX_CLOCK_SKEW**: 30 seconds (configurable per implementation)

**Note**: The previous ">5 minutes" rule is REMOVED. All timing is now TTL-driven.

**Message ID Timestamp Consistency**:
- The timestamp embedded in `message_id` (first 8 bytes) MUST match `ts` within ±1 second
- Recipients MUST reject messages where these timestamps differ by >1 second
- This prevents ID/timestamp manipulation attacks

**Relay Behavior**:
- Relays MUST store messages until `ts + ttl`
- Relays MUST delete expired messages
- Relays MUST NOT extend TTL (TTL is signed, see §8.1)
- Relays MAY reject messages with excessive TTL (e.g., >30 days)
- Relays SHOULD indicate maximum supported TTL in service description

### 8.4 Replay Protection

**Message ID Uniqueness**:
- Message IDs MUST be unique per sender
- Recipients SHOULD maintain a cache of recently seen message IDs
- Cache duration SHOULD match maximum expected TTL

**Duplicate Handling**:
- If message ID was previously processed → return cached response (idempotent)
- If message ID is new → process and cache

### 8.5 Encryption

- Sensitive content SHOULD use `authcrypt`
- Relays SHOULD NOT be able to read encrypted content
- Key agreement uses X25519 (Curve25519 ECDH)
- Symmetric encryption uses XSalsa20-Poly1305

### 8.5.1 Encryption Profile (Normative)

AMP 001 defines a single encryption profile: `authcrypt`.

| Profile | `enc` bytes | Sender binding from encryption | Primary use |
|---------|-------------|--------------------------------|-------------|
| `authcrypt` | `alg`, `mode`, `nonce`, `ciphertext` | Yes (sender static key agreement key) | Default interoperable encrypted messaging |

**`authcrypt` sender steps**:
1. Resolve sender and recipient DID Documents.
2. Select sender static X25519 key agreement key (see §8.9).
3. Select recipient static X25519 key agreement key (see §8.9).
4. Compute shared secret with X25519.
5. Derive symmetric key compatible with NaCl box precomputation.
6. Encrypt deterministic CBOR body with XSalsa20-Poly1305 and transmit without `epk`.

**`authcrypt` recipient steps**:
1. Resolve sender DID Document and obtain sender static key agreement public key.
2. Use recipient local static private key.
3. Compute shared secret and derive the same symmetric key.
4. Decrypt ciphertext, then continue with signature verification (§8.6).

### 8.6 Sign-Then-Encrypt

**Design Choice**: AMP uses **sign-then-encrypt** (StE) — the signature covers the plaintext body, then the body is encrypted.

**Why StE?**
- Signature proves sender created the actual content (not just ciphertext)
- Works with authenticated key agreement and signature verification
- Simpler: no circular dependency between signature and ciphertext

**Order of Operations (Sender)**:

```
┌─────────────────────────────────────────────────────────────┐
│  1. Construct plaintext body (any CBOR value)               │
│  2. Encode body as deterministic CBOR → body_bytes          │
│  3. Compute Sig_Input using body_bytes                      │
│  4. Sign → sig                                              │
│  5. Encrypt body_bytes → enc.ciphertext                     │
│  6. Transmit message with: sig + enc (body field absent)    │
└─────────────────────────────────────────────────────────────┘
```

**Critical**: The `enc.ciphertext` MUST encrypt the exact `deterministic_cbor(body)` byte sequence used in Sig_Input. If there is no payload, `body` MUST be CBOR `null` (`0xF6`) before signing and encryption. Encrypting a different serialization will cause signature verification to fail after decryption.

**Encrypted Message Structure**:

```cbor
{
  "v": 1,
  "id": h'...',
  "typ": 0x10,
  "ts": 1707055200000,
  "ttl": 86400000,
  "from": "did:web:...",
  "to": "did:web:...",
  "sig": h'...',              ; signature over PLAINTEXT body
  "enc": {
    "alg": "X25519-XSalsa20-Poly1305",
    "mode": "authcrypt",
    "ciphertext": h'...',
    "nonce": h'...'
  }
  ; NOTE: body field is ABSENT (plaintext is encrypted in enc.ciphertext)
}
```

**Verification (Recipient)**:

```
1. Decrypt enc.ciphertext → recover body_bytes (raw bytes)
2. Reconstruct Sig_Input using body_bytes directly (no re-encoding)
3. Verify sig against Sig_Input
4. Decode body_bytes as CBOR → plaintext body
5. MUST reject if decryption, signature, or CBOR decoding fails
```

**Note**: Recipients use the decrypted bytes directly for signature verification, without re-encoding. This ensures byte-exact match with what the sender signed.

**Failure Handling**: Recipients MUST reject the message and return the appropriate error (see §15.3 for retry semantics):

| Failure | Error Code | Description |
|---------|------------|-------------|
| Decryption fails | 3001 UNAUTHORIZED | Invalid ciphertext, wrong key, corrupted data (see §15.3 privacy note) |
| Sig_Input reconstruction fails | 1001 INVALID_MESSAGE | Missing required fields in message headers |
| Signature verification fails | 1002 INVALID_SIGNATURE | Invalid signature, wrong public key, tampered content |
| CBOR decoding fails | 1001 INVALID_MESSAGE | Decrypted bytes are not valid CBOR |

**Security Properties**:
- **Content authenticity**: signature proves sender created the plaintext content
- **Confidentiality**: only recipient can decrypt
- **Tamper detection**: if attacker modifies ciphertext without knowing plaintext, decryption yields garbage that fails signature verification
- **No downgrade**: encrypted message has `enc` field; unencrypted has `body` field — different structures

**Limitation**: The signature does not directly cover ciphertext bytes. If encryption keys are compromised, an attacker could re-encrypt known plaintext with different parameters and the original signature over plaintext still verifies. This does not compromise content authenticity but changes ciphertext artifacts.

**Important Implications**:

| Implication | Description |
|-------------|-------------|
| **Relay cannot verify signature** | Relays see only ciphertext; they cannot verify sender signature without decryption. Anti-abuse/gating at relay layer must use other mechanisms (e.g., sender reputation, rate limits, outer transport auth). |
| **Verification order** | Recipients MUST decrypt first, then verify signature. When `enc` is present, `body` exists only inside the ciphertext. |
| **Re-encryption after key compromise** | If encryption keys are compromised, known plaintext can be re-encrypted with different parameters while signature still verifies on plaintext. If ciphertext immutability is required, additional binding is needed (out of scope for AMP core). |

**Note**: The signature does NOT directly cover the ciphertext. Binding is achieved through decrypt-then-verify: any tampering with `enc` that changes the plaintext will fail signature verification.

### 8.7 Extension Field Security

**WARNING**: The `ext` field is NOT signed.

**Implications**:
- Relays or intermediaries MAY add/modify `ext` fields
- `ext` MUST be treated as **untrusted input**
- `ext` MUST NOT be used for security decisions or authorization
- `ext` SHOULD only contain non-critical metadata (debug info, routing hints, telemetry)

**Safe Uses**:
- `ext.debug`: Debugging information
- `ext.trace_id`: Distributed tracing correlation
- `ext.relay_hints`: Relay-suggested routing preferences

**Unsafe Uses** (MUST NOT):
- `ext.permissions`: ❌ Authorization data must be in signed body
- `ext.override_ttl`: ❌ TTL is signed, cannot be overridden
- `ext.verified`: ❌ Verification status must be computed, not passed

**Future Extension**: If signed extensions are needed, a future protocol version may introduce a `signed_ext` field included in Sig_Input.

### 8.8 Privacy Considerations

- **Metadata exposure**: `from`, `to`, `typ`, `ts`, and `ttl` are visible to relays and network observers. Implementations SHOULD avoid placing sensitive information in headers.
- **Presence leakage**: Presence and capacity signals can reveal operational patterns. Agents SHOULD allow opt-out or coarse-grained reporting.
- **Error oracles**: Use generic error codes (e.g., 3001 UNAUTHORIZED) for decryption failures to avoid revealing recipient existence or keys.
- **Logging**: Implementations SHOULD minimize retention of plaintext bodies and consider redaction for sensitive payloads.

### 8.9 DID Key Selection Policy

To reduce cross-implementation ambiguity, key selection MUST follow these rules.

**Signature key selection**:
- If `from` is a DID URL with fragment, implementations MUST use that exact verification method.
- If `from` is a bare DID, implementations MUST select from active Ed25519 methods referenced by `assertionMethod` (fallback: `authentication`) and choose the lexicographically smallest method ID.
- Verification MUST fail with 3001 (UNAUTHORIZED) if no eligible signing key exists.

**Key agreement selection (`authcrypt`)**:
- Sender and recipient keys MUST come from active X25519 methods referenced by `keyAgreement`.
- If multiple eligible keys exist, select the lexicographically smallest method ID.
- Receivers SHOULD attempt decryption with all active local key agreement private keys to support rotation.

---

## 9. Agentries Integration (AMP Discovery)

Discovery, contactability, and directory semantics are defined in **RFC 008**.
This RFC reserves CONTACT and PRESENCE message types but does not specify discovery workflows.

See: `008-agent-discovery-directory.md`

---

## 10. Presence & Status

Presence semantics are defined in **RFC 008**.

See: `008-agent-discovery-directory.md`

---

## 11. Provisional Responses

Provisional response semantics (`PROCESSING`, `PROGRESS`, `INPUT_REQUIRED`) are defined in **RFC 006**.

See: `006-session-protocol.md`

---

## 12. Capability Namespacing & Versioning

Capability identifiers, versioning, and schema registry details are defined in **RFC 004**.

See: `004-capability-schema-registry.md`

---

## 13. Protocol Version Negotiation

### 13.1 Version Handshake

**Version Field Mapping**:
- Header field `v` is the **major** AMP version (integer).
- `HELLO`, `HELLO_ACK`, and `HELLO_REJECT` MUST be sent with `v = 1` as a stable negotiation envelope.
- `versions` in HELLO are semver strings; `major(x.y.z)` is the integer before the first dot.
- After selection, all subsequent messages MUST set `v = major(selected)`.
- If a non-handshake message is received with an unsupported `v`, the recipient MUST respond with ERROR 1004 (UNSUPPORTED_VERSION).

When establishing communication, agents negotiate the protocol version:

```
Agent A                              Agent B
   │                                    │
   │  HELLO {versions: ["1.0", "2.0"]}  │
   │ ──────────────────────────────────►│
   │                                    │
   │  HELLO_ACK {selected: "2.0"}       │
   │◄────────────────────────────────── │
   │                                    │
   │  [Proceed with AMP v2.0 semantics] │
```

### 13.2 Message Types

```
; Handshake (0x70-0x72)
HELLO           = 0x70    ; Initiate with supported versions
HELLO_ACK       = 0x71    ; Confirm selected version
HELLO_REJECT    = 0x72    ; No compatible version
```

### 13.3 HELLO Body

```cbor
{
  "v": 1,
  "typ": 0x70,  ; HELLO
  "body": {
    "versions": ["2.0", "1.0"],  ; preferred first
    "extensions": ["streaming", "compression"],
    "agent_info": {
      "name": "code-review-bot",
      "implementation": "amp-go/0.1.0"
    }
  }
}
```

### 13.4 Backward Compatibility

- Agents SHOULD support at least one prior major version
- Unknown message types MUST be responded to with ERROR
- Extensions are opt-in; core protocol remains stable

### 13.5 Version Negotiation State Machine

```
UNNEGOTIATED
  ├─ send HELLO → WAIT_ACK
  └─ receive HELLO → SELECT
WAIT_ACK
  ├─ HELLO_ACK(selected) → NEGOTIATED
  └─ HELLO_REJECT → FAILED
SELECT
  ├─ compatible version → send HELLO_ACK → NEGOTIATED
  └─ no compatible version → send HELLO_REJECT → FAILED
```

**Rules**:
- Only one active negotiation per peer at a time.
- After `NEGOTIATED`, all non-handshake messages MUST use `v = major(selected)`.

---

## 14. Interoperability

Ecosystem interoperability guidance (A2A/MCP bridges) is defined in **RFC 008**.

See: `008-agent-discovery-directory.md`

---

## 15. Error Codes

### 15.1 Error Message Structure

```cbor
{
  "typ": 0x0F,  ; ERROR
  "reply_to": "<original_message_id>",
  "body": {
    "code": 4001,
    "category": "client",
    "message": "Invalid capability version format",
    "details": {
      "field": "version",
      "expected": "semver",
      "received": "latest"
    },
    "retry": false
  }
}
```

### 15.2 Error Code Ranges

| Range | Category | Description |
|-------|----------|-------------|
| 1xxx | Protocol | Message format, encoding, signature errors |
| 2xxx | Routing | Delivery, relay, addressing errors |
| 3xxx | Security | Authentication, authorization, encryption errors |
| 4xxx | Client | Invalid request, bad parameters |
| 5xxx | Server | Processing failures, resource exhaustion |

### 15.3 Standard Error Codes

**Protocol Errors (1xxx)**:
| Code | Name | Description | Retry |
|------|------|-------------|-------|
| 1001 | INVALID_MESSAGE | Malformed CBOR or missing required fields | No |
| 1002 | INVALID_SIGNATURE | Signature verification failed | No |
| 1003 | INVALID_TIMESTAMP | Message expired or future-dated | No |
| 1004 | UNSUPPORTED_VERSION | Protocol version not supported | No |
| 1005 | UNKNOWN_TYPE | Message type not recognized | No |

**Routing Errors (2xxx)**:
| Code | Name | Description | Retry |
|------|------|-------------|-------|
| 2001 | RECIPIENT_NOT_FOUND | DID could not be resolved | Yes |
| 2002 | ENDPOINT_UNREACHABLE | Could not connect to endpoint | Yes |
| 2003 | RELAY_REJECTED | Relay refused to accept message (includes policy rejections, e.g., TTL=0 when recipient offline) | Yes |
| 2004 | TTL_EXPIRED | Message TTL exceeded | No |

**Security Errors (3xxx)**:
| Code | Name | Description | Retry |
|------|------|-------------|-------|
| 3001 | UNAUTHORIZED | Sender not authorized, or decryption failed (key mismatch, message not intended for recipient). **Privacy note**: Recipients SHOULD return 3001 for decryption failures rather than a distinct error to avoid leaking whether the recipient exists or can decrypt (oracle attack prevention). | No |
| 3002 | CONTACT_REQUIRED | Must request contact first (DISCOVERABLE) | No |
| 3003 | CONTACT_DENIED | Contact request was denied | No |
| 3004 | DELEGATION_INVALID | Delegation credential invalid or expired | No |
| 3005 | RATE_LIMITED | Too many requests | Yes (with backoff) |

**Client Errors (4xxx)**:
| Code | Name | Description | Retry |
|------|------|-------------|-------|
| 4001 | BAD_REQUEST | Invalid request parameters | No |
| 4002 | CAPABILITY_NOT_FOUND | Requested capability not available | No |
| 4003 | VERSION_MISMATCH | Capability version not supported | No |
| 4004 | SCHEMA_VIOLATION | Input doesn't match schema | No |

**Server Errors (5xxx)**:
| Code | Name | Description | Retry |
|------|------|-------------|-------|
| 5001 | INTERNAL_ERROR | Unexpected processing error | Yes |
| 5002 | UNAVAILABLE | Agent temporarily unavailable | Yes |
| 5003 | TIMEOUT | Processing timeout | Yes |
| 5004 | OVERLOADED | Agent at capacity | Yes (with backoff) |

---

## 16. Acknowledgment Semantics

### 16.1 ACK vs PROC_OK vs PROC_FAIL

| Message | Meaning | Sender | When |
|---------|---------|--------|------|
| `ACK` | "I received your message" | Relay or recipient | Immediately on receipt |
| `PROC_OK` | "I successfully processed your request" | Recipient only | After processing completes |
| `PROC_FAIL` | "I tried but failed to process" | Recipient only | After processing fails |

**ACK Source Disambiguation**:

Since ACK can come from either a relay or the final recipient, the ACK body MUST include source identification:

```cbor
{
  "typ": 0x03,  ; ACK
  "from": "did:web:...",              ; ACK sender's DID (signed)
  "reply_to": "<original_message_id>",
  "body": {
    "ack_source": "relay",            ; "relay" or "recipient"
    "received_at": 1707055200000,     ; timestamp
    ? "ack_target": "did:web:..."     ; which recipient (for multi-recipient messages)
  }
}
```

**Validation Rules (MUST)**:

| Rule | Requirement |
|------|-------------|
| **Recipient ACK** | When `ack_source` = "recipient", `from` MUST be in the original message's `to` field (or equal to `to` if single recipient) |
| **Relay ACK** | When `ack_source` = "relay", `from` MUST be a trusted relay (listed in sender's or recipient's DID Document as `AgentMessagingRelay` service) |
| **Signature** | ACK MUST be signed by `from`; the signature proves ACK authenticity |
| **Multi-recipient** | When original message has multiple recipients (`to` is array), ACK SHOULD include `ack_target` to indicate which recipient's delivery is being confirmed |

**Invalid ACK Handling**:
- `from` not matching `ack_source` semantics → reject as protocol error
- Unsigned or invalid signature → reject
- Unknown relay (not in trusted relay list) → MAY reject or log warning

**Why This Matters**:

| Scenario | Interpretation |
|----------|---------------|
| ACK from relay | Message accepted for delivery, not yet delivered |
| ACK from recipient | Message delivered to recipient's agent |
| PROC_OK from recipient | Message successfully processed |

**SLA Implications**:
- Relay ACK starts "delivery SLA" timer
- Recipient ACK confirms delivery
- PROC_OK/PROC_FAIL confirms processing

**Retry Logic**:
- No relay ACK within timeout → retry to same/different relay
- Relay ACK but no recipient ACK within TTL → message may be queued
- Recipient ACK but no PROC_* → processing may be slow (check PROCESSING/PROGRESS)

### 16.2 Idempotency

**Requirement**: Processing the same message ID multiple times MUST produce the same result.

**Implementation**:
```
On receiving message with id X:
  1. Check cache for X
  2. If found: return cached response (ACK/PROC_OK/PROC_FAIL)
  3. If not found:
     a. Process message
     b. Cache response with TTL = message.ttl
     c. Return response
```

**Cache Key**: `(sender_did, message_id)`

**Why Both?**: Same message may arrive via multiple paths (retry, relay redundancy). Idempotency ensures consistent behavior.

### 16.3 Retry Strategy

**Sender Retry**:
```
On no response within timeout:
  1. If error.retry == false: fail permanently
  2. If error.retry == true or no error:
     a. Compute base_wait = min(base * 2^attempt, max_backoff)
        base = 1s, max_backoff = 60s
     b. Add jitter: wait = base_wait * (0.5 + random(0, 0.5))
     c. Retry with SAME message ID
     d. Max attempts = 5
```

**Exponential Backoff with Jitter**:
| Attempt | Base Wait | With Jitter (range) |
|---------|-----------|---------------------|
| 1 | 1s | 0.5s - 1s |
| 2 | 2s | 1s - 2s |
| 3 | 4s | 2s - 4s |
| 4 | 8s | 4s - 8s |
| 5 | 16s | 8s - 16s |

**Why Jitter?** Prevents thundering herd when multiple agents retry simultaneously after a shared relay recovers. Random distribution spreads load.

### 16.4 Timeout Recommendations

| Message Type | Suggested Timeout |
|--------------|-------------------|
| PING/PONG | 5s |
| ACK | 10s |
| CAP_QUERY | 30s |
| CAP_INVOKE (simple) | 60s |
| CAP_INVOKE (complex) | Use PROCESSING/PROGRESS |
| CONTACT_REQUEST | 24h (async approval) |

### 16.5 Confirmation and Persistence Rules

**Definitions**:
- **Delivery confirmation**: ACK with `ack_source = "relay"` or `"recipient"`.
- **Processing confirmation**: PROC_OK or PROC_FAIL (or CAP_RESULT for CAP_INVOKE).

**Persistence Rules**:
- **Senders** MUST retain outbound messages until delivery confirmation or `ts + ttl`, whichever occurs first.
- **Relays** MUST retain messages until recipient ACK or `ts + ttl`, whichever occurs first.
- **Relays** MAY delete immediately after recipient ACK.
- **Recipients** SHOULD retain processed message IDs for at least `ttl` to enforce idempotency.

---

## 17. Registry Governance

### 17.1 What Requires Registration

| Registry | Examples | Authority |
|----------|----------|-----------|
| Message Type Codes | 0x01-0xFF | AMP Specification |
| Error Codes | 1001-5999 | AMP Specification |
| Extension Fields | ext.vendor.* | Vendor |

### 17.2 Allocation Policy

**Message Type Codes**:
- 0x00-0x7F: Reserved for AMP core (requires RFC)
- 0x80-0xEF: Available for standard extensions (requires registration)
- 0xF0-0xFF: Experimental/vendor-specific (no registration)

**Error Codes**:
- x000-x899: Reserved for AMP core
- x900-x999: Vendor-specific (no registration)

**Capability Namespaces**:
- Defined in RFC 004 (Capability Schema Registry & Compatibility)

### 17.3 Registration Process

1. Open issue on AMP specification repository
2. Provide: code/namespace, purpose, semantics
3. Review by maintainers
4. Merge into registry document

---

## 18. Open Questions

1. **Relay Protocol**: How are relays discovered? How are they authenticated?
2. **Message Persistence**: How long do relays store messages? (Partially addressed in §8.3)
3. **Group Messaging**: How to handle multi-agent collaboration?
4. **Reputation System**: How to bootstrap and maintain agent reputation scores?
5. **Payment Integration**: How to handle paid capability invocation?

---

## 19. References

### 19.1 Normative References

- [RFC 2119: Key words for use in RFCs](https://www.rfc-editor.org/rfc/rfc2119.html)
- [RFC 8174: Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words](https://www.rfc-editor.org/rfc/rfc8174.html)
- [RFC 3339: Date and Time on the Internet](https://www.rfc-editor.org/rfc/rfc3339.html)
- [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949.html)
- [CDDL (RFC 8610)](https://www.rfc-editor.org/rfc/rfc8610.html)
- [Ed25519 (RFC 8032)](https://www.rfc-editor.org/rfc/rfc8032.html)
- [COSE (RFC 9052)](https://www.rfc-editor.org/rfc/rfc9052.html)

### 19.2 Informative References

- [Agentries](https://agentries.xyz)
- [NaCl Cryptography](https://nacl.cr.yp.to/)
- [MTProto](https://core.telegram.org/mtproto)
- [DIDComm (reference)](https://identity.foundation/didcomm-messaging/spec/)
- [A2A Protocol](https://a2a-protocol.org)
- [MCP (Model Context Protocol)](https://modelcontextprotocol.io)
- [Semantic Versioning](https://semver.org)

---

## Appendix A. Test Vectors

The vectors below use deterministic CBOR (RFC 8949 §4.2). Hex strings are lowercase. Keys are test-only.

### A.1 Common Parameters

- Ed25519 private key (seed): `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`
- Ed25519 public key: `03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8`
- X25519 recipient private key: `1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100`
- X25519 recipient public key: `87968c1c1642bd0600f6ad869b88f92c9623d0dfc44f01deffe21c9add3dca5f`
- X25519 sender static private key: `8f8e8d8c8b8a898887868584838281807f7e7d7c7b7a79787776757473727170`
- X25519 sender static public key: `46d09ef40df38265c53eb1e834cab2eff2dda6e85866e5a0706348400502f27f`
- Nonce (24 bytes): `000102030405060708090a0b0c0d0e0f1011121314151617`
- from: `did:web:example.com:agent:alice`
- to: `did:web:example.com:agent:bob`
- ttl: `86400000`
- Vector DID mapping note: for deterministic fixtures, both example DIDs can resolve to the same signing key in a local test DID document. Production deployments MUST use distinct per-entity keys.

### A.2 Vector 1: MESSAGE (null body)

- ts: `1707055200000`
- id: `0000018d746b37000000000000000001`
- typ: `0x10`
- body_cbor: `f6`

Sig_Input (hex):
```
8466414d502d763140a6626964500000018d746b3700000000000000000162746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b37006374746c1a05265c0063747970106466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c69636541f6
```

Signature (hex):
```
ddfe6db4951b1244be2953963b3323d1957bf95f04e123b0e4283fec5267961c6af0752a2e6ccbbfe313d08107c3ccc45a79add798bc4afd1d78f89ae38fdb02
```

Message (hex):
```
a9617601626964500000018d746b3700000000000000000162746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b3700637369675840ddfe6db4951b1244be2953963b3323d1957bf95f04e123b0e4283fec5267961c6af0752a2e6ccbbfe313d08107c3ccc45a79add798bc4afd1d78f89ae38fdb026374746c1a05265c00637479701064626f6479f66466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365
```

### A.3 Vector 2: HELLO

- ts: `1707055201000`
- id: `0000018d746b3ae80000000000000002`
- typ: `0x70`
- body_cbor:
```
a36876657273696f6e738263312e3063322e306a6167656e745f696e666fa2646e616d6566616d702d676f6e696d706c656d656e746174696f6e6c616d702d676f2f302e312e306a657874656e73696f6e73816973747265616d696e67
```

Sig_Input (hex):
```
8466414d502d763140a6626964500000018d746b3ae8000000000000000262746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b3ae86374746c1a05265c006374797018706466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365585da36876657273696f6e738263312e3063322e306a6167656e745f696e666fa2646e616d6566616d702d676f6e696d706c656d656e746174696f6e6c616d702d676f2f302e312e306a657874656e73696f6e73816973747265616d696e67
```

Signature (hex):
```
3d94b24e329a3cd13847eda767878474a18177179e98d3c5c1eccec5a5c0d391100fbf28c088967bb44dfe0031c4222d40cd6a6f3f57af9719556034b05bab05
```

Message (hex):
```
a9617601626964500000018d746b3ae8000000000000000262746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b3ae86373696758403d94b24e329a3cd13847eda767878474a18177179e98d3c5c1eccec5a5c0d391100fbf28c088967bb44dfe0031c4222d40cd6a6f3f57af9719556034b05bab056374746c1a05265c0063747970187064626f6479a36876657273696f6e738263312e3063322e306a6167656e745f696e666fa2646e616d6566616d702d676f6e696d706c656d656e746174696f6e6c616d702d676f2f302e312e306a657874656e73696f6e73816973747265616d696e676466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365
```

### A.4 Vector 3: ACK

- ts: `1707055202000`
- id: `0000018d746b3ed00000000000000003`
- typ: `0x03`
- reply_to: `0000018d746b37000000000000000001`
- body_cbor:
```
a36a61636b5f736f7572636569726563697069656e746a61636b5f746172676574781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626b72656365697665645f61741b0000018d746b40c4
```

Sig_Input (hex):
```
8466414d502d763140a7626964500000018d746b3ed0000000000000000362746f781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c6963656274731b0000018d746b3ed06374746c1a05265c0063747970036466726f6d781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f62687265706c795f746f500000018d746b370000000000000000015855a36a61636b5f736f7572636569726563697069656e746a61636b5f746172676574781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626b72656365697665645f61741b0000018d746b40c4
```

Signature (hex):
```
d18b0711cfedd531cc4ac1ea26cee7ce31827df504587578b4d50897a4a61582f4eb9bfe20fd11ca84e008df73d33972a672c434d7078daf5d1a866af73fad0d
```

Message (hex):
```
aa617601626964500000018d746b3ed0000000000000000362746f781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c6963656274731b0000018d746b3ed0637369675840d18b0711cfedd531cc4ac1ea26cee7ce31827df504587578b4d50897a4a61582f4eb9bfe20fd11ca84e008df73d33972a672c434d7078daf5d1a866af73fad0d6374746c1a05265c00637479700364626f6479a36a61636b5f736f7572636569726563697069656e746a61636b5f746172676574781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626b72656365697665645f61741b0000018d746b40c46466726f6d781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f62687265706c795f746f500000018d746b37000000000000000001
```

### A.5 Vector 4: STREAM_START / STREAM_DATA / STREAM_END

- stream_id: `stream-001`
- chunk bytes: `68656c6c6f`
- hash (sha256): `2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824`

STREAM_START:
- ts: `1707055203000`
- id: `0000018d746b42b80000000000000004`
- typ: `0x13`
- body_cbor:
```
a76866696c656e616d656968656c6c6f2e74787469686173685f616c676f667368613235366973747265616d5f69646a73747265616d2d3030316a6368756e6b5f73697a65056a746f74616c5f73697a65056c636f6e74656e745f747970656a746578742f706c61696e6c746f74616c5f6368756e6b7301
```
- sig_input:
```
8466414d502d763140a6626964500000018d746b42b8000000000000000462746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b42b86374746c1a05265c0063747970136466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c6963655878a76866696c656e616d656968656c6c6f2e74787469686173685f616c676f667368613235366973747265616d5f69646a73747265616d2d3030316a6368756e6b5f73697a65056a746f74616c5f73697a65056c636f6e74656e745f747970656a746578742f706c61696e6c746f74616c5f6368756e6b7301
```
- signature:
```
56ad1b38f984a729a63b8848b88cd9c5a7810e531f799b749d4dc1ad356b81e5aa28e5118d6c2cf4bf94c61bf982c85c0e0f48e46a9e0c13376cb7ce750aa006
```
- message:
```
a9617601626964500000018d746b42b8000000000000000462746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b42b863736967584056ad1b38f984a729a63b8848b88cd9c5a7810e531f799b749d4dc1ad356b81e5aa28e5118d6c2cf4bf94c61bf982c85c0e0f48e46a9e0c13376cb7ce750aa0066374746c1a05265c00637479701364626f6479a76866696c656e616d656968656c6c6f2e74787469686173685f616c676f667368613235366973747265616d5f69646a73747265616d2d3030316a6368756e6b5f73697a65056a746f74616c5f73697a65056c636f6e74656e745f747970656a746578742f706c61696e6c746f74616c5f6368756e6b73016466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365
```

STREAM_DATA:
- ts: `1707055203001`
- id: `0000018d746b42b90000000000000005`
- typ: `0x14`
- body_cbor:
```
a364646174614568656c6c6f65696e646578006973747265616d5f69646a73747265616d2d303031
```
- sig_input:
```
8466414d502d763140a6626964500000018d746b42b9000000000000000562746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b42b96374746c1a05265c0063747970146466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c6963655828a364646174614568656c6c6f65696e646578006973747265616d5f69646a73747265616d2d303031
```
- signature:
```
7ed51a5e33658449836bdfe62a84985f31a3ff409bfacf25f2613c6e94ac99b7a475efbbe57ec2d96db31ead0679e50a4bfd0d2378b0da0e005852b374370102
```
- message:
```
a9617601626964500000018d746b42b9000000000000000562746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b42b96373696758407ed51a5e33658449836bdfe62a84985f31a3ff409bfacf25f2613c6e94ac99b7a475efbbe57ec2d96db31ead0679e50a4bfd0d2378b0da0e005852b3743701026374746c1a05265c00637479701464626f6479a364646174614568656c6c6f65696e646578006973747265616d5f69646a73747265616d2d3030316466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365
```

STREAM_END:
- ts: `1707055203002`
- id: `0000018d746b42ba0000000000000006`
- typ: `0x15`
- body_cbor:
```
a2646861736858202cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b98246973747265616d5f69646a73747265616d2d303031
```
- sig_input:
```
8466414d502d763140a6626964500000018d746b42ba000000000000000662746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b42ba6374746c1a05265c0063747970156466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365583da2646861736858202cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b98246973747265616d5f69646a73747265616d2d303031
```
- signature:
```
75056f0085fb7d9cd45599a8093897a154e04f7685fda0e57b2c73d6f4100c6087a3fb7f7e58e344132b78cc54692472b5d295bbfe295ec45d4296c76a0f0a0c
```
- message:
```
a9617601626964500000018d746b42ba000000000000000662746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b42ba63736967584075056f0085fb7d9cd45599a8093897a154e04f7685fda0e57b2c73d6f4100c6087a3fb7f7e58e344132b78cc54692472b5d295bbfe295ec45d4296c76a0f0a0c6374746c1a05265c00637479701564626f6479a2646861736858202cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b98246973747265616d5f69646a73747265616d2d3030316466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365
```

### A.6 Vector 5: Encrypted MESSAGE (authcrypt)

- ts: `1707055204000`
- id: `0000018d746b46a00000000000000007`
- typ: `0x10`
- body_cbor: `a1636d736766736563726574`
- ciphertext (tag || ciphertext):
```
924706080f2aa18f82f7b18ac051c9884fbc614779749f98c1031101
```

Sig_Input (hex):
```
8466414d502d763140a6626964500000018d746b46a0000000000000000762746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b46a06374746c1a05265c0063747970106466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c6963654ca1636d736766736563726574
```

Signature (hex):
```
f9b70bbd8de3e7aba29b2f7ac2de00d3f5d021b651d687f059a86a0e5b5da7f90a970839892f1ae629c4d4aa1b1ee247ba657cf502d7c621b38950fda5710d09
```

Message (hex):
```
a9617601626964500000018d746b46a0000000000000000762746f781d6469643a7765623a6578616d706c652e636f6d3a6167656e743a626f626274731b0000018d746b46a063656e63a463616c6778185832353531392d5853616c736132302d506f6c7931333035646d6f646569617574686372797074656e6f6e63655818000102030405060708090a0b0c0d0e0f10111213141516176a63697068657274657874581c924706080f2aa18f82f7b18ac051c9884fbc614779749f98c1031101637369675840f9b70bbd8de3e7aba29b2f7ac2de00d3f5d021b651d687f059a86a0e5b5da7f90a970839892f1ae629c4d4aa1b1ee247ba657cf502d7c621b38950fda5710d096374746c1a05265c0063747970106466726f6d781f6469643a7765623a6578616d706c652e636f6d3a6167656e743a616c696365
```

### A.7 Negative Vectors (Expected Failures)

These vectors are defined as mutations over the positive vectors above.

| Vector | Mutation | Expected Result |
|--------|----------|-----------------|
| N1 | Flip any 1 bit in `A.2` signature | Reject with `1002 INVALID_SIGNATURE` |
| N2 | Keep `A.2` bytes but evaluate with `now > ts + ttl` | Reject with `1003 INVALID_TIMESTAMP` |
| N3 | In `A.6`, flip 1 byte in `ciphertext` | Reject with `3001 UNAUTHORIZED` (or `1001` if decrypt output is invalid CBOR after implementation pipeline) |
| N4 | Change `typ` in `A.2` from `0x70` to unassigned value | Reject with `1005 UNKNOWN_TYPE` |
| N5 | In `A.4` ACK, set `ack_source = "relay"` while `from` is not a trusted relay DID | Reject as protocol error (recommended `1001 INVALID_MESSAGE`) |

---

## Appendix B. Implementation Notes

These notes are non-normative and highlight common interoperability pitfalls.

- Deterministic CBOR: always canonicalize map key order and integer widths before signing; avoid floating point.
- Null body: when there is no payload, `body` MUST be CBOR null (`0xF6`) and MUST be signed; `body` is absent only when `enc` is present.
- Sig_Input: include only fields that are present; `null` is not equivalent to an absent `reply_to` or `thread_id`.
- Encrypted messages: decrypt first, then verify signature on the decrypted bytes; do not re-encode decrypted bytes.
- Message IDs: first 8 bytes MUST equal `ts` (ms, big-endian); last 8 bytes MUST be CSPRNG output.
- Replay cache: key by `(sender_did, message_id)` and retain entries for at least `ttl`.
- Batch processing: validate each inner message independently; use `batch_index` for per-item errors.
- Streaming: compute hash over concatenated chunk data in index order; handle out-of-order and duplicate chunks idempotently.

---

## Changelog

Versioning note: public version numbers were reset on 2026-02-06 for external publication. `Legacy Version` preserves the previous internal sequence.

| Date | Version | Legacy Version | Author | Changes |
|------|---------|----------------|--------|---------|
| 2026-02-04 | 0.1 | 1.0 | Ryan Cooper | Initial draft |
| 2026-02-04 | 0.2 | 1.1 | Ryan Cooper | Round 1 feedback from Jason |
| 2026-02-04 | 0.3 | 2.0 | Ryan Cooper | Major revision: binary protocol (CBOR), capability invocation, document/credential exchange |
| 2026-02-04 | 0.4 | 2.1 | Ryan Cooper | Added Section 9: Agentries Integration (opt-in AMP discovery via DID Document service); Full English translation |
| 2026-02-04 | 0.5 | 2.2 | Ryan Cooper, Jason Huang | Three-tier visibility model (PRIVATE/DISCOVERABLE/OPEN); Contact request flow for gated agents |
| 2026-02-04 | 0.6 | 2.3 | Ryan Cooper | Policy-based auto-approval mechanism for DISCOVERABLE agents; Human-in-the-loop as optional fallback |
| 2026-02-04 | 0.7 | 3.0 | Ryan Cooper | Major feature additions: Presence & Status (Section 10), Provisional Responses (Section 11), Capability Namespacing & Versioning (Section 12), Protocol Version Negotiation (Section 13), Interoperability with A2A/MCP (Section 14) |
| 2026-02-04 | 0.8 | 3.1 | Ryan Cooper | Redesigned Presence: capability signals (raw metrics) instead of intent signals (discrete states). Protocol transmits data; UI derives labels. |
| 2026-02-04 | 0.9 | 3.2 | Ryan Cooper | Consolidated message type code registry (§4.3); unified streaming semantics (STREAM_START/DATA/END); added registry governance reference |
| 2026-02-04 | 0.10 | 4.0 | Ryan Cooper | Security hardening: deterministic CBOR encoding (§8.1), Sig_structure (§8.1), ts/ttl offline handling (§8.3), replay protection (§8.4). New sections: Error Codes (§15), Acknowledgment Semantics & Idempotency (§16), Registry Governance (§17) |
| 2026-02-04 | 0.11 | 5.0 | Ryan Cooper | **Security audit fixes**: Two-layer envelope design (§8.1) - inner signature now covers typ/to/ts/ttl/reply_to/thread_id; Sign-then-encrypt with enc_digest binding (§8.6); Extension field security warnings (§8.7); Unified TTL-driven timestamp validation (§8.3) - removed conflicting 5-min rule; Streaming specification (§6.2) - chunk ordering, index base, hash computation standardized; ACK source disambiguation (§16.1) - ack_source field distinguishes relay vs recipient |
| 2026-02-04 | 0.12 | 5.1 | Ryan Cooper | **Consistency fixes**: Simplified to pure sign-then-encrypt (removed enc_digest from Sig_Input - binding via decrypt-then-verify); Clarified relay envelope as transport-layer only (not in CDDL); Fixed §4.2 timestamp rule to match §8.3 (TTL-driven); Fixed STREAM_END semantics (sender sends, receiver ACKs); Fixed ext reference (§8.1→§8.7); Added ACK from/ack_source consistency requirement |
| 2026-02-04 | 0.13 | 5.2 | Ryan Cooper | **Boundary conditions**: Sign-then-encrypt implications documented (relay cannot verify, verification order, re-encryption attack); Relay metadata marked out-of-scope (removed example); ACK validation rules as MUST (recipient/relay DID verification, multi-recipient ack_target field) |
| 2026-02-04 | 0.14 | 5.3 | Ryan Cooper | **Consistency fixes**: ttl now REQUIRED (was optional, caused Sig_Input ambiguity); StE clarified: enc.ciphertext MUST encrypt deterministic_cbor(body) bytes exactly; anoncrypt privacy boundary clarified (does NOT hide from relay, only prevents recipient proving sender to third parties); Removed "Two-Layer Envelope" section (was confusing with out-of-scope relay metadata); Fixed terminology (Sig_structure→Sig_Input); Unified timestamp consistency language (MUST match within ±1s) |
| 2026-02-04 | 0.15 | 5.4 | Ryan Cooper | **Final consistency pass**: Removed "Default TTL" validation row (ttl is required); §3.2 now references Sig_Input instead of "canonicalized CBOR encoding of message"; anoncrypt description unified across §3.2 and §8.6; "Ciphertext binding" reworded to "Tamper detection" with re-encryption limitation noted; StE rationale no longer mentions "sender hidden"; Added note about partial examples omitting required fields |
| 2026-02-04 | 0.16 | 5.5 | Ryan Cooper | **Edge case definitions**: TTL=0 semantics defined (no relay storage, immediate forward, reject if offline); §3.2 notes decrypt-then-verify for encrypted messages; §8.2 expanded with explicit deterministic CBOR requirement for unencrypted body signing/verification |
| 2026-02-04 | 0.17 | 5.6 | Ryan Cooper | **Refinements**: §8.2 now includes explicit numbered steps for encrypted message verification; TTL=0 error changed from 2002 to 2003 (RELAY_REJECTED - policy rejection, not transport failure); §4.1 field notes now consolidates deterministic CBOR requirement for body |
| 2026-02-04 | 0.18 | 5.7 | Ryan Cooper | **Polish**: §15.3 RELAY_REJECTED description includes policy rejections (e.g., TTL=0); §4.1 field notes clarifies re-encoding is for unencrypted only (encrypted uses raw bytes); §8.2 step 7 points to §8.6 for failure handling |
| 2026-02-04 | 0.19 | 5.8 | Ryan Cooper | **Clarity**: §8.2 "Important" scoped to unencrypted messages only; §8.6 adds explicit failure handling list (decryption/signature/CBOR failures) |
| 2026-02-04 | 0.20 | 5.9 | Ryan Cooper | §8.6 failure list: added "Sig_Input reconstruction fails (missing required fields)" |
| 2026-02-04 | 0.21 | 5.10 | Ryan Cooper | **Error code mapping**: §8.6 failure handling now maps to error codes (1001, 1002, 3001); §8.2 adds parallel failure handling table for unencrypted messages |
| 2026-02-04 | 0.22 | 5.11 | Ryan Cooper | §15.3 UNAUTHORIZED clarified to include decryption/key mismatch; §8.2 and §8.6 failure tables note "non-retryable" with pointer to §15.3 |
| 2026-02-04 | 0.23 | 5.12 | Ryan Cooper | §8.2/§8.6 changed to "see §15.3 for retry semantics" (single source of truth); §15.3 UNAUTHORIZED adds privacy note about oracle attack prevention |
| 2026-02-04 | 0.24 | 5.13 | Ryan Cooper | §8.6 decryption failure row cross-references §15.3 privacy note |
| 2026-02-06 | 0.25 | 5.14 | Nowa | Clarified body/enc exclusivity and null payload signing; defined encrypted-payload CDDL; specified message-id encoding; added normative language; aligned version negotiation with major `v`; documented AgentMessagingGated service type |
| 2026-02-06 | 0.26 | 5.15 | Nowa | Added TOC and terminology; added CDDL for message bodies; defined batch messages; added state machines for key flows; added privacy considerations; clarified confirmation/persistence rules; split references into normative/informative; aligned examples with capability IDs |
| 2026-02-06 | 0.27 | 5.16 | Nowa | Moved discovery/presence/provisional/capability details to RFCs 004/006/008; slimmed core CDDL to core message bodies; added cross-RFC pointers |
| 2026-02-06 | 0.28 | 5.17 | Nowa | Added conformance criteria and test vectors/implementation notes for AMP Core |
| 2026-02-06 | 0.29 | 5.18 | Nowa | Clarified authcrypt/anoncrypt byte-level profiles and verification steps; added DID key selection policy; fixed ACK section markdown; added negative test vectors and test DID mapping note |
| 2026-02-06 | 0.30 | 5.19 | Nowa | Simplified AMP 001 to a single encryption profile (`authcrypt`); removed anoncrypt-specific rules; updated encrypted test vector to authcrypt |
