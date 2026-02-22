# RFC 009: Reputation & Trust Signals

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-06
**Updated**: 2026-02-22
**Version**: 0.11

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (principal binding and auth policy)
- RFC 003: Relay & Store-and-Forward (at-least-once delivery behavior)
- RFC 004: Capability Schema Registry & Compatibility (optional capability publication)
- RFC 005: Delegation Credentials & Authorization (delegated submission policy)
- RFC 006: Session Protocol (optional dispute workflow sessioning)
- RFC 007: Agent Payment Protocol (payment outcome-derived signals)
- RFC 008: Agent Discovery & Directory (reputation endpoint discovery)
- RFC 010: Observability & Evaluation Telemetry (informative telemetry source)

---

## Abstract

This RFC defines interoperable reputation signal and trust-score semantics for AMP agents. It standardizes how signed reputation events are submitted, queried, disputed, resolved, and aggregated into deterministic trust scores. The goal is portable trust evaluation without turning reputation into an authorization primitive.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Reputation Data Model
4.1 Signal Dimensions
4.2 Reputation Event CDDL
4.3 Evidence and Integrity
4.4 Event Lifecycle and Idempotency
5. Reputation Protocol Semantics
5.1 Message Type Usage and Dispatch
5.2 SIGNAL_SUBMIT
5.3 SIGNAL_QUERY
5.4 SCORE_QUERY
5.5 DISPUTE_OPEN and DISPUTE_RESOLVE
5.6 Deterministic Aggregation and Decay Algorithm
5.7 Reputation Body CDDL
5.8 CAP-Exposed Reputation Profile Mapping
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

Agent ecosystems need machine-consumable trust signals for unknown counterparties. Without a shared model, each implementation uses incompatible scoring and dispute rules, reducing interoperability and increasing abuse risk.

This RFC defines:
- Signed reputation event submission and query semantics.
- Deterministic trust score computation and decay behavior.
- DID-level and optional service-scoped trust score semantics.
- Dispute open/resolve lifecycle for incorrect or abusive signals.
- Deterministic error mapping for reputation operations.

This RFC does not define:
- Access control or authorization decisions (RFC 005 remains authoritative).
- Payment settlement proof semantics (RFC 007).
- Telemetry schema ingestion pipelines (RFC 010).
- Identity method governance beyond DID verification rules (RFC 001/002).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope/signature semantics for all reputation operations.
- Enforces dispatch, correlation, and algorithm rules in Sections 5 and 6.
- Applies deterministic error mapping in Section 7.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Reputation event | Signed assertion about observed behavior of a subject DID. |
| Subject | DID being evaluated by reputation signals. |
| Subject service | Optional service/capability scope under one subject DID. |
| Reporter | DID submitting a reputation event. |
| Submit attestation | Verifiable record that binds one stored reputation event to its original signed `signal_submit` AMP envelope bytes. |
| Confidence | Reporter-declared confidence (`0..100`) used by weighting rules. |
| Trust score | Deterministic aggregate integer score (`-100..100`) over eligible events. |
| Dispute | Signed challenge against one reputation event, opened by the subject DID. |
| Eligible event | Event that is active, unexpired, and not excluded by dispute state/policy. |

### 2.2 Role Profiles and MTI Requirements

`Signal Producer Profile`:
- MUST submit signed events with stable `event_id`.
- MUST provide valid `subject`, `reporter`, `category`, `weight`, `confidence`, and `observed_at`.
- MUST preserve idempotency on retries with same `event_id`.

`Reputation Aggregator Profile`:
- MUST enforce reporter identity binding and event validation.
- MUST implement deterministic aggregation in Section 5.6.
- MUST implement dispute lifecycle transitions in Section 6.
- MUST expose score/query responses with deterministic error mapping.
- Core MTI baseline is strict reporter binding: authenticated caller DID MUST equal `event.reporter`.

`Reputation Consumer Profile`:
- MUST treat trust score as advisory only.
- MUST NOT use trust score as a substitute for signature/auth/delegation checks.
- SHOULD retain score snapshot timestamp and event_count for auditability.

`Dispute Operator Profile` (optional extension):
- MAY support `dispute_resolve`.
- MUST enforce authorized operator policy for resolution actions.

`Public Query Profile` (optional extension):
- MAY allow `signal_query` from caller DID different from query `subject` when `signal_query.view="public"`.
- MUST return redacted event view (no `event.context`, no `event.evidence.inline`, and `attestation.submit_envelope = null`).
- MUST still preserve deterministic ordering, pagination, and error mapping rules.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- This RFC uses `REQUEST`/`RESPONSE` envelopes and does not allocate new type codes.
- Reputation operations are dispatched only when signed body includes `rep_v`.
- Envelope signature, `reply_to`, and idempotency semantics remain governed by RFC 001.

