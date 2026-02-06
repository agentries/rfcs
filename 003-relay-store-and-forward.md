# RFC 003: Relay & Store-and-Forward

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-06
**Updated**: 2026-02-06
**Version**: 0.4

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)
- RFC 002: Transport Bindings (TCP-first, HTTP/WS mappings)

**Related:**
- RFC 008: Agent Discovery & Directory (service declaration and route discovery)

---

## Abstract

This RFC defines relay queueing, retention, delivery, and commit semantics for AMP messages when recipients are intermittently reachable. It standardizes relay behavior for at-least-once delivery, TTL-bound storage, polling/webhook retrieval, and recipient-ACK-based delivery commit, while keeping transport details in RFC 002 and message semantics in RFC 001.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
2.1 Terminology
3. Boundary Contracts with Other RFCs
4. Relay Data Model
4.1 Transfer Receipt Object
4.2 Transfer Receipt Algorithm Profile (MTI)
4.3 Commit Receipt Object (Dual-Custody)
5. Relay Semantics
5.1 Ingress Acceptance
5.2 Retention and Expiry
5.3 Delivery Attempt Policy
5.4 Polling and Webhook Retrieval
5.5 Delivery Commit Rules
5.6 Relay-to-Relay Handoff (Federation Profile)
5.6.1 Deterministic Downstream Selection
5.6.2 Loop Prevention
5.6.3 Global Idempotency and Duplicate Suppression
5.6.4 Auditable Custody Transfer and Rollback
5.6.5 Handoff Processing Algorithm (Normative)
5.6.6 Dual-Custody Commit Feedback
5.6.7 Federation Timeouts and Retry (MTI)
6. State Machines
7. Error Handling and Retry
8. Versioning and Compatibility
9. Security Considerations
10. Privacy Considerations
11. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions

---

## 1. Problem Statement and Scope

AMP message semantics are defined in RFC 001, but real deployments need asynchronous delivery when agents are offline, firewalled, or mobile. Without a normative relay store-and-forward model, interoperability is reduced to only always-online direct endpoints.

This RFC defines:
- Relay queueing and persistence behavior.
- Recipient retrieval semantics via polling/webhook paths.
- Delivery commit conditions tied to RFC 001 ACK semantics.
- Optional relay-to-relay custody transfer semantics.

Out of scope:
- Relay pricing/settlement.
- Relay reputation scoring.
- Transport framing/auth details (RFC 002).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted per RFC 2119 and RFC 8174.

`Relay Core Profile` (minimum conformant relay):
- MUST implement TTL-bound queueing.
- MUST implement at-least-once recipient delivery behavior.
- MUST support polling retrieval semantics defined here.
- MUST commit delivery only via valid recipient ACK (RFC 001 Section 16).

`Relay Push Profile` (optional extension):
- Adds webhook delivery behavior.

`Relay Federation Profile` (optional extension; mandatory rules when implemented):
- MUST implement relay-to-relay handoff behavior.
- MUST implement deterministic downstream relay selection.
- MUST enforce loop prevention (`hop_limit` + visited-path check).
- MUST implement duplicate suppression keyed by `(from_did, msg_id, recipient_did)` for the TTL window.
- MUST implement auditable custody transfer with explicit rollback on downstream acceptance timeout/failure.
- MUST use RFC 002 relay-forward wrapper and transfer-receipt object for federation interoperability.
- MUST support downstream commit feedback in dual-custody mode via commit-receipt.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Queue record | Relay-internal record binding one AMP payload to one or more target recipients. |
| Delivery attempt | A relay attempt to deliver one queued message to one recipient endpoint. |
| Commit | State transition marking recipient delivery as complete after valid recipient ACK. |
| Custody transfer | Handoff from one relay to another where downstream relay assumes delivery responsibility. |
| Relay path | Ordered list of relay identities already traversed by this message. |
| Hop limit | Remaining relay-to-relay forwarding budget to prevent loops. |
| Dedupe key | Tuple `(from_did, msg_id, recipient_did)` used to suppress duplicate enqueue/delivery. |
| Transfer receipt | Authenticated evidence that downstream relay accepted custody. |
| Commit receipt | Authenticated evidence that downstream relay observed recipient terminal state. |

---

## 3. Boundary Contracts with Other RFCs

