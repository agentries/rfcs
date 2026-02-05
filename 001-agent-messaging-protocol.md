# RFC 001: Agent Messaging Protocol (AMP)

**Status**: Draft  
**Authors**: Ryan Cooper, Jason Apple Huang  
**Created**: 2026-02-04  
**Updated**: 2026-02-04  
**Version**: 5.13

---

## Abstract

AMP (Agent Messaging Protocol) is a native communication protocol designed for the AI Agent ecosystem.

**Core Positioning**:
- 🎯 **Goal**: Native messaging protocol for AI Agent ecosystem
- ⚡ **Features**: Binary, efficient, agent-to-agent communication, capability invocation, document/credential exchange
- 🔗 **Position**: Standalone protocol (not a DIDComm profile)

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

---

## 2. Requirements

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
- **R7**: Messages MUST be persisted until confirmed
- **R8**: Protocol MUST support asynchronous communication

### 2.4 Efficiency
- **R9**: Message format MUST be binary (CBOR)
- **R10**: Support batch message transmission
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
- Modes:
  - `authcrypt`: Authenticated encryption (recipient can cryptographically verify sender)
  - `anoncrypt`: Anonymous encryption (recipient cannot prove sender to third parties via encryption; see §8.6 for privacy boundaries)

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
amp-message = {
  ; Header
  v: uint,                    ; Protocol version (1)
  id: bstr,                   ; Message ID (16 bytes: 8 timestamp + 8 random)
  typ: uint,                  ; Message type
  ts: uint,                   ; Unix timestamp (milliseconds) - when created
  ttl: uint,                  ; Time-to-live (milliseconds) - REQUIRED (see §8.1, §8.3)
  
  ; Routing
  from: did,                  ; Sender DID
  to: did / [+ did],          ; Recipient DID(s)
  ? reply_to: bstr,           ; Message ID being replied to
  ? thread_id: bstr,          ; Conversation/thread ID
  
  ; Security
  sig: bstr,                  ; Ed25519 signature (see §8.1 for Sig_Input)
  ? enc: encrypted-payload,   ; Encrypted payload (replaces body)
  
  ; Payload
  ? body: any,                ; Message body (structure determined by type)
  
  ; Extension
  ? ext: {* tstr => any},     ; Extension fields (NOT signed, see §8.7)
}

did = tstr  ; "did:web:agentries.xyz:agent:xxx"
```

**Field Notes**:
- `ts` + `ttl` determine message validity window (see §8.3)
- `sig` covers ALL semantically critical fields (see §8.1 Sig_Input)
  - Includes: id, typ, ts, ttl, from, to, reply_to, thread_id
  - Always signs **plaintext** body (for encrypted messages, decrypt first then verify; see §8.6)
- `body` MUST be encoded using **deterministic CBOR** (RFC 8949 §4.2) for signing; for unencrypted messages, verifiers MUST re-encode body deterministically before verification; for encrypted messages, verifiers use decrypted bytes directly (see §8.2)
- `ext` is NOT signed — treat as untrusted (see §8.7 for security implications)

**Note on Examples**: Code examples throughout this document may omit some required fields (e.g., `ttl`, `sig`) for brevity. All required fields listed above MUST be present in actual implementations.

### 4.2 Message ID Design

Inspired by MTProto, message IDs contain time information:

```
Message ID (16 bytes):
┌────────────────────┬────────────────────┐
│  Timestamp (8B)    │  Random (8B)       │
│  Millisecond Unix  │  Random number     │
└────────────────────┴────────────────────┘

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
; 0x16-0x1F reserved

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

**Streaming Clarification**: Large documents use the generic streaming mechanism (STREAM_START/DATA/END) with document metadata in the body. DOC_SEND is for small inline documents only.

---

## 5. Capability Invocation

Core interaction pattern between agents:

### 5.1 Capability Query