With RFC 002:
- Reporter identity authorization MUST respect transport principal binding policy.
- Unauthorized principal/from combinations for reputation operations MUST map to `3001`.

With RFC 003:
- At-least-once delivery can produce duplicates; aggregator MUST enforce idempotency by (`reporter`, `event_id`).
- Relay behavior MUST NOT modify reputation body bytes.

With RFC 004:
- Reputation endpoints MAY be exposed as capabilities.
- If carried via `CAP_INVOKE`, capability negotiation and schema checks follow RFC 004.
- CAP interoperability baseline capability ID for this RFC is `xyz.agentries.reputation.workflow:1.0.0` (Section 5.8).

With RFC 005:
- Delegation credential semantics remain governed by RFC 005.
- This RFC message set does not define non-CAP delegation carriage; delegated execution (if used) follows RFC 001 Section 4.6 and RFC 005 via `CAP_INVOKE.body.delegation`.

With RFC 006:
- Dispute workflows MAY be session-scoped.
- Session context validation remains governed by RFC 006.

With RFC 007:
- Payment outcomes MAY be source material for reputation events.
- Payment settlement proof validation MUST NOT be replaced by reputation score.

With RFC 008:
- Discovery MAY publish reputation endpoint hints.
- Discovery metadata MUST NOT override signed reputation event semantics.

---

## 4. Reputation Data Model

### 4.1 Signal Dimensions

Signal fields:
- `category`: one of `delivery`, `payment`, `quality`, `policy`, `abuse`.
- `subject_service`: optional service/capability identifier under `subject`.
- `weight`: signed integer impact (`-100..100`).
- `confidence`: reporter confidence (`0..100`).
- `observed_at`: event timestamp in epoch milliseconds.
- `expires_at`: optional event expiry in epoch milliseconds.

Normalization rules:
- `weight` outside `-100..100` MUST be rejected with `4001`.
- `confidence` outside `0..100` MUST be rejected with `4001`.
- If `expires_at` is present, `expires_at` MUST be greater than `observed_at`.
- `observed_at` MUST NOT be in the far future (`> server_now + 300000`), else reject with `4001`.

### 4.2 Reputation Event CDDL

```cddl
did = tstr
unix-ms = uint
event-id = bstr .size 16
dispute-id = bstr .size 16
service-id = tstr

rep-category = "delivery" / "payment" / "quality" / "policy" / "abuse"

evidence-ref = {
  ? "uri": tstr,
  ? "hash_alg": "sha-256" / "sha-512",
  ? "hash": bstr,
  ? "inline": bstr
}

submit-attestation = {
  "attest_v": 1,
  ? "submit_envelope": bstr / null,
  "hash_alg": "sha-256",
  "hash": bstr .size 32
}

reputation-event = {
  "event_id": event-id,
  "subject": did,
  ? "subject_service": service-id,
  "reporter": did,
  "category": rep-category,
  "weight": int,       ; MUST be in [-100, 100]
  "confidence": uint,  ; MUST be in [0, 100]
  "observed_at": unix-ms,
  ? "expires_at": unix-ms,
  ? "evidence": evidence-ref,
  ? "context": { * tstr => any }
}
```

### 4.3 Evidence and Integrity

- If `evidence.hash_alg` is present, `evidence.hash` MUST be present.
- If `evidence.hash` is present, `evidence.hash_alg` MUST be present.
- Submission acceptance MUST NOT depend on dereferencing `evidence.uri` or external fetch success.
- MTI verifiable-evidence profile: when `evidence.inline` is present, `evidence.hash_alg` and `evidence.hash` MUST be present and hash verification of `inline` MUST succeed; otherwise reject with `4001`.
- Implementations claiming verifiable-attestation interoperability MUST implement the `evidence.inline` verification path above.
- URI-only evidence references are advisory and MAY be verified asynchronously by local policy workflows.
- Evidence hash mismatch found outside the synchronous submit path MUST NOT change deterministic submit response and SHOULD be handled through dispute/policy flows.
- Evidence references MAY be unavailable at query time; score computation MUST remain deterministic from stored event fields.
- Aggregators MUST persist one `submit-attestation` per accepted event.
- In self-query profile, `submit-attestation.submit_envelope` MUST be present and MUST be the exact signed AMP envelope bytes received for `signal_submit`.
- In self-query profile, `submit-attestation.hash` MUST equal `sha-256(submit-attestation.submit_envelope)`.
- `submit-attestation.submit_envelope` size MUST be `<= 8192` bytes; oversized envelopes MUST be rejected with `4001`.
- `submit-attestation.submit_envelope` MUST decode to a signed AMP envelope that is either:
  - Direct profile submit: `typ=REQUEST`, `body.rep_v=1`, and `body.op=signal_submit`; or
  - CAP-exposed profile submit: outer dispatch via RFC 004 (`CAP_INVOKE`) targeting `id = xyz.agentries.reputation.workflow:1.0.0`, where `CAP_INVOKE.params` contains `rep_v=1` and `op=signal_submit`.
