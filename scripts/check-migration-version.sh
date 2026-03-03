#!/usr/bin/env bash
set -euo pipefail

latest_file_version=""

if ls migrations/*.sql >/dev/null 2>&1; then
  latest_file_version=$(ls migrations/*.sql | sed -E 's#.*/([0-9]+)_.*#\1#' | sort | tail -n 1)
fi

if [[ -z "$latest_file_version" ]]; then
  echo "ERROR: no SQL migration files found under migrations/"
  exit 1
fi

if [[ ! -f migrations/LATEST_VERSION ]]; then
  echo "ERROR: migrations/LATEST_VERSION not found"
  exit 1
fi

latest_declared_version=$(tr -d '[:space:]' < migrations/LATEST_VERSION)

if [[ "$latest_file_version" != "$latest_declared_version" ]]; then
  echo "ERROR: migrations/LATEST_VERSION mismatch"
  echo "  latest sql file version: $latest_file_version"
  echo "  declared version:        $latest_declared_version"
  exit 1
fi

echo "OK: migration versions are consistent ($latest_declared_version)"