`RFC 001 boundary`:
- Relay MUST preserve raw AMP bytes and MUST NOT modify signed fields.
- Relay MUST respect `ts + ttl` validity and TTL=0 semantics.
- Delivery commit MUST follow RFC 001 ACK validation (`ack_source`, `from`, `reply_to`, signature).

`RFC 002 boundary`:
- Transport handshake, framing, auth, and endpoint priority are defined in RFC 002.
- Polling/webhook wrappers are defined in RFC 002; this RFC defines their delivery semantics.
- Principal/from DID binding policy is enforced per RFC 002.
- Federation principal binding uses RFC 002 relay-to-relay rule (`transport principal == upstream_relay`).
- Federation handoff transport uses RFC 002 relay-forward wrapper and response mapping.

`RFC 008 boundary`:
- Endpoint discovery uses DID service declarations (`AgentMessaging`, `AgentMessagingRelay`, `AgentMessagingGated`).
- Contact-gated endpoints MUST respect RFC 008 policy before forwarding.
- Federation routing MUST consult `relayCapabilities` metadata when present (`transferModes`, `maxHopLimit`, `receiptAlgs`).

---

## 4. Relay Data Model

The model below is normative for behavior, not a mandated storage implementation.

```cddl
queue-record = {
  "msg_id": bstr .size 16,
  "raw_message": bstr,                 ; exact AMP bytes
  "from": tstr,
  "recipients": [+ recipient-state],
  "accepted_at": uint,
  "expires_at": uint,                  ; ts + ttl
  "attempts": uint,
  "status": "queued" / "dispatching" / "done" / "expired" / "rejected",
  ? "relay_path": [* tstr],            ; relay IDs traversed
  ? "hop_limit": uint,                 ; decremented on relay handoff
  ? "transfer": {
    "mode": "single" / "dual",
    ? "downstream": tstr,
    ? "state": "none" / "pending" / "accepted" / "rolled_back" / "commit_reported",
    ? "receipt": bstr,
    ? "commit_receipt": bstr
  }
}

recipient-state = {
  "did": tstr,
  "state": "pending" / "inflight" / "delivered" / "expired" / "failed",
  ? "last_attempt_at": uint,
  ? "last_error": uint,
  ? "acked_at": uint
}
```

Requirements:
- `raw_message` MUST remain byte-identical to ingress payload.
- `expires_at` MUST equal `ts + ttl` from the AMP envelope.
- Multi-recipient messages MUST track per-recipient state.
- Federation forwarding MUST append local relay ID to `relay_path`.
- Federation forwarding MUST decrement `hop_limit` by 1 on each relay handoff.
- Relay MUST enforce dedupe key `(from_did, msg_id, recipient_did)` over the active TTL window.

### 4.1 Transfer Receipt Object

The federation transfer receipt format is normative and MUST be interoperable.

```cddl
transfer-receipt-payload = {
  "receipt_v": 1,
  "msg_id": bstr .size 16,
  "from_did": tstr,
  "recipient_did": tstr,
  "upstream_relay": tstr,
  "downstream_relay": tstr,
  "accepted_at": uint,
  "hop_limit_remaining": uint,
  "status": "accepted" / "rejected",
  ? "reason_code": uint
}

transfer-receipt = bstr
; COSE_Sign1 over deterministic_cbor(transfer-receipt-payload)
```

Validation requirements:
- Signature key MUST belong to `downstream_relay` per DID resolution policy.
- `msg_id`, `from_did`, `recipient_did`, and `upstream_relay` MUST match the handoff request.
- `status = accepted` is REQUIRED for custody acceptance.
- Protected header `alg` MUST be present and MUST match local profile policy.
- Protected header `kid` MUST resolve to a verification method under `downstream_relay` DID `assertionMethod`.

### 4.2 Transfer Receipt Algorithm Profile (MTI)

To avoid federation interoperability drift, transfer-receipt signing algorithms are profile-constrained:

- Signers and verifiers in Federation Profile MUST implement COSE `alg = -8` (EdDSA) with OKP `crv = Ed25519`.
- Implementations SHOULD additionally support COSE `alg = -7` (ES256, P-256) for compatibility with existing PKI fleets.
- A receiver MUST reject transfer receipts using unsupported algorithms with `3001`.
- If federation is configured by explicit out-of-band trust policy and RFC 008 capability metadata is unavailable, implementations MUST assume Ed25519 support only.
- If RFC 008 capability metadata is available, `receiptAlgs` MUST include `-8`; additional algorithms MAY be negotiated by intersection.