- The resolved submit payload `event` MUST be byte-equivalent to the stored/reported event object.
- In public-query profile responses, `submit-attestation.submit_envelope` MUST be `null` while preserving the same `hash_alg/hash` values from the self-query attestation.

### 4.4 Event Lifecycle and Idempotency

Event lifecycle states:
- `active`
- `disputed`
- `reverted`
- `expired`

Idempotency rules:
- Submission key is (`reporter`, `event_id`).
- If same key and byte-identical `event` payload are resubmitted, operation MUST be idempotent success.
- If same key but payload differs, operation MUST fail with `4001`.

---

## 5. Reputation Protocol Semantics

### 5.1 Message Type Usage and Dispatch

This RFC defines two dispatch profiles:
- Direct reputation profile (MTI in this RFC): requests MUST use `typ = REQUEST`; responses MUST use `typ = RESPONSE` and MUST set envelope `reply_to` to triggering request `id`.
- CAP-exposed reputation profile (optional): outer dispatch follows RFC 004 (`CAP_INVOKE`/`CAP_RESULT`), while reputation operation semantics in Sections 5.2-5.6 apply to the capability payload.
- Dispatch to this RFC occurs only when reputation payload includes `rep_v`.
- For dispatched reputation payloads, `op` is REQUIRED and MUST match Section 5 operation set.
- Unknown reputation `op` values MUST be rejected with `4001`.
- Session-scoped usage MAY include signed `body.session` context; when present (or when session-scoped validation is triggered by RFC 006), validation and error mapping MUST follow RFC 006 Section 4.4 and Section 9.

Operation matrix:

The following matrix applies to the Direct reputation profile. In CAP-exposed profile, outer type/correlation follows RFC 004, while inner reputation semantics and status mapping follow this RFC after capability dispatch.

| Request `op` | Sender -> Receiver | Response `op` | `reply_to` |
|--------------|--------------------|---------------|------------|
| `signal_submit` | producer -> aggregator | `signal_submit_result` | MUST reference submit request ID |
| `signal_query` | consumer/subject -> aggregator | `signal_query_result` | MUST reference query request ID |
| `score_query` | consumer/subject -> aggregator | `score` | MUST reference query request ID |
| `dispute_open` | subject -> aggregator | `dispute_open_result` | MUST reference dispute request ID |
| `dispute_resolve` | operator -> aggregator | `dispute_resolve_result` | MUST reference resolve request ID |

### 5.2 SIGNAL_SUBMIT

Rules:
- Core MTI behavior: authenticated caller DID MUST equal `event.reporter`.
- `event.subject` and `event.reporter` MUST be valid DID strings.
- `event.observed_at` MUST NOT exceed `server_now + 300000` (5 minutes), otherwise reject with `4001`.
- Aggregator MUST apply idempotency rules from Section 4.4 before persistence.
- Accepted events enter `active` unless local policy marks as `disputed` immediately.

### 5.3 SIGNAL_QUERY

Rules:
- Query filters MAY include `subject`, optional `subject_service`, optional `category`, optional pagination.
- `view` defaults to `"self"` when absent.
- MTI query authorization for raw events (`view="self"`): authenticated caller DID MUST equal query `subject`; otherwise reject with `3001`.
- Public raw-event query (`view="public"`) is optional and only valid when implementation enables Public Query Profile; otherwise reject with `3001`.
- `limit` default is `20` when absent and MUST be in `1..20` when present; out-of-range values MUST be rejected with `4001`.
- Result ordering MUST be deterministic: `observed_at` descending, then `event_id` lexical ascending.
- `cursor` MUST be treated as opaque by clients.
- If `cursor` is present, requester MUST keep `subject`, `subject_service`, and `category` unchanged from the original query page; `limit` MAY change.
- Invalid or expired cursor, or cursor/query-context mismatch, MUST be rejected with `4001`.
- `signal_query_result` MUST include event lifecycle status metadata for each returned event and MAY include `next_cursor` and `has_more`.
- `signal_query_result` MUST include per-event `submit-attestation` for independent signature verification.
- For `view="public"`, `signal_query_result.events[*].event.context` and `signal_query_result.events[*].event.evidence.inline` MUST be omitted, and `attestation.submit_envelope` MUST be `null`.

### 5.4 SCORE_QUERY

Rules:
- `score_query` requires `subject`.
- MTI counterparty-query baseline: authenticated callers MUST be allowed to query score for any `subject`.
- `subject_service` MAY be supplied for service-scoped score calculation.
- `window_days` MAY be supplied to bound eligible events to a trailing window and MUST be within `1..3650` when present.
- `as_of` MAY be supplied to force deterministic replay; if absent, server evaluation time is used.
- Aggregator MUST compute score via Section 5.6 deterministic algorithm.
- If no eligible events exist, return `score=0`, `tier="insufficient_data"`, and `event_count=0`.

