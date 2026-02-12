# RFC 010: Observability & Evaluation Telemetry

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-06
**Updated**: 2026-02-10
**Version**: 0.5

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (principal binding and endpoint auth policy)
- RFC 003: Relay & Store-and-Forward (at-least-once delivery behavior)
- RFC 004: Capability Schema Registry & Compatibility (optional capability publication)
- RFC 005: Delegation Credentials & Authorization (delegated telemetry execution policy)
- RFC 006: Session Protocol (optional session-scoped telemetry context)
- RFC 007: Agent Payment Protocol (payment outcomes as telemetry sources)
- RFC 008: Agent Discovery & Directory (telemetry endpoint hint discovery)
- RFC 009: Reputation & Trust Signals (telemetry-derived trust inputs)
- RFC 011: Multi-Agent Coordination & Group Messaging (coordination-derived telemetry sources)

---

## Abstract

This RFC defines interoperable telemetry and evaluation semantics for AMP agents. It standardizes how signed telemetry events and evaluation outcomes are submitted, queried, and aggregated into deterministic rollups. The goal is portable observability and quality measurement without turning telemetry into an authorization primitive.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Telemetry Data Model
4.1 Event Taxonomy
4.2 Telemetry and Evaluation CDDL
4.3 Privacy Classes and Redaction
4.4 Idempotency and Retention
5. Telemetry Protocol Semantics
5.1 Message Type Usage and Dispatch
5.2 TELEMETRY_SUBMIT
5.3 TELEMETRY_QUERY
5.4 ROLLUP_QUERY
5.5 EVAL_SUBMIT and EVAL_QUERY
5.6 Deterministic Bucketing and Aggregation
5.7 Telemetry Body CDDL
5.8 CAP-Exposed Telemetry Profile Mapping
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

Agent ecosystems need portable operational visibility across implementations. Without shared telemetry semantics, reliability and quality metrics become incomparable, and post-incident forensics depend on vendor-specific logs.

This RFC defines:
- Signed telemetry event submission/query semantics.
- Signed evaluation result submission/query semantics.
- Deterministic rollup aggregation rules and bucket alignment.
- Deterministic error mapping for telemetry operations.
- Privacy classes and redaction behavior for query paths.

This RFC does not define:
- AMP envelope/security primitives (RFC 001).
- Transport protocol negotiation and framing (RFC 002).
- Log storage backend topology or vendor-specific observability pipeline internals.
- Authorization policy decisions (RFC 005 remains authoritative).
- Reputation scoring rules (RFC 009).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope/signature semantics for telemetry operations.
- Enforces dispatch, query, and aggregation rules in Section 5.
- Applies deterministic error mapping in Section 7.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Telemetry event | Signed record of one observed runtime fact (latency, error, throughput, policy decision, etc.). |
| Evaluation result | Signed outcome of one quality/evaluation run against a scenario or rubric. |
| Subject | DID being observed/evaluated. |
| Reporter | DID submitting telemetry/evaluation records. |
| Privacy class | Event visibility policy: `internal`, `partner`, or `public`. |
| Rollup bucket | Deterministic time-window aggregate over telemetry samples. |
| Eligible sample | Telemetry event that matches filter/window and passes privacy policy for current query view. |

### 2.2 Role Profiles and MTI Requirements

`Telemetry Producer Profile`:
- MUST submit signed telemetry events with stable `event_id`.
- MUST provide valid `subject`, `reporter`, `event_type`, `occurred_at`, and numeric `value`.
- MUST preserve idempotency on retries with same `event_id`.

`Telemetry Collector Profile`:
- MUST enforce reporter identity binding and validation rules.
- MUST implement deterministic rollups in Section 5.6.
- MUST expose query/rollup responses with deterministic ordering and errors.
- Core MTI baseline is strict reporter binding:
  - for `telemetry_submit`, authenticated caller DID MUST equal `event.reporter`;
  - for `eval_submit`, authenticated caller DID MUST equal `evaluation.reporter`.

`Telemetry Consumer Profile`:
- MUST treat telemetry as observability input, not authorization proof.
- SHOULD retain query timestamps and filter criteria for auditability.

`Evaluation Runner Profile` (optional extension):
- MAY submit evaluation outcomes via `eval_submit`.
- MUST use deterministic score range (`0..100`) and outcome enums.