### 4.3 Commit Receipt Object (Dual-Custody)

Dual-custody mode requires downstream-to-upstream terminal delivery feedback.

```cddl
commit-receipt-payload = {
  "commit_v": 1,
  "msg_id": bstr .size 16,
  "from_did": tstr,
  "recipient_did": tstr,
  "upstream_relay": tstr,
  "downstream_relay": tstr,
  "result": "delivered" / "failed" / "expired",
  "committed_at": uint,
  ? "reason_code": uint
}

commit-receipt = bstr
; COSE_Sign1 over deterministic_cbor(commit-receipt-payload)
```

Validation requirements:
- Signature key MUST belong to `downstream_relay` DID `assertionMethod`.
- `msg_id`, `from_did`, `recipient_did`, and `upstream_relay` MUST match prior accepted transfer context.
- Protected header `alg` and `kid` constraints are identical to Section 4.1.

---

## 5. Relay Semantics

### 5.1 Ingress Acceptance

On relay ingress:
1. Parse incoming AMP payload enough to extract required envelope fields (`id`, `ts`, `ttl`, `from`, `to`, `typ`).
2. Apply RFC 001 temporal checks (`INVALID_TIMESTAMP` for expired/future-invalid messages).
3. Enforce policy checks (auth, size limits, principal/from policy via RFC 002).
4. If accepted and `ttl > 0`, create/update queue record.

TTL=0 rule:
- Relay MUST NOT queue TTL=0 messages.
- Relay MUST attempt immediate delivery only.
- If immediate next-hop delivery is unavailable, relay MUST reject with `2003 RELAY_REJECTED`.

### 5.2 Retention and Expiry

- Relay MUST retain queued entries until recipient commit or `expires_at`, whichever comes first.
- Relay MUST NOT extend TTL.
- Relay MUST mark expired per-recipient states and remove fully expired records.
- Relay MAY enforce max accepted TTL by policy (reject at ingress).
- Relay MAY use a transient queue status `expired` before final cleanup/removal.

### 5.3 Delivery Attempt Policy

Delivery endpoint selection in this section applies to relay-to-recipient delivery and MUST follow RFC 002 priority:

```
amps > wss > https
```

Rules:
- Within same binding class, preserve DID service order.
- Delivery attempts SHOULD use exponential backoff with jitter.
- Relay SHOULD cap retries by expiry horizon, not fixed attempt count.
- Same message MAY be delivered multiple times (at-least-once).
- Relay-to-relay endpoint selection is defined in Section 5.6.1.

### 5.4 Polling and Webhook Retrieval

`Polling`:
- Relay MUST implement RFC 002 polling wrapper.
- Polling MAY redeliver already seen messages until commit.
- Cursor progression MUST be monotonic for a consumer identity.

`Webhook` (if supported):
- Relay MUST implement RFC 002 webhook wrapper and signature checks.
- Webhook delivery is at-least-once and MAY redeliver until commit.

### 5.5 Delivery Commit Rules

Delivery commit is recipient-scoped and ACK-driven.

A relay MUST mark recipient delivery committed only when receiving a valid recipient ACK meeting all conditions:
- `typ = ACK` and `body.ack_source = "recipient"`.
- ACK signature is valid per RFC 001.
- ACK `reply_to` equals original message `id`.
- ACK `from` DID matches one intended recipient DID of the original message.

Additional rules:
- `PROC_OK` / `PROC_FAIL` do NOT replace commit; they are processing outcomes.
- For multi-recipient messages, relay MUST commit recipients independently.
- Queue record becomes `done` only when all intended recipients are terminal (`delivered` / `failed` / `expired`).

### 5.6 Relay-to-Relay Handoff (Federation Profile)

A relay in Federation Profile MAY hand off queued messages to another trusted relay.

Handoff rules:
- Upstream relay MUST forward raw AMP bytes unchanged.
- Downstream relay MUST apply normal ingress rules.
- Handoff metadata MUST be transported using RFC 002 relay-forward wrapper.
- Federation handoff MUST be recipient-scoped: one handoff object covers exactly one `recipient_did`.
- For AMP messages with multiple recipients, upstream relay MUST split into per-recipient federation handoffs before transfer.
- Upstream relay MUST transfer custody only after downstream acceptance evidence.
- Acceptance evidence MUST be a valid `transfer-receipt` object (Section 4.1), auditable by `msg_id`.

