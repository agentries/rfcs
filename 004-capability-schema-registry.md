# RFC 004: Capability Schema Registry & Compatibility

**Status**: Draft
**Authors**: Ryan Cooper, Nowa
**Created**: 2026-02-04
**Updated**: 2026-02-07
**Version**: 0.6

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (carrier only)
- RFC 003: Relay and Store-and-Forward (delivery/persistence)
- RFC 006: Session Protocol (state + recovery)
- RFC 008: Agent Discovery and Directory

---

## Abstract

This RFC defines a normative capability registry model and compatibility negotiation rules for AMP capability messaging (`CAP_QUERY`, `CAP_DECLARE`, `CAP_INVOKE`, `CAP_RESULT`). It standardizes capability identifiers, schema metadata, version selection, and validation/error behavior so agents can interoperate without out-of-band per-vendor contracts.

---

## Table of Contents

1. Scope and Non-Goals
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Capability Identifier and Schema Metadata
4.1 Identifier and Namespace Format
4.2 Schema Descriptor Object
4.3 Namespace Governance
4.4 Semver Range Grammar (Normative)
5. Registry Model and Publication
5.1 Registry Sources
5.2 Integrity, Caching, and Freshness
5.3 Offline Registry Profile
6. Capability Discovery and Negotiation Semantics
6.1 CAP_QUERY
6.2 CAP_DECLARE
6.3 Negotiation Algorithm (Deterministic)
6.4 Discovery and Negotiation CDDL
6.5 Pagination and Result Ordering
7. Capability Invocation Semantics
7.1 CAP_INVOKE and CAP_RESULT
7.2 Validation Order and Error Mapping
7.3 Invocation State Machine
7.4 Invocation CDDL
8. Compatibility and Fallback Policy
9. Error Handling and Retry
10. Versioning and Compatibility
11. Security Considerations
12. Privacy Considerations
13. Implementation Checklist
14. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions

---

## 1. Scope and Non-Goals

This RFC defines:
- Capability identifier format and namespace ownership rules.
- Capability schema descriptor format (input/output schema metadata).
- Discovery and compatibility negotiation semantics for `CAP_QUERY` and `CAP_DECLARE`.
- Invocation validation semantics for `CAP_INVOKE` and `CAP_RESULT`.
- Deterministic error mapping for capability-related failures.

This RFC does not define:
- AMP base envelope, signatures, encryption, or core type code allocation (RFC 001).
- Transport binding behavior, HTTP/WS/TCP wrappers, or principal binding (RFC 002).
- Queueing, retry persistence, or federation custody transfer (RFC 003).
- Session state persistence protocol (RFC 006).
- Directory indexing/ranking policy or contact approval policy (RFC 008).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 message envelope semantics for all capability message types.
- Implements all required rules for each claimed profile below.
- Applies validation and error mapping in Section 7.2 and Section 9.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Capability Name | Stable symbolic name, e.g. `org.agentries.code-review`. |
| Capability ID | Name plus concrete version, e.g. `org.agentries.code-review:2.1.0`. |
| Schema Descriptor | Metadata describing versioned input/output contract and compatibility claims. |
| Registry Source | Location that serves schema descriptors and schema documents. |
| Negotiated Version | Concrete version chosen for one invocation/session path. |
| Compatibility Match | Result where requester and provider share at least one executable version. |

### 2.2 Role Profiles and MTI Requirements

`Core Agent Profile`:
- MUST parse and validate `CAP_DECLARE`, `CAP_INVOKE`, `CAP_RESULT`.
- MUST support exact-version invocation by `id`.
- MUST apply `4002/4003/4004` mapping from Section 9.

`Capability Provider Profile`:
- MUST publish at least one schema descriptor per supported capability.
- MUST validate invocation params against the negotiated input schema.
- MUST return exactly one terminal `CAP_RESULT` for each accepted `CAP_INVOKE`.

`Registry Publisher Profile`:
- MUST serve schema descriptors with deterministic version keys.
- MUST provide content integrity metadata (hash and algorithm) per schema artifact.
- SHOULD provide explicit freshness metadata (`etag`, `updated_at`, `max_age_s`).