```cbor
; Request
{
  "typ": 0x20,  ; CAP_QUERY
  "body": {
    "filter": {
      "type": "code-review",
      "version": ">=1.0"
    }
  }
}

; Response
{
  "typ": 0x21,  ; CAP_DECLARE
  "body": {
    "capabilities": [
      {
        "type": "code-review",
        "version": "2.0",
        "input_schema": "https://...",
        "output_schema": "https://..."
      }
    ]
  }
}
```

### 5.2 Capability Invocation (RPC)

```cbor
; Request
{
  "typ": 0x22,  ; CAP_INVOKE
  "body": {
    "capability": "code-review",
    "version": "2.0",
    "params": {
      "code": "fn main() {...}",
      "language": "rust"
    },
    "timeout_ms": 30000
  }
}

; Response
{
  "typ": 0x23,  ; CAP_RESULT
  "body": {
    "status": "success",
    "result": {
      "issues": [...],
      "suggestions": [...]
    }
  }
}
```

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
; ACK body: { "stream_id": "doc-abc123", "chunks_received": 10, "verified": true }
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
- `body` - **plaintext** payload content (see §8.6 for encrypted messages)

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

**For Unencrypted Messages** (body field present):

Verification steps:
1. Extract `from` DID from message
2. Resolve DID Document to obtain public key
3. Encode `body` using deterministic CBOR (RFC 8949 §4.2) → body_bytes
4. Reconstruct Sig_Input using body_bytes
5. Verify `sig` using Ed25519_Verify(public_key, CBOR_Encode(Sig_Input), sig)
6. MUST reject if verification fails

**For Encrypted Messages** (enc field present):

1. Extract `from` DID from message
2. Resolve DID Document to obtain public key
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

- Sensitive content SHOULD use `authcrypt` or `anoncrypt`
- Relays SHOULD NOT be able to read encrypted content
- Key agreement uses X25519 (Curve25519 ECDH)
- Symmetric encryption uses XSalsa20-Poly1305

### 8.6 Sign-Then-Encrypt

**Design Choice**: AMP uses **sign-then-encrypt** (StE) — the signature covers the plaintext body, then the body is encrypted.

**Why StE?**
- Signature proves sender created the actual content (not just ciphertext)
- Works naturally with both `authcrypt` and `anoncrypt` modes
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

**Critical**: The `enc.ciphertext` MUST encrypt the exact `deterministic_cbor(body)` byte sequence used in Sig_Input. Encrypting a different serialization will cause signature verification to fail after decryption.

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
    "mode": "authcrypt",      ; or "anoncrypt"
    "epk": h'...',            ; ephemeral public key
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

**Limitation**: If an attacker knows the plaintext, they can re-encrypt it with different parameters (new nonce/ephemeral key) and the signature remains valid. This does not compromise authenticity (content unchanged) but means the ciphertext itself is not cryptographically bound to the signature. See "Re-encryption attack" in Important Implications below.

**Encryption Modes**:
- `authcrypt`: Recipient can verify sender's identity (authenticated encryption)
- `anoncrypt`: Recipient cannot cryptographically verify sender from encryption alone

**Privacy Note on `anoncrypt`**: The `from` field remains in the unencrypted message header and is signed. Therefore, `anoncrypt` does NOT hide the sender from relays or observers — they can still see `from`. The purpose of `anoncrypt` is to prevent the *recipient* from cryptographically proving (to third parties) who sent the message based on encryption alone. Sender authentication still occurs via signature verification.

**Important Implications**:

| Implication | Description |
|-------------|-------------|
| **Relay cannot verify signature** | Relays see only ciphertext; they cannot verify sender signature without decryption. Anti-abuse/gating at relay layer must use other mechanisms (e.g., sender reputation, rate limits, outer transport auth). |
| **Verification order** | Recipients MUST decrypt first, then verify signature. When `enc` is present, `body` exists only inside the ciphertext. |
| **Re-encryption attack** | An attacker who knows the plaintext can re-encrypt with different parameters (new ephemeral key, nonce) and the signature remains valid. This does NOT compromise authenticity (content unchanged) but changes ciphertext/routing artifacts. If "encryption parameters immutability" is required, additional mechanisms are needed (out of scope for AMP core). |

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

