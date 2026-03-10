#!/bin/bash
# SPDX-FileCopyrightText: 2026 The Liberum Contributors
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

cd "${ROOT_DIR}"
git submodule update --init --recursive

echo "Liberum bootstrap complete."
echo "Next:"
echo "  ./scripts/build_liberum_all.sh"
echo "  QEMU=~/repo/qemu ./scripts/run_tellus_wasmrt.sh"