`Offline Registry Profile`:
- MUST allow schema resolution without network access.
- MUST provide local artifact lookup metadata (`bundle_id` + `artifact_key`) for each schema-ref.
- MUST preserve the same hash verification model as online mode before schema validation.
- MUST fail closed when referenced local artifacts are missing or hash verification fails.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- RFC 001 defines message type codes (`0x20`-`0x23`) and base envelope fields.
- RFC 004 defines capability body schemas, compatibility semantics, and capability-specific validation rules.
- Capability validation failures MUST map to RFC 001 client errors `4002`, `4003`, `4004`.

With RFC 002:
- Capability messages are opaque AMP payload bytes to transport bindings.
- Transport success/failure MUST NOT alter capability compatibility semantics.

With RFC 003:
- Relays MUST treat capability bodies as opaque payload and MUST NOT rewrite capability descriptors or invocation params.
- Store-and-forward redelivery MUST preserve invocation body bytes unchanged.

With RFC 006:
- Session protocol MAY pin negotiated capability IDs for session scope.
- RFC 006 session recovery MUST NOT change negotiated capability version unless renegotiation occurs.

With RFC 008:
- Discovery indexes MAY cache capability summaries, but canonical compatibility decisions use RFC 004 descriptors/negotiation payloads.
- RFC 008 directory metadata MUST NOT override namespace ownership or schema integrity rules in RFC 004.

---

## 4. Capability Identifier and Schema Metadata

### 4.1 Identifier and Namespace Format

Canonical forms:

```
capability-name = <reverse-domain namespace> "." <slug>
capability-id   = <capability-name> ":" <semver>
```

Examples:
- `org.agentries.code-review`
- `org.agentries.code-review:2.1.0`
- `com.acme.risk-evaluator:1.4.2`

Rules:
- Namespace prefix MUST use reverse-domain form to avoid collisions.
- Capability name comparison is case-sensitive byte comparison.
- Version comparison uses semantic version ordering (`major.minor.patch`).
- `major` incompatibility MUST be treated as breaking unless explicitly bridged by provider policy.

### 4.2 Schema Descriptor Object

Each versioned capability declaration MUST include:
- Concrete `id` (`name:semver`).
- `name` and `version` fields consistent with `id`.
- Input and output schema references.
- Compatibility metadata (`supported_ranges` and optional `deprecated_ranges`).
- Integrity metadata for schema references.

CDDL:

```cddl
semver = tstr
semver-range = tstr
capability-name = tstr
capability-id = tstr
hash-alg = "sha-256" / "sha-512"

schema-ref-online = {
  "uri": tstr,
  ? (
    "bundle_id": tstr,
    "artifact_key": tstr
  ),
  "hash_alg": hash-alg,
  "hash": bstr,
  ? "media_type": tstr,
  ? "updated_at": tstr
}

schema-ref-offline = {
  ? "uri": tstr,
  "bundle_id": tstr,
  "artifact_key": tstr,
  "hash_alg": hash-alg,
  "hash": bstr,
  ? "media_type": tstr,
  ? "updated_at": tstr
}

schema-ref = schema-ref-online / schema-ref-offline

capability-descriptor = {
  "id": capability-id,
  "name": capability-name,
  "version": semver,
  "input_schema": schema-ref,
  "output_schema": schema-ref,
  ? "supported_ranges": [* semver-range],
  ? "deprecated_ranges": [* semver-range],
  ? "notes": tstr
}
```

Consistency rules:
- `id` MUST equal `name + ":" + version`.
- `input_schema.hash` and `output_schema.hash` MUST match retrieved schema bytes.
- If `hash_alg = "sha-256"`, `hash` MUST be exactly 32 bytes.
- If `hash_alg = "sha-512"`, `hash` MUST be exactly 64 bytes.
- If `supported_ranges` is absent, only exact `version` is supported.
- Each `schema-ref` MUST include either:
  - `uri`, or
  - both `bundle_id` and `artifact_key`.
- In `Offline Registry Profile`, `bundle_id` and `artifact_key` are REQUIRED and `uri` is OPTIONAL.

### 4.3 Namespace Governance

- Namespace owner is the controller of the namespace domain.
- Namespace owner is responsible for collision prevention and version policy within that namespace.
- No global central allocator is required.
- Cross-vendor interoperability SHOULD prefer public namespaces with stable change policy.
- Capability namespaces are not subject to centralized registration in RFC 001 Section 17.3.

### 4.4 Semver Range Grammar (Normative)