### 5.5 DISPUTE_OPEN and DISPUTE_RESOLVE

Rules:
- `dispute_open` MUST reference existing `event_id`; if not found, reject with `4001`.
- `dispute_open.subject` MUST equal referenced `event.subject`.
- Authenticated caller identity MUST equal `dispute_open.subject`; mismatch MUST be rejected with `3001`.
- Opening a dispute transitions referenced event to `disputed`.
- `dispute_resolve` MUST be restricted to authorized operators by local policy.
- `dispute_resolve` MUST reference existing `dispute_id`; if not found, reject with `4001`.
- Resolution:
  - `upheld`: event returns to `active`.
  - `reverted`: event transitions to `reverted` and MUST be excluded from scoring.

### 5.6 Deterministic Aggregation and Decay Algorithm

For each `score_query(subject, subject_service?, category?, window_days?, as_of?)`:
1. Determine evaluation timestamp `eval_now`:
   - if `as_of` present, `eval_now = as_of`;
   - else `eval_now = server_now`.
   - `eval_now` MUST be interpreted as Unix epoch milliseconds in UTC.
   - `allowed_clock_skew_ms` is fixed at `300000` (5 minutes) for this RFC revision.
   - if `as_of > server_now + allowed_clock_skew_ms`, reject with `4001`.
2. Build eligible set `E`:
   - `event.subject == subject`
   - if `subject_service` present, `event.subject_service == subject_service`
   - if `category` present, `event.category == category`
   - `event.observed_at <= eval_now`
   - state is `active`
   - not expired (`expires_at` absent or `expires_at > eval_now`)
   - if `window_days` present, `event.observed_at >= eval_now - window_days * 86400000`
3. For each `e in E`, compute:
   - `age_days = floor((eval_now - e.observed_at) / 86400000)`
   - `decay_bp = max(0, 10000 - min(age_days, 50) * 200)`  ; linear 2%/day, floor at day 50
   - `norm_i = e.confidence * decay_bp`
   - `weighted_i = e.weight * norm_i`
4. If `sum(norm_i) == 0`, return insufficient data (`score=0`, tier=`insufficient_data`, `event_count=|E|`).
5. `score = trunc_toward_zero(sum(weighted_i) / sum(norm_i))`
6. Clamp score into `[-100, 100]`.
7. Map score to tier:
   - `>= 60` -> `trusted`
   - `>= 20` and `< 60` -> `caution`
   - `> -20` and `< 20` -> `neutral`
   - `> -60` and `<= -20` -> `risky`
   - `<= -60` -> `blocked`

All arithmetic MUST be integer deterministic; floating-point implementations MUST produce equivalent truncation behavior.

When score is computed, response `computed_at` MUST equal `eval_now`.

### 5.7 Reputation Body CDDL

```cddl
score-tier = "trusted" / "caution" / "neutral" / "risky" / "blocked" / "insufficient_data"
event-state = "active" / "disputed" / "reverted" / "expired"
dispute-state = "none" / "open" / "resolved_upheld" / "resolved_reverted"
query-view = "self" / "public"
rep-session-context = {
  "session_id": bstr .size 16,
  "session_scope": true
}

signal-submit-body = {
  "rep_v": 1,
  "op": "signal_submit",
  "event": reputation-event,
  ? "session": rep-session-context
}

signal-submit-result-body = {
  "rep_v": 1,
  "op": "signal_submit_result",
  "event_id": event-id,
  "status": "accepted" / "duplicate" / "rejected",
  ? "reason": tstr,
  ? "session": rep-session-context
}

signal-query-body = {
  "rep_v": 1,
  "op": "signal_query",
  "subject": did,
  ? "subject_service": service-id,
  ? "category": rep-category,
  ? "cursor": tstr,
  ? "limit": uint,
  ? "view": query-view,
  ? "session": rep-session-context
}

signal-query-event = {
  "event": reputation-event,
  "event_state": event-state,
  "dispute_state": dispute-state,
  "attestation": submit-attestation
}

signal-query-result-body = {
  "rep_v": 1,
  "op": "signal_query_result",
  "subject": did,
  ? "subject_service": service-id,
  "events": [* signal-query-event],
  ? "next_cursor": tstr / null,
  "has_more": bool,
  ? "session": rep-session-context
}

score-query-body = {
  "rep_v": 1,
  "op": "score_query",
  "subject": did,
  ? "subject_service": service-id,
  ? "category": rep-category,
  ? "window_days": uint,
  ? "as_of": unix-ms,
  ? "session": rep-session-context
}

score-body = {
  "rep_v": 1,
  "op": "score",
  "subject": did,
  ? "subject_service": service-id,
  "score": int,
  "tier": score-tier,
  "event_count": uint,
  ? "window_days": uint,
  ? "as_of": unix-ms,
  "computed_at": unix-ms,
  ? "session": rep-session-context
}

dispute-open-body = {
  "rep_v": 1,
  "op": "dispute_open",
  "event_id": event-id,
  "subject": did,
  "reason": tstr,
  ? "evidence": evidence-ref,
  ? "session": rep-session-context
}

dispute-open-result-body = {
  "rep_v": 1,
  "op": "dispute_open_result",
  "dispute_id": dispute-id,
  "event_id": event-id,
  "status": "opened" / "rejected",
  ? "reason": tstr,
  ? "session": rep-session-context
}

dispute-resolve-body = {
  "rep_v": 1,
  "op": "dispute_resolve",
  "dispute_id": dispute-id,
  "resolution": "upheld" / "reverted",
  ? "reason": tstr,
  ? "session": rep-session-context
}

dispute-resolve-result-body = {
  "rep_v": 1,
  "op": "dispute_resolve_result",
  "dispute_id": dispute-id,
  "status": "resolved" / "rejected",
  ? "reason": tstr,
  ? "session": rep-session-context
}

rep-capability-id = "xyz.agentries.reputation.workflow:1.0.0"

rep-cap-invoke-params =
  signal-submit-body /
  signal-query-body /
  score-query-body /
  dispute-open-body /
  dispute-resolve-body

rep-cap-result-success =
  signal-submit-result-body /
  signal-query-result-body /
  score-body /
  dispute-open-result-body /
  dispute-resolve-result-body
```

