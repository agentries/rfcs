#!/usr/bin/env bash
set -euo pipefail

# Validates RFC header metadata and checks for README drift.
# No external dependencies beyond grep/sed/awk.
#
# Checks:
#   1. Header fields: Status, Authors, Created, Updated, Version in lines 1-10
#   2. RFC number match: # RFC NNN: title matches filename NNN-*.md
#   3. Version sync: RFC header version matches README.md table (with v prefix)
#   4. Status sync: RFC header status matches README.md table
#   5. Date format: Created/Updated are valid YYYY-MM-DD
#
# Exit 0 = all pass, Exit 1 = any fail.

repo_root="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
readme="$repo_root/README.md"
errors=0

if [ ! -f "$readme" ]; then
  echo "error: README.md not found at: $readme" >&2
  exit 2
fi

for rfc_file in "$repo_root"/[0-9][0-9][0-9]-*.md; do
  [ -f "$rfc_file" ] || continue

  filename="$(basename "$rfc_file")"
  file_number="$(echo "$filename" | sed 's/^\([0-9][0-9][0-9]\)-.*/\1/')"

  # Read first 10 lines for header parsing
  header="$(head -10 "$rfc_file")"

  # --- Check 1: Required header fields ---
  for field in Status Authors Created Updated Version; do
    if ! echo "$header" | grep -q "^\*\*${field}\*\*:"; then
      echo "error: $filename: missing header field: $field" >&2
      errors=$((errors + 1))
    fi
  done

  # --- Check 2: RFC number matches filename ---
  title_line="$(echo "$header" | head -1)"
  title_number="$(echo "$title_line" | sed -n 's/^# RFC \([0-9][0-9]*\):.*/\1/p')"

  if [ -z "$title_number" ]; then
    echo "error: $filename: title line does not match '# RFC NNN: ...' pattern" >&2
    errors=$((errors + 1))
  else
    # Normalize: strip leading zeros for comparison
    norm_file="$(echo "$file_number" | sed 's/^0*//')"
    norm_title="$(echo "$title_number" | sed 's/^0*//')"
    if [ "$norm_file" != "$norm_title" ]; then
      echo "error: $filename: RFC number mismatch: filename=$file_number title=$title_number" >&2
      errors=$((errors + 1))
    fi
  fi

  # --- Extract header values ---
  hdr_status="$(echo "$header" | sed -n 's/^\*\*Status\*\*:[[:space:]]*\(.*\)/\1/p' | sed 's/[[:space:]]*$//')"
  hdr_version="$(echo "$header" | sed -n 's/^\*\*Version\*\*:[[:space:]]*\(.*\)/\1/p' | sed 's/[[:space:]]*$//')"
  hdr_created="$(echo "$header" | sed -n 's/^\*\*Created\*\*:[[:space:]]*\(.*\)/\1/p' | sed 's/[[:space:]]*$//')"
  hdr_updated="$(echo "$header" | sed -n 's/^\*\*Updated\*\*:[[:space:]]*\(.*\)/\1/p' | sed 's/[[:space:]]*$//')"

  # --- Check 5: Date format (YYYY-MM-DD) ---
  for date_field in "$hdr_created" "$hdr_updated"; do
    date_label="Created"
    if [ "$date_field" = "$hdr_updated" ]; then
      date_label="Updated"
    fi
    if [ -n "$date_field" ]; then
      if ! echo "$date_field" | grep -Eq '^[0-9]{4}-[0-9]{2}-[0-9]{2}$'; then
        echo "error: $filename: $date_label is not YYYY-MM-DD: '$date_field'" >&2
        errors=$((errors + 1))
      fi
    fi
  done

  # --- Check 3 & 4: README sync ---
  # Strip leading zeros from file_number for README table lookup (README uses no-leading-zero)
  readme_number="$(echo "$file_number" | sed 's/^0*//')"
  # Pad to 3 digits for matching patterns like "| 001 |"
  readme_padded="$(printf '%03d' "$readme_number")"

  # Find the matching row in the README table: | 001 | ... | Draft v0.46 | ... |
  readme_row="$(grep "^| ${readme_padded} |" "$readme" || grep "^| ${readme_number} |" "$readme" || true)"

  if [ -z "$readme_row" ]; then
    echo "warn: $filename: no matching row in README.md table for RFC $file_number" >&2
    continue
  fi

  # Extract the Status column (3rd pipe-delimited field) from README
  # Table format: | NNN | Title | Status vX.Y (notes) | Author | Date |
  readme_status_col="$(echo "$readme_row" | awk -F'|' '{print $4}' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"

  # Parse status word and version from README status column
  # "Draft v0.46" → status=Draft, version=0.46
  # "Draft v0.9 (coupled MTI ...)" → status=Draft, version=0.9
  readme_status="$(echo "$readme_status_col" | awk '{print $1}')"
  readme_version="$(echo "$readme_status_col" | sed -n 's/^[^ ]* v\([^ ]*\).*/\1/p')"

  # Check 4: Status sync
  if [ -n "$hdr_status" ] && [ -n "$readme_status" ]; then
    if [ "$hdr_status" != "$readme_status" ]; then
      echo "error: $filename: status mismatch: header='$hdr_status' readme='$readme_status'" >&2
      errors=$((errors + 1))
    fi
  fi

  # Check 3: Version sync
  if [ -n "$hdr_version" ] && [ -n "$readme_version" ]; then
    if [ "$hdr_version" != "$readme_version" ]; then
      echo "error: $filename: version mismatch: header='$hdr_version' readme='$readme_version'" >&2
      errors=$((errors + 1))
    fi
  fi

done

if [ "$errors" -gt 0 ]; then
  echo "FAIL: $errors error(s) found" >&2
  exit 1
fi

echo "ok: all RFC metadata checks passed"
