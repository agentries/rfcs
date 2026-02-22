# RFC 011: Multi-Agent Coordination & Group Messaging

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-06
**Updated**: 2026-02-22
**Version**: 0.6

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (principal binding and endpoint auth policy)
- RFC 003: Relay & Store-and-Forward (at-least-once fanout behavior)
- RFC 004: Capability Schema Registry & Compatibility (optional capability publication)
- RFC 005: Delegation Credentials & Authorization (delegated coordination execution policy)
- RFC 006: Session Protocol (optional session-scoped coordination context)
- RFC 007: Agent Payment Protocol (group task settlement signals)
- RFC 008: Agent Discovery & Directory (coordinator endpoint discovery)
- RFC 009: Reputation & Trust Signals (moderation and trust feedback inputs)
- RFC 010: Observability & Evaluation Telemetry (coordination telemetry inputs)

---

## Abstract

This RFC defines interoperable multi-agent coordination and group messaging semantics for AMP agents. It standardizes deterministic group membership management, role-based authorization, group message fanout, event querying, and replay/idempotency behavior. The goal is portable collaboration workflows without introducing new AMP type codes or transport coupling.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Coordination Data Model
4.1 Group and Membership Model
4.2 Role and Permission Model
4.3 Delivery and Ordering Model
4.4 Coordination CDDL
5. Coordination Protocol Semantics
5.1 Message Type Usage and Dispatch
5.2 GROUP_CREATE
5.3 MEMBER_ADD, MEMBER_REMOVE, MEMBER_ROLE_SET
5.4 GROUP_UPDATE
5.5 GROUP_SEND and GROUP_DELIVER
5.6 GROUP_STATE_GET and GROUP_EVENTS_QUERY
5.7 Coordination Body CDDL
5.8 CAP-Exposed Coordination Profile Mapping
6. State Machines
7. Error Handling and Retry
8. Versioning and Compatibility
9. Security Considerations
10. Privacy Considerations
11. Implementation Checklist
12. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions
Changelog

---

## 1. Problem Statement and Scope

Agent ecosystems require standardized multi-party collaboration semantics. Without shared group state and fanout rules, each implementation creates incompatible behavior for membership, role control, ordering, and retries.

This RFC defines:
- Signed coordination operation semantics for group lifecycle and membership changes.
- Deterministic role authorization for mutation, send, and read paths.
- Deterministic fanout behavior for group messages under at-least-once delivery.
- Deterministic event ordering and query semantics for audit/recovery.
- Deterministic error mapping for coordination operations.

This RFC does not define:
- AMP envelope/signature/encryption primitives (RFC 001).
- Transport handshake/framing rules (RFC 002).
- Relay internals and custody transfer mechanics (RFC 003).
- Capability schema authoring/compatibility rules (RFC 004).
- Delegation credential format and trust chain evaluation (RFC 005).
- Economic settlement internals (RFC 007).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope/signature semantics for all coordination operations.
- Enforces deterministic membership, role, and fanout rules in Section 5.
- Enforces deterministic state transitions in Section 6.
- Applies deterministic error mapping in Section 7.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Group | Coordination scope identified by one `group_id`. |
| Member | DID currently authorized in a group membership set. |
| Member revision (`member_rev`) | Monotonic revision incremented on membership/role/policy changes. |
| Group sequence (`group_seq`) | Monotonic sequence assigned to each committed group event. |
| Coordinator | Service that validates operations and commits group/event state. |
| Group message | One logical send operation (`group_send`) that fanouts into per-member deliveries. |
| Delivery event | One per-recipient delivery status tracked by coordinator (`queued`, `delivered`, `failed`). |
| Operation actor DID (`actor_did`) | DID from signed AMP envelope `from` field; authorization and idempotency rules bind to this value. |
| Thread reference (`thread_ref`) | Optional conversation key copied to delivered AMP `thread_id`. |

### 2.2 Role Profiles and MTI Requirements