### 5.8 CAP-Exposed Reputation Profile Mapping

This section defines RFC 009 capability interoperability baseline for RFC 004 invocation path.

Capability identity:
- `id = "xyz.agentries.reputation.workflow:1.0.0"`.

Rules:
- CAP path MUST target capability ID above.
- `CAP_INVOKE.params` MUST contain exactly one RFC 009 request body with `rep_v = 1`.
- Allowed CAP request ops: `signal_submit`, `signal_query`, `score_query`, `dispute_open`, `dispute_resolve`.
- `CAP_RESULT(status="success").result` MUST contain corresponding RFC 009 response body.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005 before reputation execution.
- Invalid/unsupported delegation evidence in CAP path MUST fail with `3004` (RFC 004/005).
- Section 5.1 Direct `REQUEST`/`RESPONSE` direction rules MUST NOT be applied to CAP envelope types.
- Providers supporting CAP path MUST publish an RFC 004-compliant descriptor for `xyz.agentries.reputation.workflow:1.0.0`.
- Session context source-of-truth in CAP path is RFC 004 envelope extension (`CAP_INVOKE.body.session`, `CAP_RESULT.body.session`) with semantics governed by RFC 006.
- `CAP_INVOKE.params.session` and `CAP_RESULT.result.session` MAY exist for payload-level compatibility, but if both payload and envelope session context are present, they MUST be semantically equivalent; mismatch MUST fail with `4001`.
- Pre-execution rejection in CAP path (validation/authorization/compatibility/schema) MUST return AMP `ERROR` per RFC 004 Section 7.2.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.

---

## 6. State Machines

### 6.1 Event State Machine

```text
NEW
  -> ACTIVE             (accepted submit)
ACTIVE
  -> DISPUTED           (dispute_open)
  -> EXPIRED            (`expires_at` elapsed)
DISPUTED
  -> ACTIVE             (resolve upheld)
  -> REVERTED           (resolve reverted)
ACTIVE / REVERTED / EXPIRED
  -> TERMINAL
```

### 6.2 Dispute State Machine

```text
NONE
  -> OPEN               (dispute_open accepted)
OPEN
  -> RESOLVED_UPHELD    (dispute_resolve upheld)
  -> RESOLVED_REVERTED  (dispute_resolve reverted)
RESOLVED_UPHELD / RESOLVED_REVERTED
  -> TERMINAL
```

---

## 7. Error Handling and Retry

Deterministic mapping:

