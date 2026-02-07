# RFC 005: Delegation Credentials & Authorization

**Status**: Draft
**Authors**: Ryan Cooper, Nowa
**Created**: 2026-02-04
**Updated**: 2026-02-07
**Version**: 0.34

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)
- RFC 004: Capability Schema Registry & Compatibility

**Related:**
- RFC 002: Transport Bindings (principal binding)
- RFC 003: Relay and Store-and-Forward (delivery behavior)
- RFC 006: Session Protocol (session-scoped delegation cache)
- RFC 008: Agent Discovery and Directory (delegation service discovery)

---

## Abstract

This RFC defines delegation credentials, chain validation, revocation, and authorization decision rules for AMP. It standardizes how one DID-authorized agent delegates a bounded scope to another DID, and how verifiers make deterministic allow/deny decisions using signed delegation artifacts.

---

## Table of Contents

1. Scope and Non-Goals
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Delegation Credential Model
4.1 Credential Fields and Semantics
4.2 Scope Model
4.3 Validity and Audience
4.4 Credential CDDL
4.5 Signature Algorithm Profile (MTI)
5. Delegation Protocol Semantics
5.1 DELEG_GRANT
5.2 DELEG_REVOKE
5.3 DELEG_QUERY
5.4 Message Body CDDL
6. Authorization Evaluation Algorithm
6.1 Validation Order (Normative)
6.2 Chain Narrowing Rules
6.3 Revocation Checks
6.4 Decision Output and Audit
7. Revocation Publication Model
7.1 Revocation Object
7.2 Revocation Sources and Status Object
7.3 Caching and Freshness
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
- Delegation credential payload semantics and signature requirements.
- Delegation chain verification and scope narrowing rules.
- Revocation object format and revocation check behavior.
- Authorization decision mapping for delegated requests.
- Semantics for RFC 001 delegation message types (`DELEG_GRANT`, `DELEG_REVOKE`, `DELEG_QUERY`).

This RFC does not define:
- AMP envelope, transport framing, or relay persistence behavior (RFC 001/002/003).
- Capability schema validation semantics (RFC 004).
- Session lifecycle semantics (RFC 006).
- Directory ranking, visibility, or reputation policy (RFC 008/009).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope and signature semantics.
- Applies validation order in Section 6.1.
- Uses error mapping in Section 8.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Delegator | DID that grants authority. |
| Delegate | DID that receives authority. |
| Delegation credential | Signed artifact describing granted scope and constraints. |
| Delegation chain | Ordered list of credentials where each link delegates to the next actor. |
| Effective scope | Intersection of all chain scopes after narrowing. |
| Revocation record | Signed artifact that invalidates a delegation ID. |
| Authorization target | Requested tuple (`capability`, `action`, `resource`) evaluated against delegation scope. |

### 2.2 Role Profiles and MTI Requirements

`Delegation Consumer Profile`:
- MUST support verification of delegation chains up to depth 3 (at minimum capability baseline).
- MUST verify delegation credential and revocation signatures with MTI algorithm profile in Section 4.5.
- MUST enforce scope narrowing and `allow_subdelegation` semantics.
- MUST enforce temporal validity and revocation checks.
- MUST map errors per Section 8.

`Delegation Issuer Profile`:
- MUST issue signed credentials with deterministic, canonical payload fields.
- MUST ensure issuer DID equals credential `delegator`.
- MUST sign credential artifacts with MTI algorithm profile in Section 4.5.
- MUST support signed revocation publication for issued delegation IDs.

`Revocation Publisher Profile`:
- MUST publish signed revocation records by delegation ID.
- MUST provide freshness metadata (`updated_at`, `max_age_s`) or equivalent out-of-band policy.
- MUST preserve append-only auditability for revocation events.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- RFC 001 defines message type codes `DELEG_GRANT (0x50)`, `DELEG_REVOKE (0x51)`, `DELEG_QUERY (0x52)`.
- RFC 005 defines delegation body semantics and verification rules.
- Delegation semantic authorization failures MUST map to RFC 001 `3004` or `3001`.
- Structural/version/availability failures MAY map to RFC 001 `1xxx/4xxx/5xxx` per Section 8.