---

## 9. Agentries Integration (AMP Discovery)

### 9.1 Design Principle: Visibility Levels

Agents have granular control over their discoverability and contactability through three visibility levels:

| Level | In Directory | Contactable | Use Case |
|-------|--------------|-------------|----------|
| `PRIVATE` | No | No | Internal agents, no external communication |
| `DISCOVERABLE` | Yes | Requires approval | Visible but gated, like LinkedIn connections |
| `OPEN` | Yes | Yes | Fully accessible public agents |

**Rationale**:
- **Privacy**: Agents may need identity without exposure
- **Security**: Public endpoints increase attack surface
- **Control**: Gating mechanism for high-value agents
- **Flexibility**: Different agents have different needs

Analogy: Having an ID card ≠ publishing your phone number. But you might list yourself in a directory with "contact me for inquiries."

### 9.2 DID Document Service Declaration

Agents wishing to receive AMP messages declare a service in their DID Document:

```json
{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://agentries.xyz/contexts/v1"
  ],
  "id": "did:web:agentries.xyz:agent:xxx",
  "verificationMethod": [...],
  
  "service": [
    {
      "id": "did:web:agentries.xyz:agent:xxx#amp",
      "type": "AgentMessaging",
      "serviceEndpoint": "https://amp.example.com/agent/xxx"
    },
    {
      "id": "did:web:agentries.xyz:agent:xxx#amp-relay",
      "type": "AgentMessagingRelay", 
      "serviceEndpoint": "https://relay.agentries.xyz"
    }
  ]
}
```

### 9.3 Service Types

| Type | Description | Use Case |
|------|-------------|----------|
| `AgentMessaging` | Direct AMP endpoint | Agent runs its own receiving service |
| `AgentMessagingRelay` | Relay endpoint | Receive via Agentries Relay |

### 9.4 Discovery Flow

```
Sender                                Recipient
   │                                      │
   │  1. Resolve DID                      │
   │ ───────────────────────────────────► │
   │                                      │
   │  2. Check DID Document               │
   │     AgentMessaging service present?  │
   │                                      │
   │  [Yes] 3a. Send to endpoint          │
   │ ───────────────────────────────────► │
   │                                      │
   │  [No]  3b. Cannot send message       │
   │        (Agent has not enabled AMP)   │
   │                                      │
```

### 9.5 Registration Options in Agentries

When registering an agent, the user chooses a visibility level:

```
Visibility Level:
○ PRIVATE     - Not listed, not contactable
○ DISCOVERABLE - Listed, requires approval to contact  
○ OPEN        - Listed, directly contactable

[If DISCOVERABLE or OPEN]
  Endpoint options:
  ○ Use Agentries Relay (recommended)
  ○ Self-hosted endpoint: [________________]
```

**DID Document implications**:
- **PRIVATE**: No AMP service, no directory listing
- **DISCOVERABLE**: `AgentMessagingGated` service type
- **OPEN**: `AgentMessaging` or `AgentMessagingRelay` service type

### 9.6 Contact Request Flow (DISCOVERABLE agents)

For agents with `DISCOVERABLE` visibility, a contact request handshake is required:

```
Requester                           Target (DISCOVERABLE)
    │                                      │
    │  1. Find agent in directory          │
    │                                      │
    │  2. CONTACT_REQUEST                  │
    │     {reason: "...", capabilities: []}│
    │ ────────────────────────────────────►│
    │                                      │
    │  3. Target reviews request           │
    │     (manual or policy-based)         │
    │                                      │
    │  4. CONTACT_RESPONSE                 │
    │     {status: "approved"|"denied"}    │
    │◄──────────────────────────────────── │
    │                                      │
    │  [If approved]                       │
    │  5. Normal AMP communication         │
    │◄────────────────────────────────────►│
```

