# AMP Conformance Suite: 2026-02-07-draft-v1

This directory is the canonical artifact set for the draft interoperability suite referenced by RFC 001 Section 1.5.2.

## Scope

- Target stage: Draft interoperability baseline.
- Current-cycle design requirement: `Draft` gate must be satisfied.
- Accepted gate status for this suite version: not yet satisfied (see `GATE-STATUS.md`).
- Current-cycle design-requirement completion command: `./validate-design-requirements.sh ./reports`.
- Referenced RFCs (core/full gate baseline): 001-006.
- Optional extension RFCs: 007, 008, 009, 010, 011.
- Profiles:
  - `amp-full-core-001-006` (default): required gate vectors from RFC 001-006.
  - `amp-full-stack-001-011`: required gate vectors from RFC 001-006 plus selected RFC 007-011 vectors.
- Vector source: Appendix A vectors in each RFC, selected for cross-implementation determinism.
- `required_vectors` defines the `amp-full-core-001-006` gate set for this draft baseline.
- `required_vectors_full_stack` defines the `amp-full-stack-001-011` gate set for this draft baseline.
- `optional_vectors` carries extension/profile vectors (including RFC 007, RFC 008, RFC 009, RFC 010, RFC 011, and optional profile vectors from RFC 002/003/006) and fixture-only entries.
- Every required vector in the selected profile MUST appear in the report with status `pass` or `fail` (not `skip`).
- `skip` is only valid for `optional_vectors`.

## Files

- `vector-set.json`: versioned vector-set manifest.
- `interop-report.schema.json`: structural JSON schema for machine-readable reports.
- `interop-report.template.json`: starter template for implementation reports.
- `validate-report.sh`: semantic validator against `vector-set.json` (profile-specific required-vector coverage, `skip` policy, duplicate/unknown IDs).
- `validate-draft-gate.sh`: draft-gate validator (core-pass baseline + full-stack precheck evidence).
- `validate-design-requirements.sh`: current-cycle design-requirement validator (delegates to draft gate).
- `validate-accepted-gate.sh`: accepted-gate validator (`>=2` implementations, profile-required vectors all `pass`).
- `reports/`: draft/full-stack precheck report artifacts.
- `GATE-STATUS.md`: current gate readiness snapshot for this suite version.

## Reporting

Each implementation run should emit one `interop-report.json` conforming to `interop-report.schema.json`.
Use `interop-report.template.json` as a starter for the default `amp-full-core-001-006` profile.
The template is not a full-stack pass report: when claiming `amp-full-stack-001-011`, all vectors in `required_vectors_full_stack` (including selected RFC 007-011 vectors) MUST be reported as `pass` or `fail` (not `skip`).
Use `*.full-stack-precheck.*.json` for draft gap-tracking runs and `*.full-stack-pass.*.json` for accepted-gate candidate runs.

Validation steps:
- Structural check (JSON Schema): use your preferred validator against `interop-report.schema.json`.
- Semantic check for default profile: `./validate-report.sh ./interop-report.json`.
- Semantic check for full-stack profile: `./validate-report.sh ./interop-report.json ./vector-set.json amp-full-stack-001-011`.
- Draft-gate check: `./validate-draft-gate.sh ./reports`.
- Current-cycle design-requirement check: `./validate-design-requirements.sh ./reports`.
- Accepted-gate check (default full-stack profile, min 2 implementations): `./validate-accepted-gate.sh ./reports`.
- The accepted-gate validator defaults to `*.full-stack-pass.*.json` for full-stack and `*.core-pass.*.json` for core profile and ignores `*.template.*.json`.
- The accepted-gate validator rejects placeholder metadata (`0.0.0-template`, `replace-with-commit`, `1970-01-01T00:00:00Z`, or environment placeholders) to prevent non-executable artifacts from being counted as accepted evidence.
- Optional accepted-gate check with explicit report glob: `./validate-accepted-gate.sh ./reports ./vector-set.json amp-full-stack-001-011 2 "*.full-stack-pass.*.json"`.

Required report fields are aligned with RFC 001 Section 1.5.2:
- implementation identifier and version
- conformance suite/vector set version
- per-vector pass/fail results
- environment metadata sufficient for rerun
