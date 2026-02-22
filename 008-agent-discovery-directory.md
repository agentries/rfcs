# RFC 008: Agent Discovery & Directory

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-06
**Updated**: 2026-02-22
**Version**: 0.4

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (endpoint selection and principal binding)
- RFC 003: Relay & Store-and-Forward (relay federation delivery semantics)
- RFC 004: Capability Schema Registry & Compatibility (capability identifier format)
- RFC 005: Delegation Credentials & Authorization (delegated invocation authorization)
- RFC 006: Session Protocol (session-scoped messaging semantics)
- RFC 009: Reputation & Trust Signals (trust/ranking signal semantics)

---

## Abstract

This RFC defines interoperable discovery, contact gating, and presence semantics for AMP agents. It standardizes how agents publish and resolve reachable endpoints, enforce contact approval for discoverable agents, advertise relay federation capabilities, and exchange presence capacity signals. The goal is deterministic peer discovery without coupling AMP to one directory topology.

---

## Table of Contents

1. Scope and Non-Goals
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Discovery Data Model
4.1 Visibility and Contactability
4.2 Service Types and DID Declaration
4.3 Directory Record Model
4.4 Relay Federation Capability Descriptor
5. Discovery Semantics
5.1 Resolution Sources and Precedence
5.2 Endpoint Selection Algorithm (Deterministic)
5.3 Contact-Gated Routing Rule
5.4 Freshness and Expiry
6. Contact Workflow Semantics
6.1 CONTACT_REQUEST
6.2 CONTACT_RESPONSE
6.3 CONTACT_REVOKE
6.4 Contact Relationship State Machine
6.5 Contact Message CDDL
7. Presence Semantics
7.1 Presence Signal Model
7.2 Presence Query and Push
7.3 Presence Subscription Profile (Optional Extension)
7.4 Presence Message CDDL
8. Error Handling and Retry
9. Versioning and Compatibility
10. Security Considerations
11. Privacy Considerations
12. Implementation Checklist
13. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions

---

## 1. Scope and Non-Goals

This RFC defines:
- Discovery metadata for AMP endpoint publication and selection.
- Contact gating semantics for discoverable agents.
- Presence message semantics and expiry handling.
- Relay federation capability advertisement used by RFC 003 routing.
- Deterministic error mapping for discovery/contact/presence failures.

This RFC does not define:
- AMP envelope, signature, encryption, or type-code allocation (RFC 001).
- Transport framing and protocol negotiation details (RFC 002).
- Relay queue retention, custody transfer, and commit semantics (RFC 003).
- Capability schema negotiation and invocation validation semantics (RFC 004).
- Reputation scoring or ranking models (RFC 009).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 message envelope/signature semantics.
- Applies deterministic discovery and contact rules in Sections 5 and 6.
- Applies deterministic error mapping in Section 8.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Visibility | Agent contactability policy: `PRIVATE`, `DISCOVERABLE`, or `OPEN`. |
| Discovery record | Resolved metadata for one target DID (visibility, endpoints, freshness). |
| Contact relationship | Policy state between requester DID and target DID for gated communication. |
| Contact-gated endpoint | Endpoint that requires prior approval before non-contact traffic. |
| Presence signal | Advisory runtime capacity/performance metadata published by an agent. |
| Relay federation capability | Relay metadata (`transferModes`, `maxHopLimit`, `receiptAlgs`) used by senders/relays before forwarding. |

### 2.2 Role Profiles and MTI Requirements

`Discovery Client Profile`:
- MUST resolve target DID service metadata before first send attempt.
- MUST enforce deterministic endpoint selection in Section 5.2.
- MUST enforce contact-gated routing rule in Section 5.3.
- MUST enforce contact/presence message correlation rules in Sections 6 and 7.

`Contact-Gated Agent Profile`:
- MUST enforce visibility policy in Section 4.1.
- MUST process `CONTACT_REQUEST` / `CONTACT_RESPONSE` / `CONTACT_REVOKE` as defined in Section 6.
- MUST return `3002 CONTACT_REQUIRED` for unauthorized non-contact traffic under `DISCOVERABLE`.

`Presence Publisher Profile`:
- MUST publish `PRESENCE` payloads conforming to Section 7.4 when presence is enabled.
- MUST include expiry metadata and treat expired presence as stale.