`Coordination Client Profile`:
- MUST support `group_create`, `group_send`, `group_state_get`, and `group_events_query`.
- MUST include stable `coord_msg_id` for each `group_send`.
- MUST handle duplicate-idempotent success and duplicate-conflict rejection.

`Coordinator Profile`:
- MUST enforce role checks in Section 4.2 and Section 5.
- MUST maintain monotonic `member_rev` and `group_seq`.
- MUST treat `group_send` commit as source-of-truth even when transport push is delayed.
- MUST expose deterministic query ordering and pagination behavior.
- Core MTI baseline is strict actor binding: authenticated caller DID MUST equal operation actor DID (`actor_did`, derived from signed envelope `from`).

`Member Delivery Profile`:
- MUST accept `group_deliver` payloads as coordinator-origin messages.
- MUST deduplicate by (`group_id`, `group_seq`) and/or RFC 001 message idempotency key.
- SHOULD preserve event provenance (`sender`, `coord_msg_id`, `member_rev`) for audit.

`Federated Coordinator Profile` (optional extension):
- MAY support multi-coordinator federation over RFC 003 relay paths.
- MUST preserve `group_seq` monotonicity per coordinator domain.
- MUST expose coordinator domain in event metadata when federation is enabled.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- This RFC uses existing AMP types and does not allocate new type codes.
- Control/query operations use `REQUEST`/`RESPONSE`.
- Group fanout deliveries use `MESSAGE` with coordination body marker (`coord_v`).
- `group_id` is not a replacement for AMP envelope `thread_id`.

With RFC 002:
- Actor identity authorization MUST respect transport principal binding policy.
- Unauthorized principal/from combinations for coordination operations MUST map to `3001`.

With RFC 003:
- At-least-once delivery can create duplicates; coordinator and recipients MUST deduplicate deterministically.
- Relay behavior MUST NOT modify coordination body bytes.

With RFC 004:
- Coordination endpoints MAY be exposed as capabilities.
- If carried via `CAP_INVOKE`, capability negotiation and schema checks follow RFC 004.
- CAP interoperability baseline capability ID for this RFC is `xyz.agentries.coordination.workflow:1.0.0` (Section 5.8).

With RFC 005:
- Delegation semantics remain governed by RFC 005.
- This RFC message set does not define non-CAP delegation carriage; delegated execution (if used) follows RFC 001 Section 4.6 and RFC 005 via `CAP_INVOKE.body.delegation`.

With RFC 006:
- Coordination operations MAY be session-scoped.
- Session context validation remains governed by RFC 006.
- `thread_ref` in this RFC is application-level and MUST NOT bypass RFC 006 rules.

With RFC 007:
- Group workflows MAY emit payment-triggering outcomes.
- Payment authorization and settlement validity MUST remain governed by RFC 007.

With RFC 008:
- Discovery MAY publish coordinator endpoint hints.
- Discovery metadata MUST NOT override signed coordination semantics.

With RFC 009:
- Reputation signals MAY be generated from moderation/member behavior events.
- Reputation actions MUST NOT mutate group state without explicit coordination authorization.

With RFC 010:
- Telemetry MAY ingest coordination outcomes and delivery latencies.
- Telemetry records MUST NOT act as coordination control inputs by themselves.

---

## 4. Coordination Data Model

### 4.1 Group and Membership Model

MTI rules:
- A group is identified by `group_id` (`bstr .size 16`).
- `member_rev` starts at `1` on create and increments by exactly `1` for each membership/role/policy mutation.
- `group_seq` starts at `0` on create and increments by exactly `1` for each committed group event.
- Membership statuses are `active` or `removed`.
- `join_policy` MTI value is `invite_only`; `open` is optional profile.

### 4.2 Role and Permission Model

Roles:
- `owner`: full control, including policy/role mutation and owner transfer.
- `admin`: membership and role mutation except owner transfer/removal of last owner.
- `member`: can send group messages and query group state/events.
- `observer`: read-only; cannot send or mutate.

