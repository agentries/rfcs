# AMP Conformance Suite: amp-standard-profiles-012-014-draft-v1

This directory is the canonical artifact set for the draft interoperability suite covering RFC 012-014 (AMP Standard Profiles: Task, Negotiation, Handoff).

## Scope

- Target stage: Draft interoperability baseline.
- Current-cycle design requirement: `Draft` gate must be satisfied.
- Accepted gate status for this suite version: not yet satisfied (see `GATE-STATUS.md`).
- Current-cycle design-requirement completion command: `./validate-design-requirements.sh ./reports`.
- Referenced RFCs: 012 (v0.1), 013 (v0.1), 014 (v0.2).
- Dependency RFCs: 001 (v0.46) — for HELLO/typ/dispatch baseline.
- Single profile: `amp-standard-profiles-012-014`.
- Vector source: Appendix A vectors in each RFC.
- All 39 vectors are required; `optional_vectors` is empty.
- Every required vector MUST appear in the report with status `pass` or `fail` (not `skip`).

## Files

- `vector-set.json`: versioned vector-set manifest.
- `interop-report.schema.json`: structural JSON schema for machine-readable reports.
- `interop-report.template.json`: starter template with all 39 vectors defaulting to `fail`.
- `validate-report.sh`: semantic validator against `vector-set.json` (required-vector coverage, `skip` policy, duplicate/unknown IDs).
- `validate-draft-gate.sh`: draft-gate validator (requires >=1 profile-pass report with all required vectors = pass).
- `validate-design-requirements.sh`: current-cycle design-requirement validator (delegates to draft gate).
- `validate-accepted-gate.sh`: accepted-gate validator (`>=2` implementations, all required vectors `pass`).
- `reports/`: conformance report artifacts.
- `GATE-STATUS.md`: current gate readiness snapshot for this suite version.

## Reporting

Each implementation run should emit one `interop-report.json` conforming to `interop-report.schema.json`.
Use `interop-report.template.json` as a starter.

Report naming convention:
- `<implementation-id>.profile-pass.<date>.json`
- `<implementation-id>.profile-pass.template.json`

Validation steps:
- Structural check (JSON Schema): use your preferred validator against `interop-report.schema.json`.
- Semantic check: `./validate-report.sh ./interop-report.json`.
- Draft-gate check: `./validate-draft-gate.sh ./reports`.
- Current-cycle design-requirement check: `./validate-design-requirements.sh ./reports`.
- Accepted-gate check (min 2 implementations): `./validate-accepted-gate.sh ./reports`.
- The accepted-gate validator defaults to `*.profile-pass.*.json` and ignores `*.template.*.json`.
- The accepted-gate validator rejects placeholder metadata (`0.0.0-template`, `replace-with-commit`, `1970-01-01T00:00:00Z`, or environment placeholders).

Required report fields are aligned with RFC 001 Section 1.5.2:
- implementation identifier and version
- conformance suite/vector set version
- per-vector pass/fail results
- environment metadata sufficient for rerun