To reduce interop drift, this RFC constrains `semver-range` to the following subset:
- Exact: `x.y.z`
- Comparator set (space-separated AND): e.g. `>=1.2.0 <2.0.0`

Rules:
- Implementations MUST support exact and comparator-set forms above.
- OR expressions (e.g., `||`) and wildcard shorthand (e.g., `1.x`) are NOT MTI.
- Pre-release ordering follows Semantic Versioning 2.0.0 rules.

---

## 5. Registry Model and Publication

### 5.1 Registry Sources

Registry source options (all valid):
- DID service endpoint owned by namespace controller.
- HTTPS catalog endpoint.
- Signed local/offline bundle.

Provider MUST expose at least one source that can resolve descriptor by capability ID.

Recommended endpoint pattern (informative):

```
GET /cap-registry/{capability-name}/{version}/descriptor.cbor
GET /cap-registry/{capability-name}/{version}/input.schema.json
GET /cap-registry/{capability-name}/{version}/output.schema.json
```

### 5.2 Integrity, Caching, and Freshness

- Consumers MUST validate descriptor/schema hash before using schema for validation.
- Consumers SHOULD cache by `id + hash` key.
- Consumers SHOULD treat descriptor as immutable when `hash` unchanged.
- If freshness metadata exists, consumers SHOULD refresh on expiry.

### 5.3 Offline Registry Profile

This profile is normative for air-gapped/private-network deployments.

Resolution model:
- Consumer resolves schema bytes from local bundle storage using (`bundle_id`, `artifact_key`).
- Consumer computes hash over local bytes and verifies `hash_alg`/`hash`.
- Only after hash verification succeeds may schema validation be executed.

Behavior requirements:
- Implementations MUST NOT require live HTTP fetch when `Offline Registry Profile` is enabled.
- If both `uri` and local bundle locators are present, implementations MAY prefer local locators.
- On local artifact miss, unreadable artifact, or hash mismatch, implementations MUST fail with `5002 UNAVAILABLE` by default.
- `4004 SCHEMA_VIOLATION` is only valid after schema bytes have been successfully loaded and verified, and invocation params fail schema validation.

---

## 6. Capability Discovery and Negotiation Semantics

### 6.1 CAP_QUERY

Purpose:
- Ask a peer/registry source for capability descriptors matching a filter.

Rules:
- Query filter MUST include either `capability` or legacy alias `type`.
- If both exist, `capability` takes precedence.
- Optional `version` is a semver range.
- If `order` is absent, provider MUST apply `newest-first`.
- If `cursor` is present, requester MUST keep `order` and `filter` unchanged from the original query page; `limit` MAY change.
- If no descriptor matches the requested capability name/alias, provider MUST return `4002 CAPABILITY_NOT_FOUND` (instead of empty `CAP_DECLARE`).
- If capability name matches but no descriptor satisfies requested `version` range, provider MUST return `4003 VERSION_MISMATCH`.

### 6.2 CAP_DECLARE

Purpose:
- Return one or more capability descriptors.

Rules:
- `capabilities` array MUST NOT be empty.
- Each descriptor MUST satisfy Section 4.2 consistency rules.
- Provider SHOULD include newest compatible versions first unless request ordering explicitly overrides this (Section 6.5 `order`).
- When `CAP_DECLARE` is a response to `CAP_QUERY`, it MUST set envelope `reply_to` to the request message `id`.

### 6.3 Negotiation Algorithm (Deterministic)

Inputs:
- Requester: preferred concrete version (optional), acceptable ordered list (optional), semver range (optional).
- Provider: supported concrete versions.

Selection order:
1. If preferred exact version is supported, select it.
2. Else select first requester acceptable version that provider supports.
3. Else if range is present, select highest provider version in range.
4. Else negotiation fails with `4003 VERSION_MISMATCH`.

Additional rules:
- Negotiation output MUST be a concrete `capability-id`.
- If requester sends `id` plus (`capability`, `version`) and values disagree, reject `4001`.
- Negotiation results MAY be cached per peer for short duration, but sender MUST tolerate stale cache mismatch and retry with fresh query.
- Range parsing MUST follow Section 4.4 grammar subset.

### 6.4 Discovery and Negotiation CDDL