`Public Metrics Profile` (optional extension):
- MAY allow cross-subject rollup query in `view="public"`.
- MUST apply redaction/privacy filtering from Section 4.3.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- This RFC uses `REQUEST`/`RESPONSE` envelopes and does not allocate new AMP type codes.
- Telemetry operations dispatch only when signed body includes `tel_v`.
- Signature, `reply_to`, and idempotency semantics remain governed by RFC 001.

With RFC 002:
- Reporter identity authorization MUST respect transport principal binding policy.
- Unauthorized principal/from combinations for telemetry operations MUST map to `3001`.

With RFC 003:
- At-least-once delivery can produce duplicates; collectors MUST enforce idempotency by (`reporter`, `event_id`) and (`reporter`, `eval_id`) where applicable.
- Relay behavior MUST NOT modify telemetry body bytes.

With RFC 004:
- Telemetry endpoints MAY be exposed as capabilities.
- If carried via `CAP_INVOKE`, capability negotiation and schema checks follow RFC 004.
- CAP interoperability baseline capability ID for this RFC is `org.agentries.telemetry.workflow:1.0.0` (Section 5.8).

With RFC 005:
- Delegation semantics remain governed by RFC 005.
- This RFC message set does not define non-CAP delegation carriage; delegated execution (if used) follows RFC 001 Section 4.6 and RFC 005 via `CAP_INVOKE.body.delegation`.

With RFC 006:
- Telemetry operations MAY be session-scoped.
- Session context validation remains governed by RFC 006.

With RFC 007:
- Payment lifecycle outcomes MAY be source telemetry signals.
- Payment settlement validity MUST NOT be replaced by telemetry records.

With RFC 008:
- Discovery MAY publish telemetry endpoint hints.
- Discovery metadata MUST NOT override signed telemetry semantics.

With RFC 009:
- Reputation engines MAY consume telemetry as one input source.
- Telemetry success/failure signals MUST NOT bypass RFC 009 dispute/review policies.

With RFC 011:
- Multi-agent coordination outcomes MAY be source telemetry signals.
- Coordination state mutation MUST remain governed by RFC 011 and MUST NOT be driven by telemetry queries/results.

---

## 4. Telemetry Data Model

### 4.1 Event Taxonomy

MTI telemetry event categories:
- `transport`: delivery latency, retries, endpoint failures.
- `execution`: runtime duration, token counts, tool outcomes.
- `capability`: invocation status, compatibility failures.
- `payment`: quote/capture/refund timing and result codes.
- `policy`: authorization/guardrail decisions.
- `quality`: evaluation-related quality signals.

Event type naming:
- `event_type` MUST be lowercase dot-delimited token path (for example `execution.latency_ms`, `payment.capture_status`).
- `event_type` length MUST be `1..128`.

### 4.2 Telemetry and Evaluation CDDL

```cddl
did = tstr
unix-ms = uint
event-id = bstr .size 16
eval-id = bstr .size 16
session-id = bstr .size 16

privacy-class = "internal" / "partner" / "public"
rollup-interval = "1m" / "5m" / "1h" / "1d"
raw-query-view = tstr
rollup-query-view = "self" / "public"

telemetry-session-context = {
  "session_id": session-id,
  "session_scope": true
}

telemetry-event = {
  "event_id": event-id,
  "subject": did,
  "reporter": did,
  "event_type": tstr,
  "category": "transport" / "execution" / "capability" / "payment" / "policy" / "quality",
  "value": int,
  ? "unit": tstr,
  "privacy": privacy-class,
  "occurred_at": unix-ms,
  ? "dimensions": { * tstr => tstr },
  ? "retention_expires_at": unix-ms
}

evaluation-result = {
  "eval_id": eval-id,
  "subject": did,
  "reporter": did,
  "scenario": tstr,
  "score": uint, ; MUST be in [0,100]
  "outcome": "pass" / "fail" / "inconclusive",
  "executed_at": unix-ms,
  ? "notes": tstr,
  ? "artifact_hash_alg": "sha-256" / "sha-512",
  ? "artifact_hash": bstr
}
```

### 4.3 Privacy Classes and Redaction

Reserved sensitive dimension keys (case-sensitive):
- `email`, `phone`, `ip`, `auth_header`, `token`, `wallet`, `prompt_raw`, `response_raw`.