`Presence Subscription Profile` (optional extension):
- MAY support `PRESENCE_SUB` / `PRESENCE_UNSUB`.
- MUST enforce subscription TTL and authorization policy if implemented.

`Directory Index Profile` (optional extension):
- MAY provide searchable index records.
- MUST treat DID Document resolution as authoritative on conflict.
- MUST expose freshness metadata (`updated_at`, `expires_at`) per record.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- RFC 001 defines type codes for `CONTACT_*` (`0x06-0x08`) and `PRESENCE_*` (`0x60-0x63`).
- This RFC defines body semantics, state rules, and error mapping for those types.
- Unsupported discovery semantics MAY be rejected with `1005 UNKNOWN_TYPE` per RFC 001 behavior.

With RFC 002:
- Transport binding and endpoint URL priority are defined in RFC 002.
- This RFC defines service eligibility and contact gating; RFC 002 applies binding priority on eligible endpoints.
- Principal/from DID binding remains governed by RFC 002.

With RFC 003:
- Relay delivery/commit semantics remain in RFC 003.
- `relayCapabilities` declared here are consumed by RFC 003 federation routing decisions.

With RFC 004:
- Capability identifiers referenced in discovery filters MUST follow RFC 004 naming/version rules.
- Discovery capability hints MUST NOT bypass RFC 004 validation at invocation time.

With RFC 005:
- Delegation artifacts MUST NOT bypass `DISCOVERABLE` contact policy unless target policy explicitly allows it.
- Delegated execution authorization remains governed by RFC 005.

With RFC 006:
- Session semantics remain independent of discovery.
- Presence and contact messages MUST NOT be treated as session-control operations.

---

## 4. Discovery Data Model

### 4.1 Visibility and Contactability

Visibility levels:

| Visibility | Listed in index | Contactability |
|------------|------------------|----------------|
| `PRIVATE` | No | No inbound AMP traffic except local/private policy |
| `DISCOVERABLE` | Yes | Requires approved contact relationship |
| `OPEN` | Yes | Direct inbound AMP traffic allowed by policy |

Normative rules:
- `PRIVATE` agents MUST NOT publish public AMP service endpoints.
- `DISCOVERABLE` agents MUST require contact approval before accepting non-contact message traffic.
- `OPEN` agents MAY still apply local abuse/rate policies, but MUST NOT require prior contact for normal traffic.

### 4.2 Service Types and DID Declaration

Supported DID service `type` values:
- `AgentMessaging`: direct AMP endpoint.
- `AgentMessagingGated`: contact-gated AMP endpoint.
- `AgentMessagingRelay`: relay endpoint.

Service interpretation rules:
- `AgentMessagingGated` implies `DISCOVERABLE` policy enforcement.
- `AgentMessagingRelay` MAY include `relayCapabilities` (Section 4.4).
- If multiple service entries exist, each entry is an independent candidate for Section 5.2 selection.

### 4.3 Directory Record Model

Directory index record (if an index is used):

```cddl
unix-ms = uint

discovery-record = {
  "disc_v": 1,
  "did": tstr,
  "visibility": "PRIVATE" / "DISCOVERABLE" / "OPEN",
  "endpoints": [* discovery-endpoint],
  ? "capabilities": [* tstr],   ; RFC 004 capability IDs or names
  "updated_at": unix-ms,
  "expires_at": unix-ms
}

discovery-endpoint = {
  "service_id": tstr,
  "type": "AgentMessaging" / "AgentMessagingGated" / "AgentMessagingRelay",
  "url": tstr,
  ? "relayCapabilities": relay-capabilities
}
```

Rules:
- DID Document data is authoritative if index record conflicts.
- Index records MUST NOT be used to route to endpoint URLs absent from DID services.
- `expires_at <= now` means stale record and MUST NOT be used for first-choice routing.
- Unknown optional fields MAY be ignored.

### 4.4 Relay Federation Capability Descriptor

```cddl
relay-capabilities = {
  "federation": bool,
  ? "relayForwardEndpoint": tstr,
  ? "transferModes": [1* ("single" / "dual")],
  ? "maxHopLimit": uint,
  ? "defaultHopLimit": uint,
  ? "receiptAlgs": [1* int]
}
```