Authorization MTI:
- `group_create`: any authenticated caller MAY create; caller becomes `owner`.
- `member_add`, `member_remove`, `member_role_set`, `group_update`: caller MUST be `owner` or `admin`.
- Owner-role grant/revoke via `member_role_set` MUST be owner-only.
- `group_send`: caller MUST be `owner`, `admin`, or `member` and membership status MUST be `active`.
- `group_state_get`, `group_events_query`: caller MUST be active member unless local policy explicitly permits broader read.

### 4.3 Delivery and Ordering Model

Ordering rules:
- Coordinator MUST assign one strictly increasing `group_seq` to every committed group event.
- `group_events_query` default ordering MUST be ascending by `group_seq`.
- Same (`actor_did`, `group_id`, `coord_msg_id`) with identical payload MUST be idempotent success.
- Same key with conflicting payload MUST fail with `4001`.

Fanout rules:
- `group_send` commit produces one logical event and zero or more delivery records.
- Commit success is determined by coordinator state persistence, not immediate endpoint reachability.
- Delivery status transitions are `queued -> delivered` or `queued -> failed`.
- Failed per-recipient push MUST NOT roll back committed `group_seq`.

`thread_ref` rule:
- If `group_send.thread_ref` is present, coordinator MUST copy bytes exactly to delivered `MESSAGE.thread_id`.
- If absent, coordinator MUST NOT synthesize `thread_id` for `group_deliver`.

### 4.4 Coordination CDDL

```cddl
did = tstr
unix-ms = uint
group-id = bstr .size 16
coord-msg-id = bstr .size 16
event-id = bstr .size 16
session-id = bstr .size 16

member-role = "owner" / "admin" / "member" / "observer"
member-status = "active" / "removed"
join-policy = "invite_only" / "open"
delivery-status = "queued" / "delivered" / "failed"
coord-request-op =
  "group_create" /
  "member_add" /
  "member_remove" /
  "member_role_set" /
  "group_update" /
  "group_send" /
  "group_state_get" /
  "group_events_query"

coord-response-op = coord-request-op

coord-session-context = {
  "session_id": session-id,
  "session_scope": true
}

coord-member = {
  "did": did,
  "role": member-role,
  "status": member-status,
  "updated_at": unix-ms
}

coord-group = {
  "group_id": group-id,
  "owner": did,
  "join_policy": join-policy,
  "member_rev": uint,
  "group_seq": uint,
  "created_at": unix-ms,
  "updated_at": unix-ms,
  ? "meta": { * tstr => tstr },
  "members": [* coord-member]
}

coord-delivery = {
  "recipient": did,
  "status": delivery-status,
  ? "error_code": uint
}

coord-event = {
  "group_seq": uint,
  "event_id": event-id,
  "op": tstr,
  "actor": did,
  "occurred_at": unix-ms,
  "member_rev": uint,
  ? "coord_msg_id": coord-msg-id,
  ? "thread_ref": bstr,
  ? "payload": any,
  ? "deliveries": [* coord-delivery]
}
```

---

## 5. Coordination Protocol Semantics

### 5.1 Message Type Usage and Dispatch

Dispatch rules:
- Direct coordination profile (MTI in this RFC): requests MUST use `typ = REQUEST`; responses MUST use `typ = RESPONSE` and MUST set envelope `reply_to` to triggering request `id`.
- Group delivery fanout uses `typ = MESSAGE` with `body.coord_v = 1` and `body.op = "group_deliver"`.
- CAP-exposed coordination profile (optional): outer dispatch follows RFC 004 (`CAP_INVOKE`/`CAP_RESULT`) while coordination semantics apply to capability payload.
- Operation actor DID source is signed envelope `from` (`actor_did`) in direct and CAP paths.
- Coordination request payload MUST NOT define its own actor override field.
- Direct profile failures mapped in Section 7 MUST be returned as AMP `ERROR`; direct `RESPONSE(status="error")` MUST NOT be used.

Version and operation rules:
- Supported version is `coord_v = 1`.
- Unsupported `coord_v` MUST be rejected with `1004`.
- Unknown `op` values MUST be rejected with `4001`.
- `body.delegation` on non-CAP coordination messages MUST be rejected with `4001`.
- `op = "group_deliver"` is coordinator fanout-only and MUST NOT appear in direct `REQUEST/RESPONSE` or `CAP_INVOKE.params`; violation MUST be rejected with `4001`.