Rules:
- `privacy="public"` events MUST NOT include reserved sensitive keys in `dimensions`; violation MUST be rejected with `4001`.
- `view="public"` rollup responses MUST exclude raw event payloads and return only aggregated bucket values.
- `telemetry_query` in MTI profile is raw-event query and remains `view="self"` only.

### 4.4 Idempotency and Retention

Idempotency rules:
- Telemetry submit key is (`reporter`, `event_id`).
- Evaluation submit key is (`reporter`, `eval_id`).
- Same key + byte-identical payload MUST be idempotent success.
- Same key + conflicting payload MUST fail with `4001`.

Retention rules:
- If `retention_expires_at` is present, it MUST be greater than `occurred_at`.
- Expired retained records MAY be removed by policy and omitted from query/rollup results.

---

## 5. Telemetry Protocol Semantics

### 5.1 Message Type Usage and Dispatch

This RFC defines two dispatch profiles:
- Direct telemetry profile (MTI in this RFC): requests MUST use `typ = REQUEST`; responses MUST use `typ = RESPONSE` and MUST set envelope `reply_to` to triggering request `id`.
- CAP-exposed telemetry profile (optional): outer dispatch follows RFC 004 (`CAP_INVOKE`/`CAP_RESULT`), while telemetry semantics apply to capability payload.
- Dispatch to this RFC occurs only when telemetry payload includes `tel_v`.
- For dispatched telemetry payloads, `op` is REQUIRED and MUST match this RFC operation set.
- Unknown telemetry `op` values MUST be rejected with `4001`.
- Session-scoped usage MAY include signed `body.session` context; when present (or when session-scoped validation is triggered by RFC 006), validation and error mapping MUST follow RFC 006.

Operation matrix:

| Request `op` | Sender -> Receiver | Response `op` | `reply_to` |
|--------------|--------------------|---------------|------------|
| `telemetry_submit` | producer -> collector | `telemetry_submit_result` | MUST reference submit request ID |
| `telemetry_query` | consumer/subject -> collector | `telemetry_query_result` | MUST reference query request ID |
| `rollup_query` | consumer/subject -> collector | `rollup_result` | MUST reference query request ID |
| `eval_submit` | evaluator -> collector | `eval_submit_result` | MUST reference submit request ID |
| `eval_query` | consumer/subject -> collector | `eval_query_result` | MUST reference query request ID |

### 5.2 TELEMETRY_SUBMIT

Rules:
- Core MTI behavior: authenticated caller DID MUST equal `event.reporter`.
- `event.subject` and `event.reporter` MUST be valid DID strings.
- `event.occurred_at` MUST NOT exceed `server_now + 300000`, else reject with `4001`.
- `event.value` MUST be integer; non-integer values MUST be rejected with `4001`.
- Collector MUST apply idempotency rules before persistence.

### 5.3 TELEMETRY_QUERY

Rules:
- Query filters MAY include `subject`, optional `event_type`, optional `category`, optional pagination.
- `view` defaults to `"self"` when absent.
- If `view` is present and not `"self"`, request MUST be rejected with `4001`.
- Non-`"self"` `view` on telemetry raw query is a semantic validation failure and MUST map to `4001` (not parse-level `1001`).
- MTI raw-event query authorization: authenticated caller DID MUST equal query `subject`; otherwise reject with `3001`.
- `limit` default is `50` when absent and MUST be in `1..100` when present.
- Result ordering MUST be deterministic: `occurred_at` descending, then `event_id` ascending by raw byte-string lexical order (unsigned byte-wise compare).
- `cursor` MUST be treated as opaque by clients.
- If `cursor` is present, requester MUST keep `subject`, `event_type`, and `category` unchanged from original query page; `limit` MAY change.
- Invalid/expired cursor, or cursor/query-context mismatch, MUST be rejected with `4001`.

### 5.4 ROLLUP_QUERY

Rules:
- `rollup_query` requires `subject`, `event_type`, `interval`, `window_start`, and `window_end`.
- `view` defaults to `"self"` when absent.
- `window_end` MUST be greater than `window_start`; otherwise reject with `4001`.
- (`window_end - window_start`) MUST be `<= 7776000000` (90 days).
- `window_start` and `window_end` MUST align to interval boundaries (Section 5.6) or reject with `4001`.
- Computed bucket count `((window_end - window_start) / interval_ms)` MUST be in `1..10000`; otherwise reject with `4001`.
- `view="self"` requires authenticated caller DID == query `subject`.
- `view="public"` is optional profile; if unsupported or unauthorized by policy, reject with `3001`.