**Message Types** (added to Control category):

```
; Contact (0x06-0x08)
CONTACT_REQUEST   = 0x06    ; Request to establish contact
CONTACT_RESPONSE  = 0x07    ; Approve/deny contact request
CONTACT_REVOKE    = 0x08    ; Revoke previously granted contact
```

**Contact Request Body**:

```cbor
{
  "typ": 0x06,  ; CONTACT_REQUEST
  "body": {
    "reason": "Collaboration on code review tasks",
    "capabilities_offered": ["code-review", "testing"],
    "capabilities_requested": ["deployment"],
    "expires": "2026-02-11T00:00:00Z"
  }
}
```

**Contact Response Body**:

```cbor
{
  "typ": 0x07,  ; CONTACT_RESPONSE
  "reply_to": "<request_id>",
  "body": {
    "status": "approved",  ; or "denied", "pending"
    "granted_until": "2026-03-04T00:00:00Z",
    "restrictions": {
      "rate_limit": 100,  ; messages per hour
      "capabilities": ["code-review"]  ; subset of requested
    }
  }
}
```

### 9.7 Approval Mechanism: Policy-Based Auto-Approval

Since AMP is an agent-to-agent protocol, approval decisions SHOULD be automated via configurable policies rather than requiring human intervention.

**Design Principle**: Agents pre-configure approval policies; the system executes automatically. Human-in-the-loop is optional, not the default.

**Policy Types**:

| Policy | Description | Example |
|--------|-------------|---------|
| **Organization Trust** | Same organization → auto-approve | `org:acme-corp` agents approved |
| **Reputation Threshold** | Score-based gating | `reputation > 0.8` → approve |
| **Capability Whitelist** | Safe operations auto-approved | `read-only` → approve |
| **Credential Verification** | VC holders approved | Has `TrustedDeveloper` VC → approve |
| **Explicit Allowlist** | Pre-approved DIDs | `did:web:...:agent:trusted-bot` → approve |
| **Default Deny** | Fallback for unmatched | No match → deny |

**Policy Configuration Example**:

```json
{
  "approval_policy": {
    "rules": [
      {
        "name": "same-org",
        "condition": {"org": "$self.org"},
        "action": "approve",
        "restrictions": {"rate_limit": 1000}
      },
      {
        "name": "high-reputation",
        "condition": {"reputation": {"$gte": 0.8}},
        "action": "approve",
        "restrictions": {"rate_limit": 100}
      },
      {
        "name": "read-only-requests",
        "condition": {"capabilities_requested": {"$subset": ["read", "query"]}},
        "action": "approve"
      },
      {
        "name": "verified-developers",
        "condition": {"credentials": {"$contains": "TrustedDeveloperVC"}},
        "action": "approve"
      },
      {
        "name": "default",
        "condition": true,
        "action": "deny"
      }
    ],
    "human_fallback": false  ; optional: queue for human review if true
  }
}
```

**Evaluation Order**: Rules are evaluated top-to-bottom; first match wins.

**Human-in-the-Loop (Optional)**:
- High-value agents MAY enable `human_fallback: true`
- Unmatched requests queue for human review
- This is the exception, not the norm

**Analogy**: Firewall rules — policies are pre-configured, system auto-executes, humans intervene only for exceptions.

### 9.8 Service Types (Updated)

| Type | Visibility | Description |
|------|------------|-------------|
| `AgentMessaging` | OPEN | Direct endpoint, anyone can message |
| `AgentMessagingRelay` | OPEN | Via relay, anyone can message |
| `AgentMessagingGated` | DISCOVERABLE | Requires contact approval first |
| *(no service)* | PRIVATE | Not contactable |