Rules:
- If `federation=true`, sender metadata MUST include `relayForwardEndpoint`, `transferModes`, `maxHopLimit`, and `receiptAlgs`.
- `defaultHopLimit` is optional; if omitted, RFC 003 default applies.
- `defaultHopLimit` MUST be `<= maxHopLimit` when both exist.
- `receiptAlgs` MUST include `-8` (COSE EdDSA/Ed25519 MTI alignment with RFC 003).
- If `relayCapabilities` is absent or `federation=false`, relay-to-relay forwarding MUST NOT be assumed.

---

## 5. Discovery Semantics

### 5.1 Resolution Sources and Precedence

Resolution precedence:
1. Resolve DID Document service entries.
2. Optionally load index record (if configured).
3. If conflict exists, DID Document MUST win.
4. Index metadata MUST NOT introduce routable endpoint URLs absent from DID service entries.

Failure handling:
- DID not found or no AMP service entry -> `2001 RECIPIENT_NOT_FOUND`.
- DID resolves but no currently reachable endpoint -> `2002 ENDPOINT_UNREACHABLE`.

### 5.2 Endpoint Selection Algorithm (Deterministic)

Given `(recipient_did, sender_did, message_type, requires_capability?)`:

1. Build candidate endpoint set from DID services and preserve DID Document service-array index as `did_order`.
2. Optionally load index metadata for freshness/capability hints only; index metadata MUST NOT add endpoint URLs.
3. Remove stale candidates by local freshness policy.
4. Apply contact eligibility (Section 5.3).
5. Apply capability hint filter if configured (informative only).
6. For each candidate URL, compute transport priority per RFC 002 (`amps` > `wss` > `https`).
7. Sort candidates by tuple:
   - transport priority;
   - `did_order` ascending (RFC 002 same-binding rule);
   - `service_id` lexical ascending;
   - endpoint `url` lexical ascending.
8. Select first eligible candidate after sort.

If no eligible candidate remains:
- Return `3002` when blocked only by contact-gating policy.
- Else return `2001` or `2002` by local evidence.

### 5.3 Contact-Gated Routing Rule

For target visibility `DISCOVERABLE`:
- Allowed pre-approval message types: `CONTACT_REQUEST`, `CONTACT_RESPONSE`, `CONTACT_REVOKE`, and protocol error/handshake traffic required to complete these exchanges.
- Any other inbound application message MUST be rejected with `3002 CONTACT_REQUIRED` unless relation state is `ACTIVE`.

For target visibility `OPEN`:
- `CONTACT_*` flow MAY still be supported for optional relationship metadata, but MUST NOT be required.

For target visibility `PRIVATE`:
- Public inbound traffic SHOULD be rejected with `2001` or policy-equivalent deny without leakage.

### 5.4 Freshness and Expiry

- `updated_at` and `expires_at` are advisory freshness markers for routing.
- Expired records MAY be used only as last-resort hints and SHOULD trigger immediate refresh.
- Presence records past `expires_at` MUST be treated as unknown.

---

## 6. Contact Workflow Semantics

Direction/correlation baseline:
- `CONTACT_REQUEST` and `CONTACT_REVOKE` MUST be initiator messages and MUST NOT carry `reply_to`.
- `CONTACT_RESPONSE` MUST carry `reply_to` referencing the triggering `CONTACT_REQUEST.id`.
- A `CONTACT_RESPONSE` without valid `reply_to` correlation MUST be rejected with `4001`.

### 6.1 CONTACT_REQUEST

Type code: `0x06`.

Rules:
- Sender MUST provide non-empty `reason`.
- Request MAY include capability hints and expiry.
- Request target MUST be single-recipient.
- If `expires_at` is present and `expires_at <= now`, receiver MUST reject with `4001 BAD_REQUEST`.
- If `expires_at` is absent, receiver MAY apply local default request lifetime and treat that as effective expiry.

### 6.2 CONTACT_RESPONSE

Type code: `0x07`.

Rules:
- `reply_to` MUST reference the triggering `CONTACT_REQUEST.id`.
- Sender MUST be the contact-policy owner (target agent).
- `status` MUST be one of `approved`, `denied`, `pending`.
- `pending` keeps relation state in `PENDING` and SHOULD include reason when human/policy review is deferred.
- `approved` transitions relation to `ACTIVE`.
- For one request ID, repeated identical `pending` responses SHOULD be treated as idempotent.