### 5.5 EVAL_SUBMIT and EVAL_QUERY

`eval_submit` rules:
- Core MTI behavior: authenticated caller DID MUST equal `evaluation.reporter`.
- `evaluation.subject` and `evaluation.reporter` MUST be valid DID strings.
- `score` MUST be in `0..100`; out-of-range MUST be rejected with `4001`.
- `executed_at` MUST NOT exceed `server_now + 300000`.
- Idempotency key is (`reporter`, `eval_id`) with conflict behavior from Section 4.4.

`eval_query` rules:
- MTI query authorization: authenticated caller DID MUST equal query `subject`; otherwise reject with `3001`.
- `limit` default is `20` and MUST be in `1..100` when present.
- Result ordering MUST be deterministic: `executed_at` descending, then `eval_id` ascending by raw byte-string lexical order (unsigned byte-wise compare).

### 5.6 Deterministic Bucketing and Aggregation

For each `rollup_query(subject, event_type, interval, window_start, window_end, view)`:
1. Map interval to milliseconds:
   - `1m = 60000`, `5m = 300000`, `1h = 3600000`, `1d = 86400000`.
2. Validate boundary alignment:
   - `window_start % interval_ms == 0`
   - `window_end % interval_ms == 0`
3. Build eligible sample set `S`:
   - `event.subject == subject`
   - `event.event_type == event_type`
   - `window_start <= event.occurred_at < window_end`
   - `event.value` is integer
   - privacy policy allows sample under requested `view`
4. Partition into fixed buckets:
   - `bucket_index = floor((event.occurred_at - window_start) / interval_ms)`.
5. For each bucket compute integer aggregates:
   - `count`
   - `sum`
   - `min`
   - `max`
   - `avg = trunc_toward_zero(sum / count)` when `count > 0`
6. Empty-bucket rule:
   - `count=0`, `sum=0`, and `min/max/avg = null`.

All arithmetic MUST be deterministic integer arithmetic.

### 5.7 Telemetry Body CDDL

```cddl
telemetry-submit-body = {
  "tel_v": 1,
  "op": "telemetry_submit",
  "event": telemetry-event,
  ? "session": telemetry-session-context
}

telemetry-submit-result-body = {
  "tel_v": 1,
  "op": "telemetry_submit_result",
  "event_id": event-id,
  "status": "accepted" / "duplicate" / "rejected",
  ? "reason": tstr,
  ? "session": telemetry-session-context
}

telemetry-query-body = {
  "tel_v": 1,
  "op": "telemetry_query",
  "subject": did,
  ? "event_type": tstr,
  ? "category": "transport" / "execution" / "capability" / "payment" / "policy" / "quality",
  ? "cursor": tstr,
  ? "limit": uint,
  ? "view": raw-query-view,
  ? "session": telemetry-session-context
}

telemetry-query-result-body = {
  "tel_v": 1,
  "op": "telemetry_query_result",
  "subject": did,
  "events": [* telemetry-event],
  ? "next_cursor": tstr / null,
  "has_more": bool,
  ? "session": telemetry-session-context
}

rollup-query-body = {
  "tel_v": 1,
  "op": "rollup_query",
  "subject": did,
  "event_type": tstr,
  "interval": rollup-interval,
  "window_start": unix-ms,
  "window_end": unix-ms,
  ? "view": rollup-query-view,
  ? "session": telemetry-session-context
}

rollup-bucket = {
  "bucket_start": unix-ms,
  "bucket_end": unix-ms,
  "count": uint,
  "sum": int,
  "min": int / null,
  "max": int / null,
  "avg": int / null
}

rollup-result-body = {
  "tel_v": 1,
  "op": "rollup_result",
  "subject": did,
  "event_type": tstr,
  "interval": rollup-interval,
  "window_start": unix-ms,
  "window_end": unix-ms,
  "buckets": [* rollup-bucket],
  ? "session": telemetry-session-context
}

eval-submit-body = {
  "tel_v": 1,
  "op": "eval_submit",
  "evaluation": evaluation-result,
  ? "session": telemetry-session-context
}

eval-submit-result-body = {
  "tel_v": 1,
  "op": "eval_submit_result",
  "eval_id": eval-id,
  "status": "accepted" / "duplicate" / "rejected",
  ? "reason": tstr,
  ? "session": telemetry-session-context
}

eval-query-body = {
  "tel_v": 1,
  "op": "eval_query",
  "subject": did,
  ? "scenario": tstr,
  ? "cursor": tstr,
  ? "limit": uint,
  ? "session": telemetry-session-context
}

eval-query-result-body = {
  "tel_v": 1,
  "op": "eval_query_result",
  "subject": did,
  "evaluations": [* evaluation-result],
  ? "next_cursor": tstr / null,
  "has_more": bool,
  ? "session": telemetry-session-context
}

tel-capability-id = "org.agentries.telemetry.workflow:1.0.0"

tel-cap-invoke-params =
  telemetry-submit-body /
  telemetry-query-body /
  rollup-query-body /
  eval-submit-body /
  eval-query-body

tel-cap-result-success =
  telemetry-submit-result-body /
  telemetry-query-result-body /
  rollup-result-body /
  eval-submit-result-body /
  eval-query-result-body
```

