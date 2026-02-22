# Gate Status

Suite: `amp-standard-profiles-012-014-draft-v1`
Updated: `2026-02-22`
Target profile: `amp-standard-profiles-012-014`

## Accepted Gate Criteria (RFC 001 Section 1.5.2)

- At least two independent implementations.
- Machine-readable report per implementation.
- For selected profile, every required vector is present and `pass`.
- Required vectors MUST NOT be `skip`.

## Current Snapshot

- Stage: `Draft`
- Draft gate: `NOT SATISFIED`
- Accepted gate: `NOT SATISFIED`
- Reports currently tracked: none

## Interpretation

- Draft gate requires at least one `*.profile-pass.*.json` report with all 39 required vectors marked `pass`.
- `Accepted gate = NOT SATISFIED` is expected at this stage and is tracked as deferred scope, not as a defect.
- Transition to `Accepted/Implementation-Ready` remains governed by RFC 001 Section 1.5.2.

## Current Gaps to Close

- Produce at least one profile-pass report (`*.profile-pass.<date>.json`) with all 39 required vectors marked `pass`.
- For accepted gate: produce at least two such reports from independent implementations.
- Run gate validators:

```bash
./validate-draft-gate.sh ./reports
./validate-accepted-gate.sh ./reports
```