### 9.9 UI Status Display

Agent profiles display visibility and messaging status:

```
┌─────────────────────────────────────┐
│  Agent: code-review-bot             │
│  DID: did:web:agentries.xyz:...     │
│                                     │
│  📩 AMP: Open                       │  ← green, directly contactable
│  or                                 │
│  🔔 AMP: Discoverable               │  ← yellow, request required
│  or                                 │
│  🔒 AMP: Private                    │  ← gray, not contactable
└─────────────────────────────────────┘
```

---

## 10. Presence & Status

Agents SHOULD advertise their capacity to enable intelligent routing and avoid wasted requests.

### 10.1 Design Principle: Capability Signals, Not Intent Signals

Traditional presence (XMPP-style) uses discrete states like AVAILABLE/BUSY/AWAY. These are **intent signals** designed for humans ("I don't want to be disturbed").

Agent presence should be **capability signals**: quantitative data that answers:
1. Can you handle my request right now?
2. How long will I wait?
3. Should I try a different agent?

**AMP transmits raw capacity data. UI layers derive human-friendly labels.**

### 10.2 Presence Message

```cbor
{
  "typ": 0x60,  ; PRESENCE
  "body": {
    "capacity": {
      "concurrent_max": 10,      ; maximum parallel requests
      "concurrent_current": 3,   ; currently processing
      "queue_depth": 0,          ; requests waiting
      "accepting_requests": true ; actively accepting new work
    },
    "performance": {
      "estimated_response_ms": 500,    ; typical response time
      "p95_response_ms": 2000          ; 95th percentile
    },
    "offline_until": null,       ; null = online, timestamp = temporarily away
    "expires": "2026-02-04T13:00:00Z"  ; presence data TTL
  }
}
```

### 10.3 Deriving Human-Friendly Status (Informative)

UIs MAY derive labels from capacity data:

```
if offline_until != null:
    display "AWAY"
elif not accepting_requests:
    display "DND"
elif concurrent_current / concurrent_max > 0.8:
    display "BUSY"
else:
    display "AVAILABLE"
```

This is a UI concern, not protocol concern. The protocol transmits data; applications decide presentation.

### 10.4 Presence Discovery

Agents MAY:
1. **Push**: Broadcast presence to known peers
2. **Pull**: Respond to `PRESENCE_QUERY` requests
3. **Subscribe**: Allow peers to subscribe to presence changes

```
; Presence (0x60-0x63)
PRESENCE        = 0x60    ; Presence announcement
PRESENCE_QUERY  = 0x61    ; Query agent presence
PRESENCE_SUB    = 0x62    ; Subscribe to presence updates
PRESENCE_UNSUB  = 0x63    ; Unsubscribe
```

### 10.5 Use Cases

**Intelligent Routing**: Load balancer queries presence of multiple agents, routes to lowest `concurrent_current / concurrent_max` ratio.

**SLA Estimation**: Caller checks `estimated_response_ms` before invoking, sets appropriate timeout.

**Graceful Degradation**: When `accepting_requests: false`, caller knows to queue locally or try alternative agent.

**Capacity Planning**: Monitor `queue_depth` trends to decide when to scale agent instances.

---

## 11. Provisional Responses

For long-running operations, agents SHOULD send provisional responses to indicate progress.

### 11.1 Message Types

```
; Provisional (0x09-0x0B)
PROCESSING      = 0x09    ; Request received, working on it
PROGRESS        = 0x0A    ; Progress update with percentage/ETA
INPUT_REQUIRED  = 0x0B    ; Blocked, need additional input
```

### 11.2 Flow Example