### 5.8 CAP-Exposed Telemetry Profile Mapping

This section defines RFC 010 capability interoperability baseline for RFC 004 invocation path.

Capability identity:
- `id = "org.agentries.telemetry.workflow:1.0.0"`.

Rules:
- CAP path MUST target capability ID above.
- `CAP_INVOKE.params` MUST contain exactly one RFC 010 request body with `tel_v = 1`.
- Allowed CAP request ops: `telemetry_submit`, `telemetry_query`, `rollup_query`, `eval_submit`, `eval_query`.
- `CAP_RESULT(status="success").result` MUST contain corresponding RFC 010 response body.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005.
- Invalid/unsupported delegation evidence in CAP path MUST fail with `3004`.
- Direct profile `REQUEST`/`RESPONSE` direction rules MUST NOT be applied to CAP envelope types.
- Providers supporting CAP path MUST publish an RFC 004-compliant descriptor for `org.agentries.telemetry.workflow:1.0.0`.
- Session context source-of-truth in CAP path is RFC 004 envelope extension (`CAP_INVOKE.body.session`, `CAP_RESULT.body.session`) with semantics governed by RFC 006.
- `CAP_INVOKE.params.session` and `CAP_RESULT.result.session` MAY exist for payload-level compatibility, but if both payload and envelope session context are present, they MUST be semantically equivalent; mismatch MUST fail with `4001`.
- Pre-execution rejection in CAP path MUST return AMP `ERROR` per RFC 004 Section 7.2.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.

---

## 6. State Machines

### 6.1 Telemetry Event Lifecycle

```text
NEW
  -> VALIDATED           (schema/range/auth checks pass)
VALIDATED
  -> STORED              (persisted successfully)
  -> REJECTED            (policy/validation failure)
STORED
  -> EXPIRED             (retention policy elapsed)
STORED / EXPIRED / REJECTED
  -> TERMINAL
```

### 6.2 Evaluation Record Lifecycle

```text
NEW
  -> ACCEPTED            (valid eval_submit)
  -> REJECTED            (invalid payload/policy)
ACCEPTED
  -> SUPERSEDED          (same scenario newer accepted run by policy)
ACCEPTED / SUPERSEDED / REJECTED
  -> TERMINAL
```

---

## 7. Error Handling and Retry

Deterministic mapping:

| Condition | Code | Retry |
|-----------|------|-------|
| Malformed telemetry/evaluation body | `1001` | No |
| Malformed `body.session` context object (type/shape/size) | `1001` | No |
| Unsupported `tel_v` | `1004` | No |
| Unauthorized reporter/subject action | `3001` | No |
| `telemetry_query` caller DID differs from query `subject` (MTI raw-event profile) | `3001` | No |
| `rollup_query.view="public"` not enabled/authorized by policy | `3001` | No |
| Invalid/unsupported delegation evidence in CAP telemetry path (`CAP_INVOKE.body.delegation`) | `3004` | No |
| `body.delegation` present on non-CAP telemetry message | `4001` | No |
| Invalid `op`/`typ` direction or `reply_to` correlation | `4001` | No |
| Invalid event/eval field ranges or timestamp constraints | `4001` | No |
| `telemetry_query.view` present and not `"self"` | `4001` | No |
| `rollup_query` bucket count outside `1..10000` | `4001` | No |
| Invalid rollup window/interval alignment | `4001` | No |
| Invalid/expired cursor or cursor/query-context mismatch | `4001` | No |
| Duplicate key with conflicting payload | `4001` | No |
| Telemetry store unavailable | `5002` | Yes |
| Internal telemetry engine failure | `5001` | Yes |