### 5.2 GROUP_CREATE

Request body (`op = "group_create"`) MUST include:
- `group_id`
- optional `join_policy` (default `invite_only`)
- optional `meta`
- optional `initial_members`

Processing:
- Authenticated caller DID becomes `owner` and MUST exist in resulting member list as `active`.
- `member_rev` initialized to `1`; `group_seq` initialized to `0`.
- If group already exists, behavior is idempotent only when caller DID and initial normalized state are equivalent; otherwise reject with `4001`.

Deterministic normalization for `group_create` idempotency:
- `join_policy` absent is normalized to `"invite_only"`.
- `meta` absent is normalized to an empty map.
- `initial_members` absent is normalized to an empty list.
- For each `initial_members` entry, absent `role` is normalized to `"member"`.
- `initial_members` MUST NOT contain duplicate member DIDs; duplicates MUST be rejected with `4001`.
- If `initial_members` contains caller DID, role MUST be `"owner"`; otherwise reject with `4001`.
- Effective member set is built from normalized `initial_members` plus caller as active `owner`, then sorted by DID lexical order for equivalence comparison.

Response:
- `status = "ok"`
- `group` snapshot with `member_rev` and current members.

### 5.3 MEMBER_ADD, MEMBER_REMOVE, MEMBER_ROLE_SET

Shared rules:
- Caller MUST be authorized role (`owner` or `admin`).
- Request MAY include `if_member_rev` for optimistic concurrency.
- If `if_member_rev` present and does not equal current `member_rev`, reject with `4001`.

`member_add`:
- If target DID not present, add with role and `active` status.
- If target exists as `removed`, re-activate and set requested role.
- If request role is absent, effective role defaults to `"member"`.

`member_remove`:
- Marks target as `removed`.
- Removing the last active `owner` MUST be rejected with `4001`.

`member_role_set`:
- Changes role for active member.
- Promoting/removing owner role MUST preserve at least one active owner.
- Owner-role grant/revoke (`role = "owner"` or demoting an existing owner) MUST be executed only by current owner; admin attempts MUST be rejected with `3001`.

Mutation response:
- `status = "ok"`
- `member_rev` incremented by exactly `1`.

### 5.4 GROUP_UPDATE

Purpose:
- Update group metadata (`meta`) and/or `join_policy`.

Rules:
- Caller MUST be `owner` or `admin`.
- Unknown policy values MUST be rejected with `4001`.
- `join_policy = "open"` is optional extension; if unsupported by implementation policy, reject with `3001`.
- Successful mutation MUST increment `member_rev` by exactly `1`.

### 5.5 GROUP_SEND and GROUP_DELIVER

`group_send` request MUST include:
- `group_id`
- `coord_msg_id` (`bstr .size 16`)
- `payload`
- optional `content_type`
- optional `thread_ref`
- optional `include_self` (default `false`)

Validation:
- Caller MUST be active member with send permission.
- `coord_msg_id` idempotency key is (`actor_did`, `group_id`, `coord_msg_id`).
- Same key + same normalized payload => idempotent success.
- Same key + conflicting payload => `4001`.

Processing:
1. Commit one `message_sent` event and assign next `group_seq`.
2. Build recipient set from active members at committed `member_rev`.
3. Apply `include_self` filter.
4. Create per-recipient delivery records with initial `queued` status.
5. Attempt transport push asynchronously/synchronously per local policy.
6. Update each delivery record to `delivered` or `failed` when outcome known.

Success response:
- `status = "accepted"`
- `group_seq`
- `member_rev`
- `recipient_count`

`group_deliver` fanout message body MUST include:
- `coord_v = 1`
- `op = "group_deliver"`
- `group_id`
- `group_seq`
- `member_rev`
- `sender`
- `coord_msg_id`
- `payload`
- optional `content_type`
- optional `thread_ref`

