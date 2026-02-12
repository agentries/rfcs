# Interop Reports

This directory stores machine-readable conformance reports (`interop-report.json` shape) for this suite version.

## Naming

Use stable, scan-friendly names:
- `<implementation-id>.core-pass.<date>.json`
- `<implementation-id>.full-stack-precheck.<date>.json`
- `<implementation-id>.full-stack-pass.<date>.json`
- `<implementation-id>.<profile>.template.json`

## Current Artifacts

- `amp-rust-reference.core-pass.2026-02-10.json`: core profile draft run for `amp-full-core-001-006` with required vectors marked `pass`.
- `amp-rust-reference.full-stack-precheck.2026-02-10.json`: full-stack profile precheck (`amp-full-stack-001-011`) with explicit non-pass vectors marked as `fail`.
- `amp-rust-reference.core-pass.template.json`: core pass-report template (placeholder statuses).
- `amp-rust-reference.full-stack-pass.template.json`: full-stack pass-report template (placeholder statuses).
- `amp-go-reference.full-stack-pass.template.json`: second-implementation full-stack pass-report template (placeholder statuses).

## Gate Rule Reminder

`Accepted/Implementation-Ready` requires:
- At least two independent implementations.
- Each report passes semantic validation for selected profile.
- Every profile-required vector status is `pass`.

Run gate validation from suite root:

```bash
./validate-draft-gate.sh ./reports
./validate-accepted-gate.sh ./reports
```

The validator excludes `.template.` files by default.