Retry guidance:
- `5002` and `5001` MAY be retried with bounded backoff.
- `100x/3001/3004/4001` SHOULD NOT be retried without payload/policy changes.

Optional profile note:
- Implementations MAY perform a pre-query subject DID existence check and return `2001 RECIPIENT_NOT_FOUND` when that optional profile is enabled.
- This optional profile SHOULD be disabled by default and SHOULD be enabled only in trusted deployments where subject-existence disclosure is acceptable.

---

## 8. Versioning and Compatibility

Version dimensions:
- AMP envelope `v` remains governed by RFC 001.
- Telemetry body version is `tel_v`, fixed at `1` in this revision.

Compatibility rules:
- Unsupported `tel_v` MUST be rejected with `1004`.
- Unknown optional fields MAY be ignored unless policy/security-sensitive.
- `body.delegation` MUST NOT be treated as ignorable optional input on non-CAP telemetry paths; delegation carriage/validation remains governed by RFC 001 Section 4.6 and RFC 005.
- Backward-compatible evolution MUST use optional-field extension only.

---

## 9. Security Considerations

- Implementations MUST verify message signatures before processing telemetry mutations.
- Reporter identity MUST be bound to authenticated principal policy (RFC 002).
- Telemetry results MUST NOT bypass authorization, delegation, signature, or policy checks.
- Collectors SHOULD rate-limit submissions per reporter and per principal.
- External evidence/artifact links SHOULD be treated as untrusted input.
- Query paths SHOULD avoid leaking existence/profiling details through error text.

---

## 10. Privacy Considerations

- Telemetry may include operationally sensitive metadata; producers SHOULD minimize high-cardinality personal identifiers.
- Public query paths SHOULD expose rollups instead of raw event payloads.
- Implementations SHOULD define retention windows per privacy class.
- Evaluation notes/artifacts may contain proprietary prompts or outputs and SHOULD be redacted or hashed before submission.

---

## 11. Implementation Checklist

- Implement deterministic telemetry dispatch (`tel_v` + `op`).
- Enforce (`reporter`, `event_id`) and (`reporter`, `eval_id`) idempotency/conflict handling.
- Enforce strict reporter-binding MTI baseline for submit operations.
- Enforce MTI raw-event query binding (`telemetry_query.caller_did == subject`).
- Enforce rollup interval/window validation and deterministic bucketing rules.
- Enforce score range and timestamp constraints for `eval_submit`.
- Enforce privacy redaction/class rules for public views.
- Enforce RFC 006 session context validation when `body.session` is present.
- If CAP path is supported, implement Section 5.8 fixed capability mapping.
- Apply deterministic Section 7 error mapping.
- Preserve audit logs for submit/query/rollup paths.

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
- RFC 009: Reputation & Trust Signals
- RFC 011: Multi-Agent Coordination & Group Messaging
- DID Core (W3C Recommendation)

---

## Appendix A. Minimal Test Vectors

### A.1 TELEMETRY_SUBMIT Positive

Input:
- Valid `telemetry_submit` with bounded fields and valid reporter binding.

Expected:
- Event accepted and persisted.

### A.2 TELEMETRY_SUBMIT Duplicate Idempotent Positive

Input:
- Same (`reporter`, `event_id`) and byte-identical event resubmitted.

Expected:
- Deterministic idempotent success.

### A.3 TELEMETRY_SUBMIT Duplicate Conflict Negative

Input:
- Same (`reporter`, `event_id`) with conflicting payload.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.4 TELEMETRY_QUERY Self Positive

Input:
- `telemetry_query.subject` equals authenticated caller DID.

Expected:
- Query accepted with deterministic ordering.

### A.5 TELEMETRY_QUERY Caller-Subject Mismatch Negative

Input:
- `telemetry_query.subject` differs from authenticated caller DID in MTI profile.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.6 ROLLUP_QUERY Deterministic Positive

Input:
- Fixed event set, `interval=5m`, aligned window, explicit view policy.

Expected:
- Deterministic bucket outputs (`count/sum/min/max/avg`) match Section 5.6.