With RFC 002:
- Transport principal binding is defined in RFC 002.
- Delegation evaluation MUST bind the active caller identity to transport-authenticated principal policy.

With RFC 003:
- Relays treat delegation bodies as opaque AMP payload.
- Store-and-forward redelivery MUST preserve delegation bytes unchanged.

With RFC 004:
- Capability authorization scope in this RFC refers to RFC 004 capability identifiers.
- RFC 004 schema compatibility does not bypass delegation policy checks.
- RFC 004 `CAP_INVOKE.body.delegation` is the delegated-execution carriage binding in this RFC revision.

With RFC 006:
- Sessions MAY cache validated delegation chain fingerprints.
- Session recovery MUST re-check revocation freshness before continuing privileged operations.

With RFC 008:
- Delegation service discovery MAY use DID service metadata.
- Discovery metadata MUST NOT override signed delegation semantics in this RFC.

---

## 4. Delegation Credential Model

### 4.1 Credential Fields and Semantics

A delegation credential represents a direct grant from `delegator` to `delegate`.

Core semantics:
- Credential identity key is (`delegator`, `delegation_id`).
- Grant authority is bounded by `scope`, `validity`, and optional `aud`.
- Subdelegation is controlled by `allow_subdelegation`.
- Each credential link is independently signed by its own `delegator` DID key.

### 4.2 Scope Model

Scope dimensions:
- `capabilities`: list of allowed capability selectors.
- `actions`: list of allowed action names.
- `resources`: list of allowed resource identifiers.
- `constraints`: key-value limits interpreted by verifier policy.

MTI interoperability rules:
- Each selector in `capabilities` MUST be an exact capability name (for example `org.agentries.code-review`) or exact capability ID (for example `org.agentries.code-review:2.1.0`).
- Wildcards (`*`), negation (`!x`), and regex-like patterns are NOT MTI and MUST be rejected with `3004`.
- At least one of `capabilities`, `actions`, `resources` MUST be present.
- Unknown constraint keys MUST be treated as unsupported policy input and MUST fail closed (`3004`) unless explicitly enabled by local profile policy.
- Scope normalization for chain evaluation is defined in Section 6.2 and is REQUIRED.

### 4.3 Validity and Audience

Validity rules:
- `issued_at` and `expires_at` are required epoch-millisecond timestamps.
- `not_before` is optional; if absent, effective value is `issued_at`.
- Credential is valid only when `not_before <= now < expires_at`.

Audience rules:
- If `aud` is present, verifier DID MUST match one of the listed DID values.
- If `aud` is absent, credential is audience-unbound.

### 4.4 Credential CDDL

```cddl
did = tstr
delegation-id = tstr
unix-ms = uint
capability-selector = tstr
action-name = tstr
resource-id = tstr

constraint-map = {
  * tstr => any
}

delegation-scope = {
  ? "capabilities": [+ capability-selector],
  ? "actions": [+ action-name],
  ? "resources": [+ resource-id],
  ? "constraints": constraint-map
}

delegation-validity = {
  "issued_at": unix-ms,
  ? "not_before": unix-ms,
  "expires_at": unix-ms
}

delegation-credential-payload = {
  "cred_v": 1,
  "delegation_id": delegation-id,
  "delegator": did,
  "delegate": did,
  "scope": delegation-scope,
  "validity": delegation-validity,
  ? "allow_subdelegation": bool,
  ? "max_chain_depth": uint,
  ? "aud": [+ did],
  ? "nonce": bstr
}

delegation-credential-envelope = {
  "format": "cose_sign1",
  "credential": bstr
  ; COSE_Sign1 over deterministic_cbor(delegation-credential-payload)
}
```

Credential consistency rules:
- Signer DID MUST equal payload `delegator`.
- `expires_at` MUST be strictly greater than effective `not_before`.
- If `max_chain_depth` is present, it MUST be >= 1.
- If `allow_subdelegation` is absent, default is `false`.
- COSE protected header `alg` MUST be present.
- COSE protected header `kid` MUST resolve to a verification method under delegator DID policy.

### 4.5 Signature Algorithm Profile (MTI)

To prevent algorithm drift across implementations:
- Credential and revocation signers/verifiers MUST implement COSE `alg = -8` (EdDSA) with OKP `crv = Ed25519`.
- Implementations MAY additionally support other algorithms by local profile policy.
- Unsupported signature algorithms for delegation artifacts MUST be rejected as `3004 DELEGATION_INVALID`.

