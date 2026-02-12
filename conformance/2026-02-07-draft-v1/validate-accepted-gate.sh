#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 2
fi

if [ "$#" -lt 1 ] || [ "$#" -gt 5 ]; then
  echo "usage: $0 <reports-dir> [vector-set.json] [profile-id] [min-implementations] [report-glob]" >&2
  exit 2
fi

reports_dir="$1"
vector_set="${2:-$(dirname "$0")/vector-set.json}"
profile_id="${3:-amp-full-stack-001-011}"
min_impls="${4:-2}"
report_glob="${5:-}"
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

if ! [[ "$min_impls" =~ ^[0-9]+$ ]] || [ "$min_impls" -lt 1 ]; then
  echo "error: min-implementations must be a positive integer: $min_impls" >&2
  exit 2
fi

if [ -z "$report_glob" ]; then
  if [ "$profile_id" = "amp-full-stack-001-011" ]; then
    report_glob="*.full-stack-pass.*.json"
  else
    report_glob="*.core-pass.*.json"
  fi
fi

required_field="$(jq -r --arg id "$profile_id" '.profiles[] | select(.id == $id) | .required_field' "$vector_set")"
if [ -z "$required_field" ] || [ "$required_field" = "null" ]; then
  echo "error: unknown profile-id: $profile_id" >&2
  echo "available profiles:" >&2
  jq -r '.profiles[].id' "$vector_set" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

required_ids="$(jq -r --arg field "$required_field" '.[$field][]' "$vector_set")"

reports_list="$(find "$reports_dir" -maxdepth 1 -type f -name "$report_glob" | sort | grep -v '\.template\.' || true)"
if [ -z "$reports_list" ]; then
  echo "error: no report JSON files found under: $reports_dir (glob=$report_glob)" >&2
  exit 1
fi
report_count="$(printf '%s\n' "$reports_list" | sed '/^$/d' | wc -l | tr -d ' ')"

impl_ids=""
required_failures=""

for report in $reports_list; do
  "$validator" "$report" "$vector_set" "$profile_id" >/dev/null

  impl_id="$(jq -r '.implementation.id' "$report")"
  impl_version="$(jq -r '.implementation.version' "$report")"
  impl_commit="$(jq -r '.implementation.commit // ""' "$report")"
  executed_at="$(jq -r '.run.executed_at' "$report")"
  env_os="$(jq -r '.run.environment.os' "$report")"
  env_arch="$(jq -r '.run.environment.arch' "$report")"
  env_runtime="$(jq -r '.run.environment.runtime' "$report")"
  env_notes="$(jq -r '.run.environment.notes // ""' "$report")"

  if [ -z "$impl_id" ] || [ "$impl_id" = "null" ]; then
    echo "error: missing implementation.id in report: $report" >&2
    exit 1
  fi

  if [ "$impl_version" = "0.0.0-template" ] || [ "$impl_commit" = "replace-with-commit" ] || [ "$executed_at" = "1970-01-01T00:00:00Z" ]; then
    echo "error: placeholder metadata is not allowed for accepted gate: $report" >&2
    exit 1
  fi

  if [ "$env_os" = "replace" ] || [ "$env_arch" = "replace" ] || [ "$env_runtime" = "replace" ]; then
    echo "error: placeholder environment metadata is not allowed for accepted gate: $report" >&2
    exit 1
  fi

  if echo "$env_notes" | grep -qi "template"; then
    echo "error: template marker found in report notes; not allowed for accepted gate: $report" >&2
    exit 1
  fi

  impl_ids="${impl_ids}${impl_id}"$'\n'

  while IFS= read -r vector_id; do
    [ -z "$vector_id" ] && continue
    status="$(jq -r --arg id "$vector_id" '.results[] | select(.vector_id == $id) | .status' "$report")"
    if [ "$status" != "pass" ]; then
      required_failures="${required_failures}${report}: ${vector_id} => ${status}"$'\n'
    fi
  done <<< "$required_ids"
done

unique_impls_count="$(printf '%s' "$impl_ids" | sed '/^$/d' | sort -u | wc -l | tr -d ' ')"
if [ "$unique_impls_count" -lt "$min_impls" ]; then
  echo "error: accepted gate requires at least ${min_impls} independent implementations; found ${unique_impls_count}" >&2
  echo "implementations found:" >&2
  printf '%s' "$impl_ids" | sed '/^$/d' | sort -u | sed 's/^/  - /' >&2
  exit 1
fi

if [ -n "$required_failures" ]; then
  echo "error: accepted gate requires all required vectors to pass for profile ${profile_id}" >&2
  echo "$required_failures" | sed '/^$/d; s/^/  - /' >&2
  exit 1
fi

echo "ok: accepted gate satisfied (profile=${profile_id}, reports=${report_count}, implementations=${unique_impls_count})"
