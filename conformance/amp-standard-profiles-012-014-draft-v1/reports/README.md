# Interop Reports

This directory stores machine-readable conformance reports (`interop-report.json` shape) for the `amp-standard-profiles-012-014-draft-v1` suite.

## Naming

Use stable, scan-friendly names:
- `<implementation-id>.profile-pass.<date>.json`
- `<implementation-id>.profile-pass.template.json`

## Current Artifacts

- `amp-reference.profile-pass.template.json`: profile-pass report template (placeholder statuses).

## Gate Rule Reminder

`Accepted/Implementation-Ready` requires:
- At least two independent implementations.
- Each report passes semantic validation for the profile.
- Every required vector status is `pass`.

Run gate validation from suite root:

```bash
./validate-draft-gate.sh ./reports
./validate-accepted-gate.sh ./reports
```

The validator excludes `.template.` files by default.