---

## 5. Delegation Protocol Semantics

### 5.1 DELEG_GRANT

Purpose:
- Deliver a new delegation credential from delegator to delegate.

Rules:
- Body MUST include exactly one `credential` envelope.
- Recipient MUST verify credential signature and payload consistency before accepting.
- If accepted, recipient stores grant by canonical key (`delegator`, `delegation_id`) and returns `PROC_OK` or equivalent success response.

### 5.2 DELEG_REVOKE

Purpose:
- Revoke a previously issued delegation credential.

Rules:
- Body MUST include signed `revocation` object (Section 7.1).
- Body MUST include top-level `delegation_id` (RFC 001 compatibility baseline).
- Revocation signer DID MUST equal the original delegator DID for the target `delegation_id`.
- Top-level `delegation_id` MUST equal signed `delegation-revocation-payload.delegation_id`; mismatch MUST be rejected with `4001`.
- On valid revoke, receiver MUST mark delegation as revoked and deny future use.

### 5.3 DELEG_QUERY

Purpose:
- Query delegation status at verifier/publisher.

Rules:
- Body MUST include non-empty `delegation_id`.
- Body SHOULD include `delegator` to avoid key collisions across issuers.
- If `delegator` is omitted and lookup is ambiguous across multiple issuers, responder MUST reject with `4001`.
- Responder SHOULD return `RESPONSE` with status object:
  - `active`, `revoked`, `expired`, or `unknown`.
- `unknown` MAY be returned without revealing broader issuer state.

Delegation evidence carriage rules (normative):
- For delegated capability invocation (`CAP_INVOKE`), delegation evidence MUST be carried in the signed AMP `body` object under key `delegation`.
- Delegated execution attempt is defined by the presence of `CAP_INVOKE.body.delegation`.
- `CAP_INVOKE` without `body.delegation` is a non-delegated invocation path.
- Delegation evidence found only in `ext` MUST NOT be used for authorization decisions.
- If delegation evidence appears outside signed `CAP_INVOKE.body.delegation` (for example in `ext`), verifier MUST deny with `3004`.
- If a non-delegation-capable message type (all `typ` except `CAP_INVOKE` in RFC 001 Section 4.6) carries `body.delegation`, verifier MUST reject with `4001`.

### 5.4 Message Body CDDL

```cddl
deleg-grant-body = {
  "credential": delegation-credential-envelope,
  ? "scope": any,      ; legacy/non-MTI compatibility alias
  ? "expires": tstr    ; legacy/non-MTI compatibility alias
}

deleg-revoke-body = {
  "delegation_id": delegation-id,
  "revocation": bstr
  ; COSE_Sign1 over deterministic_cbor(delegation-revocation-payload)
}

deleg-query-body = {
  "delegation_id": delegation-id,
  ? "delegator": did,
  ? "as_of": unix-ms
}

deleg-query-status = "active" / "revoked" / "expired" / "unknown"

deleg-query-result = {
  "delegator": did,
  "delegation_id": delegation-id,
  "status": deleg-query-status,
  ? "expires_at": unix-ms,
  ? "revoked_at": unix-ms,
  "updated_at": unix-ms,
  ? "max_age_s": uint
}

; Generic evidence object for delegated authorization checks.
; Normative carriage: signed AMP body field `delegation` (Section 5.3).
delegation-evidence = {
  "chain": [+ delegation-credential-envelope],
  ? "proof": bstr,
  ? "target": {
    ? "capability": tstr,
    ? "action": tstr,
    ? "resource": tstr
  }
}
```

---

## 6. Authorization Evaluation Algorithm

### 6.1 Validation Order (Normative)

For each delegated capability invocation request, verifier MUST execute checks in this order:
1. Validate message type is `CAP_INVOKE` (RFC 001 Section 4.6).
2. Parse signed AMP `body.delegation` evidence structure and each credential envelope.
3. Validate chain continuity (`link[i].delegate == link[i+1].delegator`).
4. Verify each credential signature and signer identity binding.
5. Validate temporal window and audience (`aud`) for each link.
6. Validate revocation state for each link.
7. Validate subdelegation permission and chain depth policy.
8. Compute effective scope by narrowing across chain links.
9. Evaluate request target against effective scope and constraints.
10. Emit allow/deny decision and audit record.

