#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 2
fi

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
  echo "usage: $0 <interop-report.json> [vector-set.json] [profile-id]" >&2
  exit 2
fi

report="$1"
vector_set="${2:-$(dirname "$0")/vector-set.json}"
profile_id="${3:-amp-full-core-001-006}"

if [ ! -f "$report" ]; then
  echo "error: report file not found: $report" >&2
  exit 2
fi

if [ ! -f "$vector_set" ]; then
  echo "error: vector set file not found: $vector_set" >&2
  exit 2
fi

suite_report="$(jq -r '.suite_version' "$report")"
suite_vectors="$(jq -r '.suite_version' "$vector_set")"

if [ "$suite_report" != "$suite_vectors" ]; then
  echo "error: suite_version mismatch: report=$suite_report vector_set=$suite_vectors" >&2
  exit 1
fi

optional_ids="$(jq -r '.optional_vectors[]' "$vector_set")"
report_ids="$(jq -r '.results[].vector_id' "$report")"

required_ids=""
if jq -e '.profiles? | type == "array"' "$vector_set" >/dev/null 2>&1; then
  required_field="$(jq -r --arg id "$profile_id" '.profiles[] | select(.id == $id) | .required_field' "$vector_set")"
  if [ -z "$required_field" ] || [ "$required_field" = "null" ]; then
    echo "error: unknown profile-id: $profile_id" >&2
    echo "available profiles:" >&2
    jq -r '.profiles[].id' "$vector_set" | sed '/^$/d; s/^/  - /' >&2
    exit 1
  fi
  required_ids="$(jq -r --arg field "$required_field" '.[$field][]' "$vector_set")"
else
  if [ "$profile_id" != "amp-full-core-001-006" ]; then
    echo "error: profile-id is not supported by this vector set: $profile_id" >&2
    exit 1
  fi
  required_ids="$(jq -r '.required_vectors[]' "$vector_set")"
fi

unknown_ids=""
while IFS= read -r id; do
  if ! grep -Fxq "$id" <<< "$required_ids" && ! grep -Fxq "$id" <<< "$optional_ids"; then
    unknown_ids="${unknown_ids}${id}"$'\n'
  fi
done <<< "$report_ids"

if [ -n "$unknown_ids" ]; then
  echo "error: unknown vector_id entries in report:" >&2
  echo "$unknown_ids" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

duplicate_ids="$(printf '%s\n' "$report_ids" | sed '/^$/d' | sort | uniq -d || true)"
if [ -n "$duplicate_ids" ]; then
  echo "error: duplicate vector_id entries in report:" >&2
  echo "$duplicate_ids" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

missing_required=""
skip_required=""

while IFS= read -r id; do
  if ! jq -e --arg id "$id" '.results[] | select(.vector_id == $id)' "$report" >/dev/null; then
    missing_required="${missing_required}${id}"$'\n'
    continue
  fi

  if jq -e --arg id "$id" '.results[] | select(.vector_id == $id and .status == "skip")' "$report" >/dev/null; then
    skip_required="${skip_required}${id}"$'\n'
  fi
done <<< "$required_ids"

if [ -n "$missing_required" ]; then
  echo "error: missing required vectors:" >&2
  echo "$missing_required" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

if [ -n "$skip_required" ]; then
  echo "error: required vectors must not be reported as skip:" >&2
  echo "$skip_required" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

invalid_skip=""
while IFS=$'\t' read -r id status; do
  [ -z "$id" ] && continue
  if [ "$status" = "skip" ] && ! grep -Fxq "$id" <<< "$optional_ids"; then
    invalid_skip="${invalid_skip}${id}"$'\n'
  fi
done < <(jq -r '.results[] | [.vector_id, .status] | @tsv' "$report")

if [ -n "$invalid_skip" ]; then
  echo "error: skip is only allowed for optional vectors:" >&2
  echo "$invalid_skip" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

echo "ok: interop report passed semantic checks against $vector_set (profile=$profile_id)"
