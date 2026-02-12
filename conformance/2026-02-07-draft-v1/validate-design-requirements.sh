#!/usr/bin/env bash
set -euo pipefail

# Current-cycle design requirements for RFC 001-011 are evaluated at Draft gate.
script_dir="$(cd "$(dirname "$0")" && pwd)"

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <reports-dir> [vector-set.json]" >&2
  exit 2
fi

reports_dir="$1"
vector_set="${2:-$script_dir/vector-set.json}"

"$script_dir/validate-draft-gate.sh" "$reports_dir" "$vector_set"
echo "ok: current-cycle design requirements satisfied (Draft gate)"