#### 5.6.1 Deterministic Downstream Selection

For each target recipient, relay selection MUST be deterministic:
- First, select candidate relay services of type `AgentMessagingRelay`.
- Then apply DID service order.
- Candidate relay MUST satisfy requested transfer mode and receipt algorithm compatibility from RFC 008 capability metadata.
- Federation handoff MUST use RFC 002 `POST /amp/v1/relay/forward` endpoint semantics.
- If still tied, apply lexical order of endpoint URI as stable tie-break.

#### 5.6.2 Loop Prevention

- Each relay handoff MUST carry `relay_path` and `hop_limit`.
- If `hop_limit` is absent at federation ingress, default value MUST be `8`.
- Relay MUST reject forwarding when:
  - local relay ID already exists in `relay_path`; or
  - `hop_limit == 0` before next handoff.
- Loop/hop-limit rejection MUST return `2003 RELAY_REJECTED` with a machine-readable reason.

#### 5.6.3 Global Idempotency and Duplicate Suppression

- Relay MUST evaluate dedupe key `(from_did, msg_id, recipient_did)` before enqueue or transfer.
- If duplicate key exists in active TTL window, relay MUST NOT create a second active queue entry for that recipient.
- Duplicate handoff requests MUST be idempotent: relay SHOULD return prior acceptance result when available.

#### 5.6.4 Auditable Custody Transfer and Rollback

- Upstream relay MUST enter `transfer.state = pending` before handoff completion.
- Upstream relay MUST mark transfer `accepted` only after valid transfer receipt verification.
- If downstream acceptance times out or fails, upstream relay MUST mark transfer `rolled_back` and resume local delivery attempts.
- In single-custody mode, upstream relay MUST delete local copy only after transfer is `accepted`.
- In dual-custody mode, upstream relay MUST retain local copy until valid commit-receipt is received or local expiry is reached.

#### 5.6.5 Handoff Processing Algorithm (Normative)

For each federation handoff attempt, relays MUST process `relay_path` and `hop_limit` in this exact order:
1. Validate local relay ID is not already present in incoming `relay_path`; if present, reject (`2003`).
2. Validate incoming `hop_limit > 0`; if not, reject (`2003`).
3. Compute outgoing `hop_limit_next = hop_limit - 1`.
4. Compute outgoing `relay_path_next = relay_path + [local_relay_id]`.
5. Forward using `hop_limit_next` and `relay_path_next` in RFC 002 relay-forward wrapper.

#### 5.6.6 Dual-Custody Commit Feedback

When `transfer_mode = "dual"` and downstream relay observes recipient terminal state:
- Downstream relay MUST send a valid `commit-receipt` to upstream relay via RFC 002 relay commit-report endpoint.
- Upstream relay MUST verify commit-receipt and transition transfer state to `commit_reported`.
- For `result = delivered`, upstream relay MAY remove local copy immediately after audit persistence.
- For `result = failed` or `result = expired`, upstream relay MUST apply local retry/expiry policy but MUST NOT treat as delivered.

#### 5.6.7 Federation Timeouts and Retry (MTI)

Federation profile defaults (minimum interoperable baseline):
- `handoff_accept_timeout_ms` default `5000` (RECOMMENDED range `1000..10000`).
- `handoff_retry_backoff_ms` sequence `1000, 2000, 4000, ...`, capped at `30000`.
- `handoff_max_attempts` default `3`.
- Implementations MUST stop handoff retries at earliest of: successful acceptance, local expiry (`ts + ttl`), or max attempts.
- In dual-custody mode, downstream relay SHOULD send commit-receipt within `commit_report_sla_ms = min(60000, remaining_ttl_ms)`.

---

## 6. State Machines

### 6.1 Queue Record State Machine

```
NEW
  -> QUEUED              (accepted, ttl>0)
  -> REJECTED            (policy/validation reject)
QUEUED
  -> DISPATCHING         (attempt started)
  -> EXPIRED             (now > expires_at with pending recipients)
DISPATCHING
  -> QUEUED              (attempt failed, retry later)
  -> DONE                (all recipients terminal)
  -> EXPIRED             (expiry reached with pending recipients)
EXPIRED
  -> DONE                (pending recipients marked expired; record finalized)
DONE / REJECTED
  -> TERMINAL
```