```
Client                              Server
   │                                   │
   │  CAP_INVOKE (complex task)        │
   │ ─────────────────────────────────►│
   │                                   │
   │  PROCESSING                       │
   │◄───────────────────────────────── │
   │                                   │
   │  PROGRESS {pct: 30, eta_ms: 5000} │
   │◄───────────────────────────────── │
   │                                   │
   │  PROGRESS {pct: 80, eta_ms: 1000} │
   │◄───────────────────────────────── │
   │                                   │
   │  CAP_RESULT (final result)        │
   │◄───────────────────────────────── │
```

### 11.3 Progress Body

```cbor
{
  "typ": 0x0A,  ; PROGRESS
  "reply_to": "<original_request_id>",
  "body": {
    "percentage": 30,
    "eta_ms": 5000,
    "status_text": "Analyzing code structure...",
    "cancellable": true
  }
}
```

### 11.4 Input Required

When an agent needs additional information to continue:

```cbor
{
  "typ": 0x0B,  ; INPUT_REQUIRED
  "reply_to": "<original_request_id>",
  "body": {
    "prompt": "Which branch should I review?",
    "options": ["main", "develop", "feature/x"],
    "timeout_ms": 60000
  }
}
```

---

## 12. Capability Namespacing & Versioning

### 12.1 Capability Identifier Format

Capabilities use reverse-domain namespacing with semantic versioning:

```
<namespace>.<capability>:<major>.<minor>

Examples:
  org.agentries.code-review:2.0
  com.acme.data-analysis:1.3
  io.github.user.custom-tool:0.1
```

### 12.2 Version Negotiation in CAP_QUERY

```cbor
{
  "typ": 0x20,  ; CAP_QUERY
  "body": {
    "filter": {
      "capability": "org.agentries.code-review",
      "version": ">=2.0 <3.0"  ; semver range
    }
  }
}
```

### 12.3 CAP_DECLARE with Versions

```cbor
{
  "typ": 0x21,  ; CAP_DECLARE
  "body": {
    "capabilities": [
      {
        "id": "org.agentries.code-review:2.1",
        "deprecated_versions": ["1.0", "1.1"],
        "input_schema": "https://schema.agentries.xyz/code-review/2.1/input.json",
        "output_schema": "https://schema.agentries.xyz/code-review/2.1/output.json"
      }
    ]
  }
}
```

---

## 13. Protocol Version Negotiation

### 13.1 Version Handshake

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

---

## 14. Interoperability

### 14.1 Design Principle

**AMP is an independent protocol**, with its own identity model (DIDs), encoding (CBOR), and features (delegation, presence). However, AMP provides optional compatibility layers to interoperate with other agent ecosystems.

### 14.2 A2A Compatibility Layer

AMP agents MAY expose an A2A-compatible Agent Card for discovery in the A2A ecosystem:

```json
{
  "name": "code-review-bot",
  "description": "Automated code review agent",
  "url": "https://agents.example.com/code-review",
  "protocols": {
    "a2a": "https://agents.example.com/code-review/a2a",
    "amp": "did:web:agentries.xyz:agent:code-review#amp"
  },
  "capabilities": [
    {
      "name": "code-review",
      "description": "Review code for issues and suggestions"
    }
  ]
}
```

### 14.3 Protocol Selection

When both A2A and AMP are available, agents negotiate:

```
1. Discover agent via A2A directory (Agent Card)
2. Check if AMP endpoint is listed
3. If both support AMP → use AMP (more efficient)
4. If only A2A → fall back to A2A (compatible)
```

### 14.4 Bridge Agents

For agents that only speak A2A, a bridge can translate:

```
┌──────────┐    AMP     ┌──────────┐    A2A    ┌──────────┐
│ AMP-only │ ─────────► │  Bridge  │ ────────► │ A2A-only │
│  Agent   │            │  Agent   │           │  Agent   │
└──────────┘            └──────────┘           └──────────┘
```

### 14.5 MCP Tool Bridge

AMP agents can expose capabilities as MCP tools:

```
AMP Capability: org.agentries.code-review:2.0
       ↓
MCP Tool: {
  "name": "code_review",
  "description": "...",
  "inputSchema": {...}
}
```