### 6.2 Chain Narrowing Rules

Given chain links `L1..Ln`:
- `L1` establishes initial grant scope.
- For each scope dimension `d` in `{capabilities, actions, resources}`:
  - In `L1`, if `d` is omitted, it is normalized to unrestricted `ANY_d`.
  - In `Li` (i > 1), if `d` is omitted, it inherits prior effective value `E(i-1,d)` (no change).
  - In `Li` (i > 1), if `d` is present, `Li[d]` MUST be a subset of `E(i-1,d)`.
  - Effective value updates as `E(i,d) = Li[d]` when present, otherwise `E(i-1,d)`.
- If any link expands scope, chain is invalid (`3004`).
- If `allow_subdelegation=false` at `Li`, no `Li+1` is permitted.
- If `Li.max_chain_depth = d` is present, remaining downstream links count `(n - i)` MUST be `<= d`.
- Effective scope is the normalized terminal tuple `(E(n,capabilities), E(n,actions), E(n,resources), constraints)`.

Caller binding rules:
- Transport/authenticated caller identity MUST match `Ln.delegate` unless explicit local policy allows equivalent identity mapping.
- Mismatch MUST be denied as `3001`.

### 6.3 Revocation Checks

For each link:
- Verifier MUST check local revocation cache.
- Revocation lookup key MUST be (`delegator`, `delegation_id`) from the signed credential payload.
- If cache stale or missing, verifier MUST query configured revocation source unless policy allows bounded offline mode.
- If source is unreachable and strict mode is enabled (default), verifier MUST fail closed with `5002`.
- If revocation exists with `revoked_at <= now`, credential is invalid (`3004`).

### 6.4 Decision Output and Audit

Verifier MUST produce auditable record containing at least:
- `decision`: allow/deny
- `reason_code`
- `requester_did`
- `effective_delegator_did`
- `delegation_ids` (paired with delegator identity)
- `target` (capability/action/resource)
- `evaluated_at`

Sensitive payloads SHOULD NOT be logged in cleartext.

---

## 7. Revocation Publication Model

### 7.1 Revocation Object

```cddl
delegation-revocation-payload = {
  "rev_v": 1,
  "delegation_id": delegation-id,
  "delegator": did,
  "revoked_at": unix-ms,
  ? "reason": tstr
}
```

Rules:
- Revocation signer MUST equal payload `delegator`.
- `revoked_at` MUST be >= credential `issued_at`.
- Multiple revocations for same (`delegator`, `delegation_id`) pair MUST be idempotent.

### 7.2 Revocation Sources and Status Object

Supported source models:
- DID service endpoint controlled by delegator.
- Signed offline revocation bundle.
- Direct `DELEG_REVOKE` message receipt.

Source authenticity MUST be anchored in DID verification policy.

Status object requirement:
- Publishers/verifiers providing revocation lookup MUST expose `deleg-query-result` keyed by (`delegator`, `delegation_id`).
- `updated_at` in `deleg-query-result` MUST indicate status freshness timestamp.
- `max_age_s` SHOULD be provided when freshness TTL is known.

### 7.3 Caching and Freshness

- Verifiers SHOULD cache revocation status by (`delegator`, `delegation_id`).
- Cache entries MUST honor publisher freshness policy (`max_age_s`) if provided.
- Expired cache entries MUST be refreshed before privileged authorization unless local emergency mode is explicitly configured.

---

## 8. Error Handling and Retry

This RFC reuses RFC 001 error codes.

| Failure | Code | Retry |
|---------|------|-------|
| Malformed delegation body or unsupported field shape | `1001` | No |
| `body.delegation` present on non-delegation-capable message type | `4001` | No |
| Unsupported `cred_v` / `rev_v` | `1004` | No |
| Top-level `delegation_id` mismatch with signed revocation payload | `4001` | No |
| Unauthorized caller identity for delegation use | `3001` | No |
| Credential invalid/expired/revoked/signature-invalid/chain-invalid | `3004` | No |
| Unsupported delegation credential/revocation signature algorithm | `3004` | No |
| Unsupported wildcard/negation selector in MTI profile | `3004` | No |
| Revocation source unavailable in strict mode | `5002` | Yes |
| Internal policy engine failure | `5001` | Yes |