### 6.2 Recipient State Machine

```
PENDING
  -> INFLIGHT            (delivery attempt)
INFLIGHT
  -> PENDING             (attempt failed/retry)
  -> DELIVERED           (valid recipient ACK)
  -> FAILED              (non-retryable reject/policy/auth failure)
  -> EXPIRED             (ttl reached)
PENDING
  -> FAILED              (non-retryable reject before first attempt)
  -> EXPIRED             (ttl reached)
DELIVERED / EXPIRED / FAILED
  -> TERMINAL
```

### 6.3 Federation Transfer State Machine

```
LOCAL_QUEUED
  -> TRANSFER_PENDING     (handoff attempt started)
TRANSFER_PENDING
  -> TRANSFER_ACCEPTED    (valid downstream transfer receipt)
  -> ROLLED_BACK          (timeout/reject/invalid receipt)
TRANSFER_ACCEPTED
  -> LOCAL_REMOVED        (single-custody mode)
  -> DUAL_ACTIVE          (dual-custody mode)
DUAL_ACTIVE
  -> COMMIT_REPORTED      (valid commit-receipt received)
COMMIT_REPORTED
  -> LOCAL_REMOVED        (audit persisted and local finalize)
ROLLED_BACK
  -> LOCAL_QUEUED         (resume local routing/retry)
DUAL_ACTIVE
  -> LOCAL_REMOVED        (local expiry reached without commit-receipt)
```

---

## 7. Error Handling and Retry

Suggested mapping (aligned with RFC 001):

| Condition | Error Code | Notes |
|----------|------------|-------|
| Envelope malformed | 1001 | Invalid message format |
| Timestamp invalid / expired at ingress | 1003 | RFC 001 temporal rules |
| Unsupported AMP version | 1004 | Non-negotiated/unsupported `v` |
| Unsupported federation object version | 1004 | `receipt_v` / `commit_v` not supported |
| Federation wrapper malformed | 1001 | Missing/invalid relay-forward fields |
| Recipient DID unresolved | 2001 | No resolvable recipient identity |
| Endpoint unavailable | 2002 | Route exists but currently unreachable |
| Relay policy rejection (rate, ttl=0 offline, abuse, loop, hop-limit) | 2003 | Policy-controlled refusal |
| Message expired in queue | 2004 | TTL elapsed before commit |
| Unauthorized principal/from | 3001 | RFC 002 binding policy |
| Invalid/unauthenticated transfer receipt | 3001 | Federation receipt verification failed |
| Invalid/unauthenticated commit receipt | 3001 | Dual-custody commit report verification failed |
| Unsupported transfer receipt algorithm | 3001 | COSE `alg` not supported by policy/profile |
| Internal relay failure | 5001 | Unexpected server error |

Retry guidance:
- Sender SHOULD retry transient 2xxx/5xxx conditions with backoff.
- Relay internal retries MUST stop at expiry.
- Recipients MUST tolerate duplicates by RFC 001 idempotency.
- Federation handoff timeout/failure MUST trigger upstream rollback before any alternate route attempt.
- Duplicate federation deliveries MUST be suppressed by dedupe key and handled idempotently.
- Dual-custody commit report timeout SHOULD trigger local expiry/retry policy and MUST be auditable.

---

## 8. Versioning and Compatibility

- This RFC does not define a new AMP message version.
- Transport compatibility and wrapper versions are inherited from RFC 002.
- Federation receipt object versions are explicit and currently fixed: `receipt_v = 1`, `commit_v = 1`.
- Unsupported federation object versions MUST be rejected with `1004`.
- Future queue metadata extensions MUST be backward-compatible and MUST NOT alter raw AMP bytes.

---

## 9. Security Considerations

- Relay MUST enforce transport authentication and principal/from DID policy (RFC 002).
- Relay MUST treat `ext` as untrusted and MUST NOT use unsigned metadata for authorization decisions.
- For encrypted messages, relay typically cannot verify sender signature; anti-abuse controls MUST rely on transport auth, policy, and rate limits.
- Replay/duplicate defenses MUST leverage message ID and recipient commit state.
- Federation links MUST use authenticated channels and explicit trust policy.
- Federation relays MUST verify transfer receipt authenticity before custody change.
- Loop-prevention controls (`relay_path`, `hop_limit`) are REQUIRED anti-amplification safeguards.
- Transfer receipts MUST be cryptographically verifiable and strongly bound to handoff tuple fields.
- Commit receipts MUST be cryptographically verifiable and strongly bound to accepted transfer context.

