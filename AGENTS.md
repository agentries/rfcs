# Repository Guidelines

## Project Structure & Module Organization
- Top-level Markdown files are RFCs named with a three-digit prefix, e.g., `001-agent-messaging-protocol.md`.
- Supporting documents live alongside the RFCs: `README.md` (process + status table), `DECISION-LOG.md` (architecture decisions), and `AMP-FIRST-PRINCIPLES.md` (research rationale).
- This repository is documentation-only; there are no nested source, test, or asset directories.

## Build, Test, and Development Commands
- There are no build, test, or run commands in this repo; edits are plain Markdown.
- Helpful local commands:
  - `rg "RFC" .` for fast search across documents.
  - `git status` / `git diff` to review edits before opening a PR.

## Coding Style & Naming Conventions
- Use Markdown headings (`#`, `##`, `###`) and short, scannable paragraphs.
- RFC filenames should follow `NNN-kebab-case.md`, and the prefix must match the RFC number.
- Keep tables aligned for clean diffs; use fenced code blocks with a language tag (e.g., `json`, `cbor`).
- Prefer ASCII diagrams where helpful to keep files diff-friendly.

## Testing Guidelines
- No automated test framework or coverage requirements.
- Do a manual review: verify cross-references, internal links, and RFC numbering; update the README “Current RFCs” table when titles, statuses, or dates change.

## Commit & Pull Request Guidelines
- Git history currently contains a single commit (“Initial commit: Agentries RFCs”), so no established commit convention exists.
- Suggested commit style: short, imperative subject lines (e.g., “Add RFC 008 outline”).
- PRs should include a brief summary, the RFC status, and any updates to `README.md` and `DECISION-LOG.md` when decisions are introduced or revised.

## RFC Workflow & Content Expectations
- Follow the lifecycle stages described in `README.md`: Proposal → Draft → Review → Accepted → Implemented → Rejected/Withdrawn.
- When adding or revising an RFC, include a clear problem statement and scope, and mirror the structure of existing RFCs for consistency.
