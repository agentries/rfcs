#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 2
fi

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <reports-dir> [vector-set.json]" >&2
  exit 2
fi

reports_dir="$1"
vector_set="${2:-$(dirname "$0")/vector-set.json}"
validator="$(dirname "$0")/validate-report.sh"

if [ ! -d "$reports_dir" ]; then
  echo "error: reports directory not found: $reports_dir" >&2
  exit 2
fi

if [ ! -f "$vector_set" ]; then
  echo "error: vector set file not found: $vector_set" >&2
  exit 2
fi

if [ ! -x "$validator" ]; then
  echo "error: validator not executable: $validator" >&2
  exit 2
fi

required_ids="$(jq -r '.required_vectors[]' "$vector_set")"

profile_reports="$(find "$reports_dir" -maxdepth 1 -type f -name '*.profile-pass.*.json' | sort | grep -v '\.template\.' || true)"

if [ -z "$profile_reports" ]; then
  echo "error: draft gate requires at least one profile-pass report (*.profile-pass.*.json)" >&2
  exit 1
fi

profile_ok=0
for report in $profile_reports; do
  "$validator" "$report" "$vector_set" amp-standard-profiles-012-014 >/dev/null

  all_pass=1
  while IFS= read -r vector_id; do
    [ -z "$vector_id" ] && continue
    status="$(jq -r --arg id "$vector_id" '.results[] | select(.vector_id == $id) | .status' "$report")"
    if [ "$status" != "pass" ]; then
      all_pass=0
      break
    fi
  done <<< "$required_ids"

  if [ "$all_pass" -eq 1 ]; then
    profile_ok=1
    break
  fi
done

if [ "$profile_ok" -ne 1 ]; then
  echo "error: draft gate requires at least one profile-pass report with all required vectors marked pass" >&2
  exit 1
fi

echo "ok: draft gate satisfied (profile-pass evidence present)"
