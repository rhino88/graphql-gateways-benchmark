#!/usr/bin/env bash
set -Eeuo pipefail

export HIVE__SUPERGRAPH__SOURCE=file
export HIVE__SUPERGRAPH__PATH=./supergraph.graphql

# Recent versions at `info` also emit a per-request access log line (`router::request`).
# Other gateways run without access logs, so mute them here too.
exec env -i RUST_LOG=info LOG_FILTER=router::request=off ./hive_router > ./gateway_log.txt 2>&1
