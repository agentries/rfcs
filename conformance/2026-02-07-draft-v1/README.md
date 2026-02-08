# AMP Conformance Suite: 2026-02-07-draft-v1

This directory is the canonical artifact set for the draft interoperability suite referenced by RFC 001 Section 1.5.2.

## Scope

- Target stage: Draft interoperability baseline.
- Referenced RFCs (core/full gate baseline): 001-006.
- Optional extension RFCs: 007, 008, 009.
- Profiles:
  - `amp-full-core-001-006` (default): required gate vectors from RFC 001-006.
  - `amp-full-stack-001-009`: required gate vectors from RFC 001-006 plus selected RFC 007-009 vectors.
- Vector source: Appendix A vectors in each RFC, selected for cross-implementation determinism.
- `required_vectors` defines the `amp-full-core-001-006` gate set for this draft baseline.
- `required_vectors_full_stack` defines the `amp-full-stack-001-009` gate set for this draft baseline.
- `optional_vectors` carries extension/profile vectors (including RFC 007, RFC 008, RFC 009, and optional profile vectors from RFC 002/003/006) and fixture-only entries.
- Every required vector in the selected profile MUST appear in the report with status `pass` or `fail` (not `skip`).
- `skip` is only valid for `optional_vectors`.

## Files

- `vector-set.json`: versioned vector-set manifest.
- `interop-report.schema.json`: structural JSON schema for machine-readable reports.
- `interop-report.template.json`: starter template for implementation reports.
- `validate-report.sh`: semantic validator against `vector-set.json` (profile-specific required-vector coverage, `skip` policy, duplicate/unknown IDs).

## Reporting

Each implementation run should emit one `interop-report.json` conforming to `interop-report.schema.json`.
Use `interop-report.template.json` as a starter for the default `amp-full-core-001-006` profile.

Validation steps:
- Structural check (JSON Schema): use your preferred validator against `interop-report.schema.json`.
- Semantic check for default profile: `./validate-report.sh ./interop-report.json`.
- Semantic check for full-stack profile: `./validate-report.sh ./interop-report.json ./vector-set.json amp-full-stack-001-009`.

Required report fields are aligned with RFC 001 Section 1.5.2:
- implementation identifier and version
- conformance suite/vector set version
- per-vector pass/fail results
- environment metadata sufficient for rerun
