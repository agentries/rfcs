# AMP Conformance Suite: 2026-02-07-draft-v1

This directory is the canonical artifact set for the draft interoperability suite referenced by RFC 001 Section 1.5.2.

## Scope

- Target stage: Draft interoperability baseline.
- Referenced RFCs (core/full gate): 001-006.
- Optional extension RFCs: 007.
- Vector source: Appendix A vectors in each RFC, selected for cross-implementation determinism.

## Files

- `vector-set.json`: versioned vector-set manifest.
- `interop-report.schema.json`: JSON schema for machine-readable reports.
- `interop-report.template.json`: starter template for implementation reports.

## Reporting

Each implementation run should emit one `interop-report.json` conforming to `interop-report.schema.json`.

Required report fields are aligned with RFC 001 Section 1.5.2:
- implementation identifier and version
- conformance suite/vector set version
- per-vector pass/fail results
- environment metadata sufficient for rerun