| Condition | Code | Retry |
|-----------|------|-------|
| Malformed reputation body | `1001` | No |
| Malformed `body.session` context object (type/shape/size) | `1001` | No |
| Unsupported `rep_v` | `1004` | No |
| Unauthorized reporter/subject/operator action | `3001` | No |
| `dispute_open` caller DID differs from `dispute_open.subject` | `3001` | No |
| `signal_query` caller DID differs from query `subject` (MTI raw-event profile) | `3001` | No |
| `signal_query.view="public"` not enabled/authorized by policy | `3001` | No |
| Invalid/unsupported delegation evidence in CAP reputation path (`CAP_INVOKE.body.delegation`) | `3004` | No |
| `body.delegation` present on non-CAP reputation message | `4001` | No |
| Invalid `op`/`typ` direction or `reply_to` correlation | `4001` | No |
| Invalid score field ranges (`weight`, `confidence`, timestamps) | `4001` | No |
| MTI inline evidence hash verification failed | `4001` | No |
| Submit attestation validation failed (hash/decode/event binding mismatch) | `4001` | No |
| `submit-attestation.submit_envelope` exceeds 8192 bytes | `4001` | No |
| `signal_query.limit` out of allowed range | `4001` | No |
| Invalid `window_days` range or invalid `as_of` constraints | `4001` | No |
| Invalid/expired cursor or cursor/query-context mismatch | `4001` | No |
| Duplicate (`reporter`, `event_id`) with conflicting payload | `4001` | No |
| Referenced `event_id` or `dispute_id` not found | `4001` | No |
| Subject DID resolution failure (when resolution required by policy) | `2001` | Maybe |
| Reputation store unavailable | `5002` | Yes |
| Internal reputation engine failure | `5001` | Yes |

Retry guidance:
- `5002` and `5001` MAY be retried with bounded backoff.
- `100x/3001/3004/4001` SHOULD NOT be retried without payload/policy changes.

---

## 8. Versioning and Compatibility

Version dimensions:
- AMP envelope `v` remains governed by RFC 001.
- Reputation body version is `rep_v`, fixed at `1` in this revision.

Compatibility rules:
- Unsupported `rep_v` MUST be rejected with `1004`.
- Unknown optional fields MAY be ignored unless policy/security-sensitive.
- `body.delegation` MUST NOT be treated as ignorable optional input on non-CAP reputation paths; delegation carriage/validation remains governed by RFC 001 Section 4.6 and RFC 005.
- Backward-compatible evolution MUST use optional-field extension only.

---

## 9. Security Considerations

- Implementations MUST verify message signatures before processing reputation mutations.
- Reporter identity MUST be bound to authenticated principal policy (RFC 002).
- Reputation score MUST NOT bypass authorization, delegation, signature, or policy checks from RFC 001/005.
- Aggregators SHOULD rate-limit signal submissions per reporter and per principal.
- Dispute resolution actions MUST be auditable and operator-authorized.
- Evidence URIs MAY be malicious; implementations SHOULD treat external evidence fetch as untrusted IO.

---

## 10. Privacy Considerations

- Reputation events may expose behavioral history; implementations SHOULD minimize public detail.
- Evidence artifacts SHOULD be referenced by hash/URI rather than embedding sensitive payloads.
- Aggregators SHOULD provide retention and deletion policy for stale or reverted events.
- Consumers SHOULD avoid broad redistribution of raw events when score snapshots are sufficient.

---

## 11. Implementation Checklist

- Implement deterministic operation dispatch (`rep_v` + `op`).
- Enforce (`reporter`, `event_id`) idempotency and conflict handling.
- Enforce strict reporter-binding MTI baseline.
- Enforce `dispute_open` caller binding (`caller_did == dispute_open.subject`).
- Enforce MTI raw-event query binding (`signal_query.caller_did == subject`).
- Enforce MTI counterparty-score support (`score_query` allowed for caller DID != subject DID).
- Validate score field ranges and timestamp constraints.
- Implement MTI inline evidence hash verification (`evidence.inline` + `hash_alg` + `hash`).
- Persist and return `submit-attestation` with envelope hash and event-binding checks.
- Enforce `submit-attestation.submit_envelope` size bound (`<= 8192`) and deterministic rejection mapping.
- Validate `signal_query.limit` default/range behavior (`default=20`, valid `1..20`).
- Implement optional `signal_query.view="public"` profile redaction rules when enabled.
- Validate `window_days`/`as_of` constraints and fixed clock-skew rule.
- Implement deterministic aggregation/decay algorithm in Section 5.6.
- Implement dispute open/resolve state transitions.
- Implement opaque cursor handling with query-context binding.
- Enforce RFC 006 session context validation/mapping when `body.session` is present.
- If CAP path is supported, implement Section 5.8 fixed capability mapping.
- Return event/dispute lifecycle status in `signal_query_result`.
- Apply deterministic Section 7 error mapping.
- Preserve audit logs for mutation and resolution operations.
- Treat score output as advisory, never as auth bypass.

---

## 12. References

### 12.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 8949: CBOR
- RFC 8610: CDDL

### 12.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay & Store-and-Forward
- RFC 004: Capability Schema Registry & Compatibility
- RFC 005: Delegation Credentials & Authorization
- RFC 006: Session Protocol
- RFC 007: Agent Payment Protocol
- RFC 008: Agent Discovery & Directory
- RFC 010: Observability & Evaluation Telemetry
- DID Core (W3C Recommendation)

---

## Appendix A. Minimal Test Vectors