### 6.3 CONTACT_REVOKE

Type code: `0x08`.

Rules:
- MAY be issued by either relation participant.
- After successful revoke, relation state MUST transition to `NONE`.
- Subsequent non-contact traffic to a `DISCOVERABLE` target MUST be denied with `3002`.

### 6.4 Contact Relationship State Machine

```text
NONE
  -> (CONTACT_REQUEST) PENDING
PENDING
  -> (CONTACT_RESPONSE pending) PENDING
  -> (CONTACT_RESPONSE approved) ACTIVE
  -> (CONTACT_RESPONSE denied) DENIED
  -> (request expiry) EXPIRED
ACTIVE
  -> (CONTACT_REVOKE) NONE
DENIED
  -> (new CONTACT_REQUEST) PENDING
EXPIRED
  -> (new CONTACT_REQUEST) PENDING
```

### 6.5 Contact Message CDDL

```cddl
contact-request-body = {
  "disc_v": 1,
  "reason": tstr,
  ? "capabilities_offered": [* tstr],
  ? "capabilities_requested": [* tstr],
  ? "expires_at": unix-ms
}

contact-response-body = {
  "disc_v": 1,
  "status": "approved" / "denied" / "pending",
  ? "reason": tstr,
  ? "granted_until": unix-ms,
  ? "restrictions": any
}

contact-revoke-body = {
  "disc_v": 1,
  ? "reason": tstr
}
```

---

## 7. Presence Semantics

### 7.1 Presence Signal Model

Presence is advisory capacity metadata, not authorization data.

Rules:
- Consumers MUST NOT treat presence as permission to bypass auth/delegation/policy checks.
- `capacity.accepting_requests=false` is a load signal, not a cryptographic deny.

### 7.2 Presence Query and Push

Type codes:
- `PRESENCE` (`0x60`)
- `PRESENCE_QUERY` (`0x61`)

Rules:
- `PRESENCE_QUERY` is a request message and SHOULD target one recipient.
- A direct answer to `PRESENCE_QUERY` MUST be `PRESENCE` with `reply_to` referencing the query message ID.
- Unsolicited/push `PRESENCE` messages MUST NOT set `reply_to`.
- `PRESENCE` SHOULD include `expires_at`.
- `PRESENCE_QUERY` MAY include an optional capability filter.
- Query response MAY be direct `PRESENCE` or policy-denied error.

### 7.3 Presence Subscription Profile (Optional Extension)

Type codes:
- `PRESENCE_SUB` (`0x62`)
- `PRESENCE_UNSUB` (`0x63`)

Rules if implemented:
- Subscription TTL MUST be bounded.
- Publisher SHOULD enforce rate limits and authorization policy.
- Unsupported subscription profile MAY return `1005 UNKNOWN_TYPE`.

### 7.4 Presence Message CDDL

```cddl
presence-capacity = {
  "concurrent_max": uint,
  "concurrent_current": uint,
  "queue_depth": uint,
  "accepting_requests": bool
}

presence-performance = {
  ? "estimated_response_ms": uint,
  ? "p95_response_ms": uint
}

presence-body = {
  "pres_v": 1,
  "capacity": presence-capacity,
  ? "performance": presence-performance,
  "offline_until": null / unix-ms,
  "expires_at": unix-ms
}

presence-query-body = {
  "pres_v": 1,
  ? "capability": tstr
}

presence-sub-body = {
  "pres_v": 1,
  ? "capability": tstr,
  ? "ttl_ms": uint
}

presence-unsub-body = {
  "pres_v": 1,
  ? "capability": tstr
}
```

---

## 8. Error Handling and Retry

Deterministic mapping:

| Condition | Code | Retry |
|-----------|------|-------|
| Malformed discovery/contact/presence body | `1001` | No |
| Unsupported `disc_v` or `pres_v` | `1004` | No |
| Unsupported discovery message type/profile (`CONTACT_*`/`PRESENCE_*` not implemented) | `1005` | No |
| Recipient DID or AMP service not found | `2001` | Maybe |
| Endpoint not reachable | `2002` | Yes |
| Contact approval required for gated target | `3002` | No (until policy/state changes) |
| Unauthorized actor for contact policy mutation | `3001` | No |
| Expired/invalid contact request window (`expires_at`) | `4001` | No |
| Invalid relay federation capability metadata (`relayCapabilities`) | `4001` | No |
| Invalid workflow/correlation state (`reply_to`, illegal transition, pending misuse) | `4001` | No |
| Directory or metadata source temporarily unavailable | `5002` | Yes |
| Internal discovery/policy engine failure | `5001` | Yes |

Retry guidance:
- `2002`, `5002`, `5001` MAY be retried with bounded backoff.
- `3002` SHOULD only be retried after successful contact approval.
- `100x/3001/4001` SHOULD NOT be retried without payload/policy changes.

---

## 9. Versioning and Compatibility

Version dimensions:
- AMP envelope version `v` is governed by RFC 001.
- Discovery/contact body version `disc_v` is fixed at `1` in this revision.
- Presence body version `pres_v` is fixed at `1` in this revision.

Compatibility rules:
- Receivers MUST reject unsupported `disc_v`/`pres_v` with `1004`.
- Unknown optional fields MAY be ignored unless security-sensitive.
- Backward-compatible additions MUST be optional fields.

---

## 10. Security Considerations

- Discovery metadata from index services is advisory; DID Document resolution is authoritative.
- Contact workflow decisions MUST rely on signed AMP messages and validated sender identity.
- Implementations MUST apply RFC 002 principal/from binding for contact and presence traffic.
- Contact endpoints SHOULD rate-limit request spam and enforce abuse controls.
- Presence metadata can be spoofed if signature validation is skipped; recipients MUST validate signatures per RFC 001.
- Relay federation capability claims SHOULD be validated against trusted DID methods before use.

---

## 11. Privacy Considerations

- Visibility defaults SHOULD be conservative (`PRIVATE` or `DISCOVERABLE`) to reduce unsolicited profiling.
- Presence signals may leak workload patterns; publishers SHOULD coarsen or quantize fields for public audiences.
- Directory index services SHOULD minimize retained metadata and publish retention policy.
- Contact deny responses SHOULD avoid leaking policy internals beyond required protocol signals.

---

## 12. Implementation Checklist

- Implement visibility and contactability policy (Section 4.1).
- Parse DID service types and relay capability metadata (Sections 4.2, 4.4).
- Implement deterministic endpoint selection and contact-gated routing (Section 5).
- Implement `CONTACT_*` workflow and state transitions (Section 6).
- Implement `PRESENCE` schema and expiry handling (Section 7).
- Apply deterministic error mapping (Section 8).
- Enforce version checks for `disc_v` and `pres_v` (Section 9).
- Add Appendix A vectors to conformance test plan.

---

## 13. References

### 13.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 002: Transport Bindings
- RFC 003: Relay & Store-and-Forward
- RFC 004: Capability Schema Registry & Compatibility
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 8610: CDDL

### 13.2 Informative References

- RFC 005: Delegation Credentials & Authorization
- RFC 006: Session Protocol
- RFC 009: Reputation & Trust Signals
- DID Core (W3C Recommendation)

---

## Appendix A. Minimal Test Vectors

### A.1 OPEN Discovery Positive

Input:
- Recipient publishes `AgentMessaging` endpoint with visibility `OPEN`.

Expected:
- Sender selects direct endpoint via Section 5.2.

### A.2 DISCOVERABLE Contact Required Negative

Input:
- Recipient visibility `DISCOVERABLE`.
- No active contact relationship.
- Sender sends non-contact message.

Expected:
- Reject with `3002 CONTACT_REQUIRED`.

### A.3 CONTACT_REQUEST to CONTACT_RESPONSE Approved Positive

Input:
- Valid `CONTACT_REQUEST`, then target replies `CONTACT_RESPONSE(status="approved")`.

Expected:
- Relationship transitions `NONE -> PENDING -> ACTIVE`.

### A.4 CONTACT_RESPONSE Correlation Negative

Input:
- `CONTACT_RESPONSE.reply_to` missing or points to non-contact request.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.5 CONTACT_REVOKE Positive