Retry guidance:
- `300x/400x` failures SHOULD NOT be retried without credential/policy mutation.
- `500x` failures MAY be retried with bounded exponential backoff.

Privacy guidance:
- Verifiers SHOULD avoid leaking whether a specific delegator exists when returning denial responses.

---

## 9. Versioning and Compatibility

This RFC versioning model:
- Credential payload object is versioned by `cred_v`.
- Revocation payload object is versioned by `rev_v`.
- New optional fields MAY be added without breaking existing parsers.
- Removing or redefining existing required fields requires a new RFC 005 major revision.

Compatibility rules:
- Receivers MUST reject unsupported `cred_v`/`rev_v` with `1004`.
- Unknown optional fields MUST be ignored unless they alter security semantics.

---

## 10. Security Considerations

- Delegation verification MUST be fail-closed for signature, chain continuity, and scope narrowing violations.
- Unknown constraints can create privilege escalation; default handling MUST be deny unless explicitly supported.
- Delegation chains increase attack surface; implementations SHOULD keep max chain depth small (recommended default: 3).
- Replay resistance SHOULD be strengthened with request binding (`proof`) where high-value operations are involved.
- Key rotation for delegator DID methods MUST preserve verifiability for historical credentials during grace period.

---

## 11. Privacy Considerations

- Delegation artifacts may reveal organizational topology (who delegates to whom).
- Implementations SHOULD minimize retention of full credential payloads in logs.
- Query interfaces SHOULD avoid broad enumeration responses and prefer delegation-ID keyed lookups.
- Revocation endpoints SHOULD apply access controls and rate limits to reduce probing.

---

## 12. Implementation Checklist

- Parse and validate `DELEG_GRANT`, `DELEG_REVOKE`, `DELEG_QUERY` bodies.
- Verify COSE signatures and DID signer binding for all credential/revocation artifacts.
- Enforce chain continuity, subdelegation, depth, and scope narrowing rules.
- Enforce temporal validity and audience checks.
- Enforce (`delegator`, `delegation_id`) as canonical lookup/cache identity key.
- Reject `body.delegation` on non-`CAP_INVOKE` message types with `4001`.
- Implement revocation source lookup with strict fail-closed default.
- Emit deterministic error codes per Section 8.
- Add conformance tests from Appendix A.

---

## 13. References

### 13.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 004: Capability Schema Registry & Compatibility
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in requirement words
- RFC 8949: CBOR
- RFC 9052: COSE Structures

### 13.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay and Store-and-Forward
- RFC 006: Session Protocol
- RFC 008: Agent Discovery and Directory
- W3C DID Core

---

## Appendix A. Minimal Test Vectors

### A.1 Single-Link Delegation Positive

Input:
- Valid `DELEG_GRANT` with one credential from A to B.
- Signature valid, not expired, not revoked.

Expected:
- Grant accepted and stored.

### A.2 Expired Credential Negative

Input:
- Credential with `expires_at < now`.

Expected:
- `3004 DELEGATION_INVALID`.

### A.3 Chain Narrowing Positive

Input:
- Chain A->B allows `{cap=code-review, action=invoke}`.
- Chain B->C narrows to same capability and action subset.

Expected:
- Chain accepted.

### A.4 Chain Expansion Negative

Input:
- A->B allows `action=read`.
- B->C requests `action=write`.

Expected:
- `3004 DELEGATION_INVALID`.

### A.5 Subdelegation Forbidden Negative

Input:
- A->B has `allow_subdelegation=false`.
- Presented chain includes B->C.

Expected:
- `3004 DELEGATION_INVALID`.

### A.6 Caller Identity Mismatch Negative

Input:
- Last chain delegate is DID C.
- Transport principal is DID D.

Expected:
- `3001 UNAUTHORIZED`.

### A.7 Revocation Positive

Input:
- Valid `DELEG_REVOKE` signed by original delegator.

Expected:
- Delegation marked revoked; later use denied with `3004`.

### A.8 Query Unknown ID

Input:
- `DELEG_QUERY` for non-existent `delegation_id`.