Fanout envelope rule:
- For each recipient, coordinator sends standard AMP `MESSAGE` with `from = coordinator_did` and `to = recipient_did`.
- `sender` inside body carries original group sender DID.

### 5.6 GROUP_STATE_GET and GROUP_EVENTS_QUERY

`group_state_get`:
- Returns current `group` snapshot.
- Caller MUST be active member (MTI).

`group_events_query` request fields:
- `group_id`
- optional `from_seq` (inclusive, default `0`)
- optional `cursor` (opaque)
- optional `limit` (`1..50`, default `20`)
- optional `include_payload` (default `false`)

Rules:
- Caller MUST be active member (MTI).
- Results MUST be deterministic ordering by `group_seq` ascending.
- If `include_payload = false`, `payload` field MUST be omitted in event items.
- `from_seq` and `cursor` MUST NOT be present together.
- If both `from_seq` and `cursor` are absent, receiver MUST treat query as `from_seq = 0`.
- If `cursor` is present, requester MUST keep `group_id` and `include_payload` unchanged from original page; `limit` MAY change.
- Cursor token MUST be bound to (`group_id`, `include_payload`) and include issuance time.
- Cursor validity window is `600000` ms from issuance; expired cursor MUST be rejected with `4001`.
- Invalid cursor/range/limit MUST be rejected with `4001`.

### 5.7 Coordination Body CDDL

```cddl
coord-request = {
  "coord_v": 1,
  "op": coord-request-op,
  ? "session": coord-session-context,
  ? "group_id": group-id,
  ? "coord_msg_id": coord-msg-id,
  ? "if_member_rev": uint,
  ? "join_policy": join-policy,
  ? "target": did,
  ? "role": member-role,
  ? "meta": { * tstr => tstr },
  ? "initial_members": [* { "did": did, ? "role": member-role }],
  ? "payload": any,
  ? "content_type": tstr,
  ? "thread_ref": bstr,
  ? "include_self": bool,
  ? "from_seq": uint,
  ? "cursor": tstr,
  ? "limit": uint,
  ? "include_payload": bool
}

coord-response = {
  "coord_v": 1,
  "op": coord-response-op,
  "status": "ok" / "accepted",
  ? "group": coord-group,
  ? "member_rev": uint,
  ? "group_seq": uint,
  ? "recipient_count": uint,
  ? "events": [* coord-event],
  ? "next_cursor": tstr / null,
  ? "has_more": bool
}

coord-deliver-body = {
  "coord_v": 1,
  "op": "group_deliver",
  "group_id": group-id,
  "group_seq": uint,
  "member_rev": uint,
  "sender": did,
  "coord_msg_id": coord-msg-id,
  "payload": any,
  ? "content_type": tstr,
  ? "thread_ref": bstr,
  ? "session": coord-session-context
}
```

### 5.8 CAP-Exposed Coordination Profile Mapping

Capability ID:
- `xyz.agentries.coordination.workflow:1.0.0`

Rules:
- `CAP_INVOKE.params` MUST contain exactly one RFC 011 request body with `coord_v = 1`.
- Allowed CAP request ops are: `group_create`, `member_add`, `member_remove`, `member_role_set`, `group_update`, `group_send`, `group_state_get`, `group_events_query`.
- `CAP_RESULT(status="success").result` MUST carry RFC 011 response body.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005.
- Invalid/unsupported delegation evidence MUST map to `3004`.
- Direct profile `REQUEST`/`RESPONSE` direction rules MUST NOT be applied to CAP envelope types.
- Session context source-of-truth in CAP path is RFC 004 envelope extension (`CAP_INVOKE.body.session`, `CAP_RESULT.body.session`) with semantics governed by RFC 006.
- `CAP_INVOKE.params.session` and `CAP_RESULT.result.session` MAY exist for payload-level compatibility, but if both payload and envelope session context are present, they MUST be semantically equivalent; mismatch MUST fail with `4001`.
- Pre-execution rejection in CAP path MUST return AMP `ERROR` per RFC 004 semantics.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.
- `CAP_RESULT(status="error")` MUST NOT be interpreted as RFC 011 `coord-response`.