---

## 10. Privacy Considerations

- Relays observe routing metadata (`from`, `to`, `typ`, `ts`, `ttl`) even when body is encrypted.
- Relays SHOULD minimize retention of payloads and logs beyond operational requirements.
- Audit logs SHOULD avoid sensitive body disclosure and prefer message IDs plus minimal metadata.
- Webhook and polling traces SHOULD avoid leaking recipient activity patterns beyond policy requirements.

---

## 11. References

### 11.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 002: Transport Bindings (TCP-first, HTTP/WS mappings)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 8949: CBOR
- RFC 9052: CBOR Object Signing and Encryption (COSE)
- RFC 9053: COSE Algorithms

### 11.2 Informative References

- RFC 008: Agent Discovery & Directory

---

## Appendix A. Minimal Test Vectors

### A.1 TTL=0 Offline Rejection

Input:
- message with `ttl=0`
- recipient endpoint unavailable

Expected:
- relay does not queue message
- reject with `2003`

### A.2 Polling Redelivery Until Commit

Input:
- queued message for recipient
- recipient polls twice without sending ACK

Expected:
- message may appear in both polling responses
- recipient state remains `pending/inflight` until ACK

### A.3 Recipient ACK Commit

Input:
- valid recipient ACK (`ack_source=recipient`, `reply_to=msg_id`, valid signature)

Expected:
- recipient state becomes `delivered`
- queue record transitions to `done` when all recipients delivered

### A.4 Multi-Recipient Partial Commit

Input:
- message with recipients `[A, B]`
- ACK from A only

Expected:
- A committed, B still pending
- queue record not `done` until B committed or expired

### A.5 Federation Loop Rejection

Input:
- handoff with `relay_path` already containing local relay ID

Expected:
- relay rejects forwarding with `2003`
- no downstream handoff is attempted

### A.6 Hop-Limit Exhaustion

Input:
- handoff with `hop_limit=0`

Expected:
- relay rejects forwarding with `2003`
- queue remains local for rollback/retry policy

### A.7 Duplicate Suppression

Input:
- same `(from_did, msg_id, recipient_did)` handed to relay twice within TTL window

Expected:
- second request does not create a second active entry
- relay returns idempotent acceptance/duplicate-suppressed result

### A.8 Transfer Rollback on Timeout

Input:
- upstream enters `TRANSFER_PENDING`
- downstream does not provide valid transfer receipt before timeout

Expected:
- transfer transitions to `ROLLED_BACK`
- upstream resumes local delivery attempts

### A.9 Transfer Acceptance in Single-Custody Mode

Input:
- valid authenticated transfer receipt received from downstream relay

Expected:
- transfer state becomes `accepted`
- upstream removes local copy only after acceptance

### A.10 Invalid Transfer Receipt Rejection

Input:
- transfer receipt signature invalid, or payload tuple mismatch (`msg_id` / `from_did` / `recipient_did` / `upstream_relay`)

Expected:
- receipt rejected with auth failure (`3001`)
- transfer remains `pending` then `rolled_back` by timeout/policy

### A.11 Unsupported Transfer Receipt Algorithm

Input:
- transfer receipt is structurally valid and signed, but COSE protected header `alg` is unsupported by receiver profile

Expected:
- receipt rejected with `3001`
- no custody transition to `accepted`

### A.12 Dual-Custody Commit Receipt Positive

Input:
- transfer accepted in `dual` mode
- downstream later sends valid `commit-receipt` with `result=delivered`

Expected:
- upstream verifies report and transitions `DUAL_ACTIVE -> COMMIT_REPORTED`
- upstream finalizes local record after audit persistence

### A.13 Multi-Recipient Federation Split

Input:
- original AMP message has recipients `[A, B]`
- federation transfer requested

Expected:
- upstream emits two handoff operations, one for `recipient_did=A`, one for `recipient_did=B`
- per-recipient transfer/commit states evolve independently

---

## Appendix B. Open Questions

1. Should batch webhook delivery be standardized here or kept in RFC 002/003 extension?