Expected:
- `RESPONSE` with `status="unknown"` or policy-equivalent denial without leakage.

### A.9 Unsupported Selector Syntax Negative

Input:
- Scope uses wildcard `org.agentries.*` in MTI profile.

Expected:
- `3004 DELEGATION_INVALID`.

### A.10 Revocation Source Unavailable (Strict Mode)

Input:
- Cache expired, revocation source unreachable.

Expected:
- `5002 UNAVAILABLE`.

### A.11 Audience Mismatch Negative

Input:
- Credential `aud=[did:web:service-x]`.
- Verifier DID is `did:web:service-y`.

Expected:
- `3004 DELEGATION_INVALID`.

### A.12 Version Rejection Negative

Input:
- Credential with unsupported `cred_v`.

Expected:
- `1004 UNSUPPORTED_VERSION`.

### A.13 Revoke ID Mismatch Negative

Input:
- `DELEG_REVOKE.body.delegation_id = "delegation:abc"`
- Signed `delegation-revocation-payload.delegation_id = "delegation:def"`

Expected:
- `4001 BAD_REQUEST`.

### A.14 Evidence In `ext` Only Negative

Input:
- Delegated request contains delegation evidence only in `ext`, with no signed `body.delegation`.

Expected:
- Verifier ignores `ext` for authorization input.
- Request denied with `3004 DELEGATION_INVALID`.

### A.15 max_chain_depth Exceeded Negative

Input:
- Chain link `L1.max_chain_depth = 1`.
- Presented chain length requires two downstream links from `L1`.

Expected:
- `3004 DELEGATION_INVALID`.

### A.16 Unsupported Signature Algorithm Negative

Input:
- Delegation credential uses non-MTI COSE `alg` not supported by verifier profile.

Expected:
- `3004 DELEGATION_INVALID`.

### A.17 Revocation Freshness Metadata Positive

Input:
- Revocation lookup returns `deleg-query-result` with `updated_at` and `max_age_s`.

Expected:
- Verifier cache policy uses returned freshness metadata.

### A.18 Delegation on Non-Capable Message Type Negative

Input:
- Control message (`PING`) includes `body.delegation`.

Expected:
- `4001 BAD_REQUEST`.

### A.19 Ambiguous Query Without Delegator Negative

Input:
- Two issuers publish distinct records with same `delegation_id`.
- `DELEG_QUERY` omits `delegator`.

Expected:
- Responder rejects with `4001 BAD_REQUEST` (ambiguous key) or policy-equivalent non-leaking denial.

### A.20 Byte-Level Error Code Checks

Input:
- Unsupported/invalid delegation evidence case mapped to `3004`.
- Ambiguous query/malformed request case mapped to `4001`.

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
| 2026-02-04 | Proposal | Ryan Cooper | Initial proposal outline |
| 2026-02-07 | 0.1 | Nowa | Rewrote RFC 005 into normative draft structure with profiles, boundary contracts, CDDL, auth algorithm, revocation model, and test vectors |
| 2026-02-07 | 0.2 | Nowa | Fixed RFC 001 compatibility for DELEG_REVOKE body, made delegation evidence carriage deterministic (signed body only), formalized scope normalization and max_chain_depth enforcement, and added negative vectors for mismatch/ext-only/depth overflow |
| 2026-02-07 | 0.3 | Nowa | Added MTI signature algorithm profile, standardized revocation freshness status object, completed error mapping with 1004/version and algorithm cases, and resolved open questions for this revision |
| 2026-02-07 | 0.31 | Nowa | Aligned with RFC 001 cross-message delegation overlay: removed schema-local capability restriction wording, added non-capable-type rejection rule, and updated validation/error/test coverage |
| 2026-02-07 | 0.32 | Nowa | Narrowed delegated-execution interop baseline to CAP_INVOKE-only and aligned carriage/validation wording with RFC 001 Section 4.6 |
| 2026-02-07 | 0.33 | Nowa | Unified selector-syntax failure mapping to 3004, defined canonical identity key as (delegator, delegation_id), clarified CAP_INVOKE delegation trigger semantics, and added ambiguity/keying conformance coverage |
| 2026-02-07 | 0.34 | Nowa | Added minimal byte-level error-code checks for delegation interop vectors (`3004`/`4001`) |