---

## 6. State Machines

### 6.1 Member Lifecycle State Machine

```text
ABSENT
  -> (member_add / create include) ACTIVE
ACTIVE
  -> (member_role_set) ACTIVE
  -> (member_remove) REMOVED
REMOVED
  -> (member_add re-activate) ACTIVE
```

Deterministic constraints:
- `member_rev` increments once per successful mutation transition.
- Illegal transitions (for example remove non-member with strict policy) MUST map to `4001`.

### 6.2 Group Send and Fanout State Machine

```text
SEND_ACCEPTED
  -> (event committed, group_seq assigned) COMMITTED
COMMITTED
  -> (recipient record created) DELIVERY_QUEUED
DELIVERY_QUEUED
  -> (push success) DELIVERED
DELIVERY_QUEUED
  -> (push terminal failure) DELIVERY_FAILED
```

Deterministic constraints:
- Once `COMMITTED`, event MUST remain queryable.
- `DELIVERY_FAILED` for some recipients MUST NOT roll back `COMMITTED` event.

---

## 7. Error Handling and Retry

| Condition | Code | Retryable |
|-----------|------|-----------|
| Unauthorized actor, non-member send/query, or role violation | `3001` | No |
| Invalid/unsupported delegation evidence in CAP coordination path (`CAP_INVOKE.body.delegation`) | `3004` | No |
| `body.delegation` present on non-CAP coordination message | `4001` | No |
| `group_deliver` appears on non-fanout request path (`REQUEST`/`RESPONSE`/`CAP_INVOKE.params`) | `4001` | No |
| Invalid `op`/`typ` direction or `reply_to` correlation | `4001` | No |
| Invalid field ranges (`limit`, malformed `group_id`/`coord_msg_id`, bad policy) | `4001` | No |
| Duplicate idempotency key with conflicting payload | `4001` | No |
| `if_member_rev` mismatch or invalid mutation transition | `4001` | No |
| `group_events_query` carries both `from_seq` and `cursor` | `4001` | No |
| Invalid/expired query cursor or context mismatch | `4001` | No |
| Unsupported `coord_v` | `1004` | No |
| Invalid CBOR/message shape | `1001` | No |
| Coordinator/group store unavailable | `5002` | Yes |
| Internal processing fault | `5001` | Yes |

Retry guidance:
- `5002` and `5001` MAY be retried with bounded exponential backoff.
- `100x/3001/3004/4001` SHOULD NOT be retried without payload/policy changes.

---

## 8. Versioning and Compatibility

Versioning rules:
- This RFC defines `coord_v = 1`.
- Receivers MUST reject unsupported major coordination versions with `1004`.

Compatibility rules:
- New optional fields MAY be added while preserving existing required semantics.
- Required-field removal or semantic reinterpretation is a breaking change.
- `body.delegation` MUST NOT be treated as ignorable optional input on non-CAP coordination paths; delegation carriage/validation remains governed by RFC 001 Section 4.6 and RFC 005.
- Backward-compatible evolution MUST use optional-field extension only.

---

## 9. Security Considerations

Threat model includes:
- Unauthorized membership mutation.
- Coordinator impersonation in fanout path.
- Replay/duplicate injection for group send operations.
- Cross-group event confusion via malformed identifiers.

Required controls:
- Enforce strict actor DID binding from transport principal to signed `from` DID.
- Enforce role checks for all mutation/send operations.
- Treat `coord_msg_id` duplicates with conflicting payload as invalid (`4001`).
- Preserve immutable committed event log once `group_seq` assigned.
- Require recipients to verify envelope signature and coordinator trust before accepting `group_deliver` body.

---

## 10. Privacy Considerations

- Group membership and event history are sensitive collaboration metadata.
- MTI query profile is member-only visibility.
- Implementations SHOULD default `include_payload=false` for event query unless explicitly required.
- Retention SHOULD be bounded by policy with auditable deletion windows.
- Public/federated read profiles MUST be explicit opt-in extensions with redaction.

---

## 11. Implementation Checklist