```cddl
cap-filter = (
  {
    "capability": capability-name,
    ? "type": tstr,
    ? "version": semver-range
  } /
  {
    "type": tstr,
    ? "version": semver-range
  }
)

cap-query-body = {
  "filter": cap-filter,
  ? "limit": uint,
  ? "cursor": tstr,
  ? "order": "newest-first" / "oldest-first"
}

cap-declare-body = {
  "capabilities": [+ capability-descriptor]
}

cap-negotiate-hints = {
  ? "preferred": semver,
  ? "acceptable": [* semver],
  ? "range": semver-range
}
```

### 6.5 Pagination and Result Ordering

For `CAP_QUERY` responses spanning multiple pages:
- Provider MUST use stable deterministic ordering.
- If request omits `order`, effective order is `newest-first`.
- Sort key MUST be:
  - `order = "newest-first"`: `(name ASC, version DESC by SemVer precedence)`
  - `order = "oldest-first"`: `(name ASC, version ASC by SemVer precedence)`
- `cursor` MUST be treated as opaque by clients.
- Invalid/expired cursor, or cursor/query-context mismatch (`filter` or `order` changed), MUST be rejected with `4001 BAD_REQUEST`.
- Cursor page boundaries MUST NOT duplicate or skip records within the same consistent snapshot.

---

## 7. Capability Invocation Semantics

### 7.1 CAP_INVOKE and CAP_RESULT

`CAP_INVOKE` MUST identify target capability by either:
- `id` (recommended), or
- (`capability` or `type`) + (`version` or `negotiate`).

`CAP_RESULT` is terminal for accepted invocation and MUST include:
- `status = "success"` with `result`, or
- `status = "error"` with structured `error`.
- `CAP_RESULT` MUST set envelope `reply_to` to the corresponding `CAP_INVOKE` message `id`.

### 7.2 Validation Order and Error Mapping

For each incoming `CAP_INVOKE`, provider MUST validate in this order:
1. Body shape and required fields.
2. Coarse authorization/policy checks that do not require capability resolution.
3. Capability identity resolution.
4. Capability-scoped authorization/policy checks (if policy depends on resolved capability/version).
5. Version compatibility.
6. Input schema validation.

Authorization sequencing notes:
- Step 2 SHOULD enforce generic caller policy (identity, tenancy, baseline allow/deny).
- Step 4 MUST enforce capability-level ACL/policy when such policy exists.
- If implementation chooses to resolve capability before full auth due to local architecture, denial responses MUST still follow `3001` leakage-minimization rules in this section.

Error mapping:
- Unauthorized caller or policy-denied invocation -> `3001 UNAUTHORIZED`
- Capability unknown -> `4002 CAPABILITY_NOT_FOUND`
- Version unsupported -> `4003 VERSION_MISMATCH`
- Params violate input schema -> `4004 SCHEMA_VIOLATION`
- Malformed/missing required invocation fields -> `4001 BAD_REQUEST`

Behavior rule:
- If invocation is rejected before execution starts, provider SHOULD return AMP `ERROR` with codes above.
- If invocation is accepted and fails during execution, provider MUST return `CAP_RESULT(status="error")`.
- To reduce capability enumeration leakage, unauthorized/policy-denied invocations SHOULD return `3001` without revealing capability/version existence.

### 7.3 Invocation State Machine

Invoker:

```
IDLE
  -> CAP_INVOKE_SENT
CAP_INVOKE_SENT
  -> (optional) ACK/PROCESSING/PROGRESS
  -> CAP_RESULT(success|error) -> DONE
  -> ERROR(3xxx/4xxx/5xxx) -> DONE
```

Executor:

```
RECEIVED
  -> VALIDATING
VALIDATING
  -> REJECTED(ERROR 3xxx/4xxx)
  -> EXECUTING
EXECUTING
  -> (optional) PROCESSING/PROGRESS
  -> CAP_RESULT(success|error) -> DONE
```

### 7.4 Invocation CDDL

```cddl
cap-invoke-by-id = {
  "id": capability-id,
  "params": any,
  ? "timeout_ms": uint
}

cap-invoke-by-name = {
  (
    { "capability": capability-name, ? "type": tstr } /
    { "type": tstr }
  ),
  (
    { "version": semver } /
    { "negotiate": cap-negotiate-hints }
  ),
  "params": any,
  ? "timeout_ms": uint
}

cap-invoke-body = cap-invoke-by-id / cap-invoke-by-name

cap-result-error = {
  "code": uint,
  ? "name": tstr,
  ? "message": tstr,
  ? "details": any
}

cap-result-body = (
  {
    "status": "success",
    "result": any
  } /
  {
    "status": "error",
    "error": cap-result-error
  }
)
```