Input:
- Active relationship, then `CONTACT_REVOKE`.

Expected:
- Relationship transitions to `NONE`.
- Subsequent non-contact traffic to `DISCOVERABLE` target fails with `3002`.

### A.6 PRIVATE Visibility Routing Negative

Input:
- Target is `PRIVATE` with no public AMP service.

Expected:
- Discovery fails with `2001` or policy-equivalent non-leaking denial.

### A.7 Relay Federation Capability Positive

Input:
- `AgentMessagingRelay` contains `relayCapabilities` with `federation=true`, required fields, and `receiptAlgs` includes `-8`.

Expected:
- Candidate is federation-eligible for RFC 003 routing.

### A.8 Relay Federation Capability Invalid Negative

Input:
- `federation=true` but missing `relayForwardEndpoint` or `receiptAlgs` omits `-8`.

Expected:
- Candidate rejected as invalid metadata (`4001 BAD_REQUEST`).

### A.9 Presence Publish/Query Positive

Input:
- Valid `PRESENCE` (`pres_v=1`) and valid `PRESENCE_QUERY`.

Expected:
- Presence accepted and query returns current non-expired signal.

### A.10 Presence Expiry Handling

Input:
- Presence with `expires_at < now`.

Expected:
- Consumer treats presence as stale/unknown and excludes it from first-choice routing.

### A.11 Unsupported Body Version Negative

Input:
- `CONTACT_REQUEST` with `disc_v=2` or `PRESENCE` with `pres_v=2`.

Expected:
- Reject with `1004 UNSUPPORTED_VERSION`.

### A.12 Presence Subscription Unsupported Profile

Input:
- Receiver does not implement subscription profile; sender sends `PRESENCE_SUB`.

Expected:
- If subscription message types are not implemented, reject with `1005 UNKNOWN_TYPE`.
- If type is recognized but disabled by local policy/profile, reject with `4001 BAD_REQUEST`.

### A.13 Presence Query Correlation Negative

Input:
- `PRESENCE_QUERY` sent.
- Responder sends `PRESENCE` without `reply_to` (or with mismatched `reply_to`) as direct answer.

Expected:
- Reject with `4001 BAD_REQUEST` at query-response handler boundary.

### A.14 Contact Pending Semantics Positive

Input:
- Valid `CONTACT_REQUEST`.
- Target replies `CONTACT_RESPONSE(status="pending")` twice with same semantics.

Expected:
- Relationship remains `PENDING`.
- Repeated pending response is treated idempotently.

### A.15 Expired CONTACT_REQUEST Negative

Input:
- `CONTACT_REQUEST.expires_at <= now`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.16 Byte-Level Error Code Checks

Input:
- Contact-gated denial case mapped to `3002`.
- Correlation/expired-window failure case mapped to `4001`.
- Unsupported discovery/presence body version case mapped to `1004`.

Expected:
- `3002` CBOR uint encoding bytes: `19 0b ba`.
- `4001` CBOR uint encoding bytes: `19 0f a1`.
- `1004` CBOR uint encoding bytes: `19 03 ec`.

---

## Appendix B. Open Questions

No open questions in this revision.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-06 | Proposal | Nowa | Initial outline for discovery and directory concepts |
| 2026-02-07 | 0.1 | Nowa | Rewrote RFC 008 into normative draft format with conformance profiles, cross-RFC boundary contracts, deterministic discovery/contact/presence semantics, CDDL, error mapping, and minimal test vectors |
| 2026-02-07 | 0.2 | Nowa | Fixed deterministic endpoint tie-break ordering, formalized contact/presence direction and correlation rules, defined pending/expiry semantics, and added byte-level error-code checks |
| 2026-02-07 | 0.3 | Nowa | Aligned endpoint selection priority with RFC 002 (`transport` first), prohibited index-only endpoint injection, tightened `1005` usage boundaries, and made invalid federation metadata mapping deterministic (`4001`) |
| 2026-02-07 | 0.4 | Nowa | Added explicit Section 8 error mapping for invalid `relayCapabilities` metadata and synchronized optional conformance vector coverage for RFC 008 |
| 2026-02-10 | 0.4 | Nowa | Synchronized document metadata (`Updated`) and repository gate-status references for audit consistency |