### A.1 SIGNAL_SUBMIT Positive

Input:
- Valid `signal_submit` with bounded `weight`/`confidence` and valid reporter binding.

Expected:
- Event accepted and persisted as `active`.

### A.2 SIGNAL_SUBMIT Duplicate Idempotent Positive

Input:
- Same (`reporter`, `event_id`) and byte-identical payload resubmitted.

Expected:
- Deterministic idempotent success (`status="duplicate"` or policy-equivalent success).

### A.3 SIGNAL_SUBMIT Duplicate Conflict Negative

Input:
- Same (`reporter`, `event_id`) but payload differs.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.4 SCORE_QUERY Deterministic Positive

Input:
- Fixed eligible event set with known `weight`, `confidence`, and `observed_at`.
- Query provides explicit `as_of`.

Expected:
- Score and tier match deterministic Section 5.6 computation.

### A.5 Unsupported rep_v Negative

Input:
- `rep_v = 2` request body.

Expected:
- Reject with `1004 UNSUPPORTED_VERSION`.

### A.6 DISPUTE_OPEN and DISPUTE_RESOLVE Positive

Input:
- Subject opens dispute for existing active event.
- Authorized operator resolves `reverted`.

Expected:
- Event transitions `active -> disputed -> reverted`.
- Reverted event excluded from future score aggregation.

### A.7 Unauthorized DISPUTE_RESOLVE Negative

Input:
- Caller without operator privilege sends `dispute_resolve`.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.8 Advisory Boundary Positive

Input:
- Low trust score for subject DID but valid signed delegated invocation request.

Expected:
- Authorization decision still follows RFC 005 policy checks.
- Reputation score does not independently deny/allow authorization path.

### A.9 Byte-Level Error Code Checks

Input:
- Unauthorized action mapped to `3001`.
- Invalid payload/range mapped to `4001`.
- Missing referenced resource mapped to `4001`.
- Unsupported `rep_v` mapped to `1004`.

Expected:
- `3001` CBOR uint encoding bytes: `19 0b b9`.
- `4001` CBOR uint encoding bytes: `19 0f a1`.
- `1004` CBOR uint encoding bytes: `19 03 ec`.

### A.10 SCORE_QUERY with window_days Positive

Input:
- Event set includes old and recent events.
- Query sets `window_days=7`.

Expected:
- Only events with `observed_at >= eval_now - 7 * 86400000` are eligible.
- Returned `event_count` reflects only in-window active events.

### A.11 SIGNAL_QUERY Cursor Context Mismatch Negative

Input:
- Page 1 request uses (`subject=A`, `category=payment`) and receives `cursor=X`.
- Page 2 reuses `cursor=X` but changes `category=quality`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.12 Service-Scoped Score Positive

Input:
- Subject DID has events for `subject_service=svc.alpha` and `subject_service=svc.beta`.
- Query uses `subject_service=svc.alpha`.

Expected:
- Score is computed only from `svc.alpha` events.
- Response echoes `subject_service=svc.alpha`.

### A.13 Future-Observed Event Exclusion (as_of) Positive

Input:
- Query uses explicit `as_of=T`.
- Candidate event has `observed_at > T`.

Expected:
- Event is excluded from eligible set.
- No negative age/over-100% decay behavior is possible.

### A.14 Referenced Resource Not Found Negative

Input:
- `dispute_open` references unknown `event_id`; or `dispute_resolve` references unknown `dispute_id`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.15 DISPUTE_OPEN Caller-Subject Mismatch Negative

Input:
- `dispute_open.subject = did:web:example.com:agent:alice`.
- Authenticated caller DID is `did:web:example.com:agent:bob`.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.16 Inline Evidence Hash Mismatch Negative

Input:
- `signal_submit` carries `evidence.inline` bytes with `hash_alg/hash` that do not match.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.17 SIGNAL_QUERY Caller-Subject Mismatch Negative

Input:
- `signal_query.subject = did:web:example.com:agent:alice`.
- Authenticated caller DID is `did:web:example.com:agent:bob`.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.18 Submit Attestation Verification Positive

Input:
- `signal_query_result.events[i].attestation.submit_envelope` decodes as signed `signal_submit` envelope.
- `attestation.hash` equals `sha-256(submit_envelope)`.
- Decoded submit `event` equals returned event object.

Expected:
- Consumer can independently verify reporter signature from `submit_envelope`.
- Attestation hash and event-binding checks pass.

### A.19 SCORE_QUERY Counterparty Positive

Input:
- `score_query.subject = did:web:example.com:agent:alice`.
- Authenticated caller DID is `did:web:example.com:agent:bob`.

Expected:
- Query is accepted under MTI counterparty-query baseline.
- Response is a normal `score` payload (or policy-equivalent success object), not `3001`.

### A.20 CAP Reputation Profile Positive