Schema notes:
- `cap-invoke-body` is structurally constrained to `by-id` or `by-name` forms above.
- If `id` is present, `negotiate` MUST NOT be present; otherwise reject with `4001`.
- Legacy `type` MAY be accepted for backward compatibility; providers SHOULD emit canonical `capability` in responses/logs.

---

## 8. Compatibility and Fallback Policy

Provider compatibility policy MUST be explicit:
- `strict`: only exact version accepted.
- `range`: semver-range negotiation allowed.
- `bridge`: provider applies explicit adapter logic between versions.

Rules:
- `bridge` behavior MUST be documented and deterministic.
- If no documented bridge exists, incompatible major versions MUST fail with `4003`.
- Fallback MUST preserve safety constraints (do not silently drop required input fields).

---

## 9. Error Handling and Retry

This RFC reuses RFC 001 error codes.

| Failure | Code | Retry |
|---------|------|-------|
| CBOR decode / structural envelope failure | `1001` | No |
| Capability body semantic field failure | `4001` | No |
| Unauthorized or policy-denied capability invocation | `3001` | No |
| Capability not found (including `CAP_QUERY` name/alias no-match) | `4002` | No |
| Version mismatch (including `CAP_QUERY` range no-match) | `4003` | No |
| Schema validation failure | `4004` | No |
| Offline artifact missing/hash mismatch | `5002` | Yes |
| Temporary registry unavailable | `5002` | Yes |
| Internal execution failure | `5001` | Yes |
| Invocation timeout | `5003` | Yes |
| Response `reply_to` correlation mismatch | `4001` | No |

Retry guidance:
- `400x` errors SHOULD NOT be retried without request mutation.
- `3001` errors SHOULD NOT be retried without credential/policy change.
- `500x` errors MAY be retried with bounded exponential backoff.
- Repeated `CAP_INVOKE` retries SHOULD preserve the same AMP `msg_id` or explicit idempotency key to avoid duplicate execution.

Correlation handling:
- On response `reply_to` mismatch, receiver MUST mark response invalid and MUST NOT apply it to invocation state.
- Receiver SHOULD emit `ERROR 4001` when a return path exists; otherwise it MAY drop and log the invalid response.

---

## 10. Versioning and Compatibility

This RFC versioning model:
- Descriptor and CDDL objects are versioned by backward-compatible field extension.
- New optional fields MAY be added without breaking existing parsers.
- Removal or semantic redefinition of existing required fields requires new RFC 004 major revision.

Capability lifecycle guidance:
- Providers SHOULD support at least one previous minor version for non-breaking transitions.
- Deprecated ranges SHOULD include a planned removal date in out-of-band docs.
- Offline profile fields (`bundle_id`, `artifact_key`) are backward-compatible optional extensions for online profiles.

---

## 11. Security Considerations

- Schema integrity is critical: consumers MUST verify schema hash before validation.
- Registry source authenticity MUST be anchored in DID trust or HTTPS trust policy.
- Do not trust client-declared compatibility claims without provider-side verification.
- Invocation adapters (`bridge` mode) can create confused-deputy risk; implementations SHOULD log pre/post transformed payload shape.
- Capability invocation authorization (who may call which capability) remains mandatory and is separate from schema compatibility.

---

## 12. Privacy Considerations

- Capability declarations may reveal internal implementation details.
- Providers SHOULD expose least-necessary descriptor metadata.
- Discovery caches SHOULD avoid retaining per-request negotiation traces longer than operational need.
- Error details for schema mismatches SHOULD avoid leaking sensitive schema internals in untrusted contexts.

---

## 13. Implementation Checklist

- Parse and validate all `CAP_*` bodies against Section 6/7 CDDL.
- Implement deterministic negotiation algorithm (Section 6.3).
- Enforce validation order and error mapping (Section 7.2).
- Verify schema descriptor and schema hashes before use.
- Publish capability descriptors with canonical `id` and version consistency.
- Add conformance tests from Appendix A.

---

## 14. References

### 14.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in requirement words
- Semantic Versioning 2.0.0

### 14.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay and Store-and-Forward
- RFC 006: Session Protocol
- RFC 008: Agent Discovery and Directory