This allows LLM applications using MCP to invoke AMP agent capabilities.

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
```

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

---

## 17. Registry Governance

### 17.1 What Requires Registration

| Registry | Examples | Authority |
|----------|----------|-----------|
| Message Type Codes | 0x01-0xFF | AMP Specification |
| Error Codes | 1001-5999 | AMP Specification |
| Capability Namespaces | org.agentries.* | Namespace owner |
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
- Reverse domain names prevent collision
- No central registration required
- Owners responsible for their namespace

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

- [Agentries](https://agentries.xyz)
- [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949.html)
- [CBOR Deterministic Encoding (RFC 8949 §4.2)](https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2)
- [CDDL (RFC 8610)](https://www.rfc-editor.org/rfc/rfc8610.html)
- [Ed25519 (RFC 8032)](https://www.rfc-editor.org/rfc/rfc8032.html)
- [COSE (RFC 9052)](https://www.rfc-editor.org/rfc/rfc9052.html)
- [NaCl Cryptography](https://nacl.cr.yp.to/)
- [MTProto](https://core.telegram.org/mtproto)
- [DIDComm (reference)](https://identity.foundation/didcomm-messaging/spec/)
- [A2A Protocol](https://a2a-protocol.org)
- [MCP (Model Context Protocol)](https://modelcontextprotocol.io)
- [Semantic Versioning](https://semver.org)

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-04 | 1.0 | Ryan Cooper | Initial draft |
| 2026-02-04 | 1.1 | Ryan Cooper | Round 1 feedback from Jason |
| 2026-02-04 | 2.0 | Ryan Cooper | Major revision: binary protocol (CBOR), capability invocation, document/credential exchange |
| 2026-02-04 | 2.1 | Ryan Cooper | Added Section 9: Agentries Integration (opt-in AMP discovery via DID Document service); Full English translation |
| 2026-02-04 | 2.2 | Ryan Cooper, Jason Huang | Three-tier visibility model (PRIVATE/DISCOVERABLE/OPEN); Contact request flow for gated agents |
| 2026-02-04 | 2.3 | Ryan Cooper | Policy-based auto-approval mechanism for DISCOVERABLE agents; Human-in-the-loop as optional fallback |
| 2026-02-04 | 3.0 | Ryan Cooper | Major feature additions: Presence & Status (Section 10), Provisional Responses (Section 11), Capability Namespacing & Versioning (Section 12), Protocol Version Negotiation (Section 13), Interoperability with A2A/MCP (Section 14) |
| 2026-02-04 | 3.1 | Ryan Cooper | Redesigned Presence: capability signals (raw metrics) instead of intent signals (discrete states). Protocol transmits data; UI derives labels. |
| 2026-02-04 | 3.2 | Ryan Cooper | Consolidated message type code registry (§4.3); unified streaming semantics (STREAM_START/DATA/END); added registry governance reference |
| 2026-02-04 | 4.0 | Ryan Cooper | Security hardening: deterministic CBOR encoding (§8.1), Sig_structure (§8.1), ts/ttl offline handling (§8.3), replay protection (§8.4). New sections: Error Codes (§15), Acknowledgment Semantics & Idempotency (§16), Registry Governance (§17) |
| 2026-02-04 | 5.0 | Ryan Cooper | **Security audit fixes**: Two-layer envelope design (§8.1) - inner signature now covers typ/to/ts/ttl/reply_to/thread_id; Sign-then-encrypt with enc_digest binding (§8.6); Extension field security warnings (§8.7); Unified TTL-driven timestamp validation (§8.3) - removed conflicting 5-min rule; Streaming specification (§6.2) - chunk ordering, index base, hash computation standardized; ACK source disambiguation (§16.1) - ack_source field distinguishes relay vs recipient |
| 2026-02-04 | 5.1 | Ryan Cooper | **Consistency fixes**: Simplified to pure sign-then-encrypt (removed enc_digest from Sig_Input - binding via decrypt-then-verify); Clarified relay envelope as transport-layer only (not in CDDL); Fixed §4.2 timestamp rule to match §8.3 (TTL-driven); Fixed STREAM_END semantics (sender sends, receiver ACKs); Fixed ext reference (§8.1→§8.7); Added ACK from/ack_source consistency requirement |
| 2026-02-04 | 5.2 | Ryan Cooper | **Boundary conditions**: Sign-then-encrypt implications documented (relay cannot verify, verification order, re-encryption attack); Relay metadata marked out-of-scope (removed example); ACK validation rules as MUST (recipient/relay DID verification, multi-recipient ack_target field) |
| 2026-02-04 | 5.3 | Ryan Cooper | **Consistency fixes**: ttl now REQUIRED (was optional, caused Sig_Input ambiguity); StE clarified: enc.ciphertext MUST encrypt deterministic_cbor(body) bytes exactly; anoncrypt privacy boundary clarified (does NOT hide from relay, only prevents recipient proving sender to third parties); Removed "Two-Layer Envelope" section (was confusing with out-of-scope relay metadata); Fixed terminology (Sig_structure→Sig_Input); Unified timestamp consistency language (MUST match within ±1s) |
| 2026-02-04 | 5.4 | Ryan Cooper | **Final consistency pass**: Removed "Default TTL" validation row (ttl is required); §3.2 now references Sig_Input instead of "canonicalized CBOR encoding of message"; anoncrypt description unified across §3.2 and §8.6; "Ciphertext binding" reworded to "Tamper detection" with re-encryption limitation noted; StE rationale no longer mentions "sender hidden"; Added note about partial examples omitting required fields |
| 2026-02-04 | 5.5 | Ryan Cooper | **Edge case definitions**: TTL=0 semantics defined (no relay storage, immediate forward, reject if offline); §3.2 notes decrypt-then-verify for encrypted messages; §8.2 expanded with explicit deterministic CBOR requirement for unencrypted body signing/verification |
| 2026-02-04 | 5.6 | Ryan Cooper | **Refinements**: §8.2 now includes explicit numbered steps for encrypted message verification; TTL=0 error changed from 2002 to 2003 (RELAY_REJECTED - policy rejection, not transport failure); §4.1 field notes now consolidates deterministic CBOR requirement for body |
| 2026-02-04 | 5.7 | Ryan Cooper | **Polish**: §15.3 RELAY_REJECTED description includes policy rejections (e.g., TTL=0); §4.1 field notes clarifies re-encoding is for unencrypted only (encrypted uses raw bytes); §8.2 step 7 points to §8.6 for failure handling |
| 2026-02-04 | 5.8 | Ryan Cooper | **Clarity**: §8.2 "Important" scoped to unencrypted messages only; §8.6 adds explicit failure handling list (decryption/signature/CBOR failures) |
| 2026-02-04 | 5.9 | Ryan Cooper | §8.6 failure list: added "Sig_Input reconstruction fails (missing required fields)" |
| 2026-02-04 | 5.10 | Ryan Cooper | **Error code mapping**: §8.6 failure handling now maps to error codes (1001, 1002, 3001); §8.2 adds parallel failure handling table for unencrypted messages |
| 2026-02-04 | 5.11 | Ryan Cooper | §15.3 UNAUTHORIZED clarified to include decryption/key mismatch; §8.2 and §8.6 failure tables note "non-retryable" with pointer to §15.3 |
| 2026-02-04 | 5.12 | Ryan Cooper | §8.2/§8.6 changed to "see §15.3 for retry semantics" (single source of truth); §15.3 UNAUTHORIZED adds privacy note about oracle attack prevention |
| 2026-02-04 | 5.13 | Ryan Cooper | §8.6 decryption failure row cross-references §15.3 privacy note |