Input:
- `CAP_INVOKE` targets `id = xyz.agentries.reputation.workflow:1.0.0`.
- `CAP_INVOKE.params` contains valid RFC 009 `signal_submit` body (`rep_v=1`).

Expected:
- RFC 004 capability validation passes.
- Reputation semantics execute and return `CAP_RESULT(status="success")` with RFC 009 result body.

### A.21 SIGNAL_QUERY Public View Redaction Positive

Input:
- Aggregator enables optional Public Query Profile.
- Caller DID differs from `signal_query.subject`.
- Request uses `signal_query.view="public"`.

Expected:
- Query accepted.
- Returned events omit `event.context` and `event.evidence.inline`.
- Returned `attestation.submit_envelope` is `null` and hash fields remain present.

### A.22 Submit Attestation Envelope Oversize Negative

Input:
- `signal_submit` envelope bytes exceed 8192 bytes.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.23 CAP Delegation Evidence Invalid Negative

Input:
- `CAP_INVOKE` targets `id = xyz.agentries.reputation.workflow:1.0.0`.
- `CAP_INVOKE.body.delegation` is present but invalid/unsupported under RFC 005 validation.

Expected:
- Reject with `3004 DELEGATION_INVALID`.

### A.24 Session Context Shape Invalid Negative

Input:
- Session-scoped reputation request carries malformed `body.session` (for example non-map, wrong `session_id` size, or non-true `session_scope`).

Expected:
- Reject with `1001 INVALID_MESSAGE` per RFC 006 session-context parsing rules.

### A.25 CAP Session Context Mismatch Negative

Input:
- CAP path request includes both `CAP_INVOKE.body.session` and `CAP_INVOKE.params.session` with semantically inconsistent values.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.26 Byte-Level Error Code Checks (Delegation + Session)

Input:
- Invalid CAP delegation evidence mapped to `3004`.
- Session context semantic mismatch mapped to `4001`.

Expected:
- `3004` CBOR uint encoding bytes: `19 0b bc`.
- `4001` CBOR uint encoding bytes: `19 0f a1`.

---

## Appendix B. Open Questions

No open questions in this revision.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-06 | Proposal | Nowa | Initial outline for reputation and trust concepts |
| 2026-02-07 | 0.1 | Nowa | Rewrote RFC 009 into normative draft format with conformance profiles, boundary contracts, deterministic aggregation/dispute semantics, CDDL schemas, error mapping, and minimal test vectors |
| 2026-02-07 | 0.2 | Nowa | Added service-scoped subject model, deterministic `window_days`/`as_of` scoring rules, query cursor/context constraints, lifecycle status in query results, stronger evidence hash requirements, and added vectors A.10-A.12 |
| 2026-02-07 | 0.3 | Nowa | Fixed deterministic-time edge case (`observed_at <= eval_now`), added strict-vs-delegated mutation baseline, standardized not-found to `4002`, added delegation error `3004`, and expanded vectors A.13-A.14 |
| 2026-02-07 | 0.4 | Nowa | Aligned delegation semantics with RFC 001/005 CAP_INVOKE-only carriage, removed direct reputation `delegation` fields, remapped missing references to `4001`, and synchronized vector/error checks accordingly |
| 2026-02-07 | 0.5 | Nowa | Added explicit `dispute_open` caller-subject binding, clarified Direct vs CAP dispatch profiles, introduced MTI inline evidence verification baseline, and added vectors A.15-A.16 |
| 2026-02-07 | 0.6 | Nowa | Added submit-attestation verification model, enforced MTI self-query authorization for `signal_query/score_query`, made insufficient-data `event_count` deterministic, and added vectors A.17-A.18 |
| 2026-02-07 | 0.7 | Nowa | Relaxed score-query auth to MTI counterparty baseline, constrained raw-event query pagination (`limit` default/range), aligned submit-attestation validation with both Direct and CAP-exposed submit dispatch, and added vector A.19 |
| 2026-02-07 | 0.8 | Nowa | Added optional Public Query Profile baseline and explicit redaction requirements for raw-event exposure |
| 2026-02-07 | 0.9 | Nowa | Completed CAP-exposed profile mapping with fixed capability ID, added attestation envelope size bound (`<=8192`) with deterministic errors, and expanded vectors A.20-A.22 |
| 2026-02-07 | 0.10 | Nowa | Aligned session-scoped carriage with RFC 006 via explicit `body.session` CDDL fields, added CAP delegation invalid mapping (`3004`), and added vector A.23 |
| 2026-02-07 | 0.11 | Nowa | Added session/CAP negative vectors A.24-A.25, added byte-level `3004` check via A.26, and clarified non-CAP delegation ignore rules in compatibility section |
| 2026-02-10 | 0.11 | Nowa | Synchronized document metadata (`Updated`) and repository gate-status references for audit consistency |