---

## Appendix A. Minimal Test Vectors

### A.1 CAP_DECLARE Positive

Input:
- `CAP_DECLARE` with one descriptor `org.agentries.code-review:2.1.0`
- Descriptor `id == name:version`
- Valid input/output schema hashes

Expected:
- Descriptor accepted

### A.2 Negotiation Exact Version Positive

Input:
- Requester preferred `2.1.0`
- Provider supports `2.0.0, 2.1.0`

Expected:
- Negotiated version `2.1.0`

### A.3 Negotiation Fallback Positive

Input:
- Preferred `2.2.0`
- Acceptable list `[2.1.0, 2.0.0]`
- Provider supports `2.0.0, 2.1.0`

Expected:
- Negotiated version `2.1.0`

### A.4 Negotiation Mismatch Negative

Input:
- Range `>=3.0.0 <4.0.0`
- Provider supports `2.x`

Expected:
- `4003 VERSION_MISMATCH`

### A.5 Invocation Schema Positive

Input:
- `CAP_INVOKE` with valid params for negotiated schema

Expected:
- One terminal `CAP_RESULT(status="success")`

### A.6 Invocation Schema Violation Negative

Input:
- Missing required field in `params`

Expected:
- `4004 SCHEMA_VIOLATION`

### A.7 Legacy Alias Compatibility

Input:
- `CAP_QUERY` uses `type` without `capability`

Expected:
- Query accepted
- Resolver maps `type` to canonical capability name

### A.8 Identity Mismatch Negative

Input:
- `CAP_INVOKE` includes `id=org.agentries.code-review:2.1.0`
- Also includes `capability=org.agentries.translate`, `version=1.0.0`

Expected:
- `4001 BAD_REQUEST`

### A.9 Unauthorized Capability Probe Negative

Input:
- Caller lacks authorization for capability invocation

Expected:
- `3001 UNAUTHORIZED`
- Error response does not reveal whether capability/version exists

### A.10 Offline Registry Positive

Input:
- `CAP_DECLARE` descriptor provides `bundle_id` + `artifact_key` + hash metadata (no `uri`)
- Local bundle contains matching schema bytes

Expected:
- Descriptor accepted in `Offline Registry Profile`
- Invocation schema validation succeeds without network fetch

### A.11 Offline Artifact Missing Negative

Input:
- `CAP_DECLARE` references local `bundle_id` + `artifact_key`
- Local artifact is missing or hash verification fails

Expected:
- `5002 UNAVAILABLE`

### A.12 CAP_RESULT Correlation Negative

Input:
- Provider returns `CAP_RESULT` with `reply_to` not equal to the triggering `CAP_INVOKE.id`

Expected:
- Response rejected as `4001 BAD_REQUEST` at protocol handler boundary

### A.13 CAP_DECLARE Correlation Negative

Input:
- Provider returns `CAP_DECLARE` as response to `CAP_QUERY` but without correct `reply_to`

Expected:
- Response rejected as `4001 BAD_REQUEST` at protocol handler boundary

### A.14 CAP_QUERY Pagination Stability Positive

Input:
- `CAP_QUERY` with `limit=2`, `order="newest-first"` over 5 descriptors
- Follow-up pages use returned opaque `cursor`

Expected:
- Deterministic order by `(name ASC, version DESC)`
- No duplicates/skips across pages

### A.15 CAP_QUERY Invalid Cursor Negative

Input:
- `CAP_QUERY` with malformed/expired `cursor`

Expected:
- `4001 BAD_REQUEST`

### A.16 CAP_INVOKE Identity Form Conflict Negative

Input:
- `CAP_INVOKE` includes `id` and also `negotiate`

Expected:
- `4001 BAD_REQUEST`

### A.17 CAP_QUERY No-Match Mapping Negative

Input:
- `CAP_QUERY.filter.capability = org.agentries.nonexistent`

Expected:
- `4002 CAPABILITY_NOT_FOUND`
- Provider does not return empty `CAP_DECLARE`

---

## Appendix B. Open Questions

- Should RFC 004 require a single canonical schema language (JSON Schema only) or allow multiple schema types with explicit `media_type` negotiation?
- Should capability descriptor signatures be mandatory in RFC 004 or deferred to a later trust-profile RFC?
- Should negotiation cache hints (`max_age_s`) be standardized in message body to reduce repeated queries?
