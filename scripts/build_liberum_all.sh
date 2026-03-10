#!/bin/bash
# SPDX-FileCopyrightText: 2026 The Salus Contributors
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
SALUS_DIR=${SALUS_DIR:-${ROOT_DIR}/salus}

if [[ ! -d "${SALUS_DIR}" ]]; then
    echo "Missing ${SALUS_DIR}. Run ./scripts/bootstrap.sh first." >&2
    exit 1
fi

cd "${SALUS_DIR}"
bazelisk build //:liberum-all
bazelisk build //test-workloads:tellus_wasmrt_reject_raw