- Implement `coord_v = 1` parsing and op dispatch.
- Enforce direct profile direction rules (`REQUEST/RESPONSE`) and `group_deliver` (`MESSAGE`).
- Enforce strict role checks and member-state transitions.
- Enforce idempotency key (`actor_did`, `group_id`, `coord_msg_id`) semantics.
- Implement monotonic `member_rev` and `group_seq` assignment.
- Implement deterministic event query ordering and pagination.
- Implement CAP mapping (`xyz.agentries.coordination.workflow:1.0.0`) and delegation error mapping (`3004`).
- Implement RFC 006 session-context validation for session-scoped operations.

---

## 12. References

### 12.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 002: Transport Bindings (TCP-first, HTTP/WS mappings)
- RFC 003: Relay & Store-and-Forward
- RFC 004: Capability Schema Registry & Compatibility
- RFC 005: Delegation Credentials & Authorization
- RFC 006: Session Protocol (State + Recovery)
- RFC 2119: Key words for use in RFCs to Indicate Requirement Levels
- RFC 8174: Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words
- RFC 8949: CBOR
- RFC 8610: CDDL

### 12.2 Informative References

- RFC 007: Agent Payment Protocol
- RFC 008: Agent Discovery & Directory
- RFC 009: Reputation & Trust Signals
- RFC 010: Observability & Evaluation Telemetry

---

## Appendix A. Minimal Test Vectors

### A.1 GROUP_CREATE Positive

Input:
- Valid direct `REQUEST` with `coord_v=1`, `op="group_create"`, and authenticated caller DID.

Expected:
- `RESPONSE(status="ok")` with `member_rev=1` and caller as active `owner`.

### A.2 MEMBER_ADD Positive

Input:
- Active `owner` sends `member_add` with valid target DID and role.

Expected:
- Success, target becomes `active`, `member_rev` increments by `1`.

### A.3 MEMBER_ADD Unauthorized Negative

Input:
- Caller with role `observer` attempts `member_add`.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.4 GROUP_SEND Positive

Input:
- Active `member` sends valid `group_send` with unique `coord_msg_id`.

Expected:
- Success with `status="accepted"`, assigned `group_seq`, and deterministic recipient count.

### A.5 GROUP_SEND Duplicate Idempotent Positive

Input:
- Retry same (`actor_did`, `group_id`, `coord_msg_id`) with identical normalized payload.

Expected:
- Deterministic idempotent success; same logical result.

### A.6 GROUP_SEND Duplicate Conflict Negative

Input:
- Same idempotency key but conflicting payload.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.7 GROUP_EVENTS_QUERY Non-Member Negative

Input:
- Caller DID is not active member.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.8 GROUP_DELIVER Thread Reference Copy Positive

Input:
- `group_send.thread_ref` present with fixed bytes.

Expected:
- Delivered `MESSAGE.thread_id` bytes equal `thread_ref` exactly.

### A.9 Unsupported coord_v Negative

Input:
- `coord_v = 2`.

Expected:
- Reject with `1004 UNSUPPORTED_VERSION`.

### A.10 Byte-Level Error Code Checks

Input:
- Unauthorized action mapped to `3001`.
- Delegation invalid mapped to `3004`.
- Invalid payload/state mapped to `4001`.
- Unsupported version mapped to `1004`.

Expected:
- `3001` CBOR uint encoding bytes: `19 0b b9`.
- `3004` CBOR uint encoding bytes: `19 0b bc`.
- `4001` CBOR uint encoding bytes: `19 0f a1`.
- `1004` CBOR uint encoding bytes: `19 03 ec`.

### A.11 CAP Coordination Profile Positive

Input:
- `CAP_INVOKE` targets `id = xyz.agentries.coordination.workflow:1.0.0`.
- `CAP_INVOKE.params` contains valid RFC 011 request body.

Expected:
- RFC 004 capability validation passes.
- Coordination semantics execute with `CAP_RESULT(status="success")`.

### A.12 CAP Delegation Evidence Invalid Negative

Input:
- CAP coordination invocation contains invalid `CAP_INVOKE.body.delegation`.