### A.7 ROLLUP_QUERY Misaligned Window Negative

Input:
- `window_start` or `window_end` not aligned with requested interval boundary.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.8 Unsupported tel_v Negative

Input:
- `tel_v = 2`.

Expected:
- Reject with `1004 UNSUPPORTED_VERSION`.

### A.9 Byte-Level Error Code Checks

Input:
- Unauthorized action mapped to `3001`.
- Delegation invalid mapped to `3004`.
- Invalid payload/range mapped to `4001`.
- Unsupported version mapped to `1004`.

Expected:
- `3001` CBOR uint encoding bytes: `19 0b b9`.
- `3004` CBOR uint encoding bytes: `19 0b bc`.
- `4001` CBOR uint encoding bytes: `19 0f a1`.
- `1004` CBOR uint encoding bytes: `19 03 ec`.

### A.10 EVAL_SUBMIT and EVAL_QUERY Positive

Input:
- Valid `eval_submit`, then `eval_query` by subject DID.

Expected:
- Evaluation accepted and query returns deterministic ordered records.

### A.11 CAP Telemetry Profile Positive

Input:
- `CAP_INVOKE` targets `id = org.agentries.telemetry.workflow:1.0.0`.
- `CAP_INVOKE.params` contains valid RFC 010 submit/query body.

Expected:
- RFC 004 capability validation passes.
- Telemetry semantics execute with `CAP_RESULT(status="success")`.

### A.12 CAP Delegation Evidence Invalid Negative

Input:
- CAP telemetry invocation contains invalid `CAP_INVOKE.body.delegation`.

Expected:
- Reject with `3004 DELEGATION_INVALID`.

### A.13 Session Context Shape Invalid Negative

Input:
- Session-scoped telemetry request carries malformed `body.session`.

Expected:
- Reject with `1001 INVALID_MESSAGE` per RFC 006 parsing rules.

### A.14 Non-CAP Delegation Field Negative

Input:
- Direct telemetry request (`REQUEST/RESPONSE` path) carries `body.delegation`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.15 TELEMETRY_QUERY Invalid View Negative

Input:
- `telemetry_query.view = "public"` on direct raw-event query path.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.16 CAP Session Context Mismatch Negative

Input:
- CAP path request includes both `CAP_INVOKE.body.session` and `CAP_INVOKE.params.session` with semantically inconsistent values.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.17 ROLLUP_QUERY Excessive Bucket Count Negative

Input:
- `rollup_query` uses aligned window/interval that computes more than `10000` buckets.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.18 EVAL_SUBMIT Invalid DID Negative

Input:
- `eval_submit.evaluation.subject` or `eval_submit.evaluation.reporter` is not a valid DID string.

Expected:
- Reject with `4001 BAD_REQUEST`.

---

## Appendix B. Open Questions

No open questions in this revision.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-06 | Proposal | Nowa | Initial outline for telemetry and evaluation concepts |
| 2026-02-08 | 0.1 | Nowa | Rewrote RFC 010 into normative draft structure with conformance profiles, boundary contracts, deterministic telemetry/evaluation semantics, CDDL schemas, CAP interop mapping, error mapping, and minimal test vectors |
| 2026-02-08 | 0.2 | Nowa | Added RFC 011 boundary alignment, split raw-event vs rollup `view` model, made query ordering byte-lex explicit for `event_id`/`eval_id`, removed non-deterministic `2001` from MTI mapping, and added vectors A.15-A.16 |
| 2026-02-08 | 0.3 | Nowa | Added `rollup_query` default `view=self`, added deterministic bucket-count upper bound (`1..10000`), required DID validity checks for `eval_submit.subject/reporter`, and added vectors A.17-A.18 |
| 2026-02-08 | 0.4 | Nowa | Aligned header/version state with changelog and README, resolved `telemetry_query.view` CDDL-vs-error mapping conflict by making non-`self` a deterministic semantic `4001`, and tightened optional `2001` profile guidance to default-off trusted deployments |
| 2026-02-08 | 0.5 | Nowa | Clarified MTI reporter-binding conformance language so submit binding explicitly covers both `telemetry_submit.event.reporter` and `eval_submit.evaluation.reporter` |
| 2026-02-10 | 0.5 | Nowa | Synchronized document metadata (`Updated`) and repository gate-status references for audit consistency |
