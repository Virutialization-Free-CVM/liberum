#!/bin/bash
# SPDX-FileCopyrightText: 2026 The Salus Contributors
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

. "$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common_variables"

TELLUS_WASMRT_IMAGE=${TELLUS_WASMRT_IMAGE:-${ROOT_DIR}/.run/tellus_wasmrt_reject_guest}
CREATE_GUEST_IMAGE_BIN=${CREATE_GUEST_IMAGE_BIN:-${TELLUS_BINS}create_guest_image}
TELLUS_WASMRT_REJECT_RAW=${TELLUS_WASMRT_REJECT_RAW:-${TELLUS_BINS}tellus_wasmrt_reject_raw.out}
WASMRT_GUEST_RAW=${WASMRT_GUEST_RAW:-${TELLUS_BINS}wasmrt_guest_raw.out}
TELLUS_MAX_SIZE=$((512 * 4096))

mkdir -p "$(dirname "${TELLUS_WASMRT_IMAGE}")"

if [[ ! -f "${SALUS_BINS}salus" ]]; then
    echo "Missing ${SALUS_BINS}salus. Run ./scripts/build_liberum_all.sh first." >&2
    exit 1
fi

if [[ ! -x "${CREATE_GUEST_IMAGE_BIN}" ]]; then
    echo "Missing ${CREATE_GUEST_IMAGE_BIN}. Run ./scripts/build_liberum_all.sh first." >&2
    exit 1
fi

if [[ ! -f "${TELLUS_WASMRT_REJECT_RAW}" ]]; then
    echo "Missing ${TELLUS_WASMRT_REJECT_RAW}. Run ./scripts/build_liberum_all.sh first." >&2
    exit 1
fi

if [[ ! -f "${WASMRT_GUEST_RAW}" ]]; then
    echo "Missing ${WASMRT_GUEST_RAW}. Run ./scripts/build_liberum_all.sh first." >&2
    exit 1
fi

"${CREATE_GUEST_IMAGE_BIN}" \
    "${TELLUS_WASMRT_REJECT_RAW}" \
    "${WASMRT_GUEST_RAW}" \
    "${TELLUS_WASMRT_IMAGE}" \
    "${TELLUS_MAX_SIZE}"

${QEMU_BIN} \
    ${MACH_ARGS} \
    -kernel ${SALUS_BINS}salus \
    -device guest-loader,kernel=${TELLUS_WASMRT_IMAGE},addr=${KERNEL_ADDR} \
    ${IOMMU_ARGS} \
    ${EXTRA_QEMU_ARGS}