Expected:
- Reject with `3004 DELEGATION_INVALID`.

### A.13 Non-CAP Delegation Field Negative

Input:
- Direct coordination request (`REQUEST/RESPONSE` path) carries `body.delegation`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.14 Session Context Shape Invalid Negative

Input:
- Session-scoped coordination request carries malformed `body.session`.

Expected:
- Reject with `1001 INVALID_MESSAGE` per RFC 006 parsing rules.

### A.15 GROUP_DELIVER Wrong Direction Negative

Input:
- Direct `REQUEST` or `CAP_INVOKE.params` carries `op = "group_deliver"`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.16 MEMBER_ROLE_SET Owner Mutation by Admin Negative

Input:
- Active `admin` attempts owner-role grant/revoke through `member_role_set`.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.17 GROUP_EVENTS_QUERY Cursor and from_seq Conflict Negative

Input:
- `group_events_query` includes both `cursor` and `from_seq`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.18 Actor Binding Mismatch Negative

Input:
- Transport-authenticated principal DID differs from signed envelope `from` DID for a coordination mutation request.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.19 GROUP_CREATE Duplicate Member DID Negative

Input:
- `group_create.initial_members` contains duplicate DID entries.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.20 GROUP_EVENTS_QUERY Expired Cursor Negative

Input:
- `group_events_query.cursor` references issuance timestamp older than `600000` ms.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.21 MEMBER_ADD Default Role Positive

Input:
- `member_add` omits `role` for a new target DID.

Expected:
- Target is added as active `member` with deterministic default-role behavior.

### A.22 CAP_RESULT Error Channel Semantics

Input:
- Post-accept coordination failure is returned as `CAP_RESULT(status="error")`.

Expected:
- Response is treated as RFC 004 CAP error channel, not RFC 011 `coord-response`.

### A.23 Direct Profile Error Channel Semantics

Input:
- Direct coordination request fails validation (for example invalid non-fanout `op="group_deliver"` on `REQUEST` path).

Expected:
- Failure is returned as AMP `ERROR` with mapped code (for example `4001`), not `RESPONSE(status="error")`.

---

## Appendix B. Open Questions

No open questions in this revision.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-06 | Proposal | Nowa | Initial outline for multi-agent coordination and group messaging concepts |
| 2026-02-08 | 0.1 | Nowa | Rewrote RFC 011 into normative draft structure with conformance profiles, boundary contracts, deterministic coordination semantics, CDDL schemas, CAP interop mapping, error mapping, and minimal test vectors |
| 2026-02-08 | 0.2 | Nowa | Fixed `group_seq` naming consistency, restricted `group_deliver` to coordinator fanout path, clarified actor binding source (`envelope.from`) and idempotency key, constrained admin owner-role mutation rules, and completed `group_events_query` cursor model with CDDL/error/vector coverage |
| 2026-02-08 | 0.3 | Nowa | Made `group_events_query` default window deterministic (`from_seq=0`), aligned CAP error-channel behavior with RFC 004 (`ERROR` pre-exec, `CAP_RESULT(status=\"error\")` post-accept), normalized A.5 idempotency terminology to `actor_did`, and added CBOR/CDDL normative references |
| 2026-02-08 | 0.4 | Nowa | Fixed direct-path error channel to AMP `ERROR` only, defined deterministic `group_create` normalization/equivalence rules, standardized cursor binding+expiry (`600000` ms), tightened `coord-response` success-only schema, and added vectors A.19-A.20 |
| 2026-02-08 | 0.5 | Nowa | Resolved CAP success/error payload interpretation boundary, standardized default role behavior for `initial_members` and `member_add`, and added vectors A.21-A.22 |
| 2026-02-09 | 0.6 | Nowa | Synchronized conformance metadata to RFC 011 v0.6 and added explicit direct-profile error-channel vector A.23 (`ERROR` vs `RESPONSE(status=error)`) |
| 2026-02-10 | 0.6 | Nowa | Synchronized document metadata (`Updated`) and repository gate-status references for audit consistency |
