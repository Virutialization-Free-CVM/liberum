# Repo Layout

## Intent

This repository is the Liberum parent repo.

- `salus/` is the backend substrate submodule.
- parent-level files are for Liberum-specific orchestration, docs, and future
  component/sandbox code.

## Current Split

### Parent repo owns

- top-level project entrypoints
- demo/run wrappers
- architecture and ownership docs
- future component/sandbox policy code
- future Wasm runtime integration code

### `salus/` submodule owns

- low-level HS-mode execution backend
- CoVE lifecycle machinery
- page ownership and page tracking
- measurement plumbing
- trap/context-switch substrate

## Target Ownership

### Keep in `salus/`

- CSR and trap handling
- vCPU/guest run substrate
- confidential page management
- CoVE/TSM backend implementation
- thin backend hook points needed by Liberum

### Move to parent repo

- `liberum-core`
- component descriptors and lifecycle policy
- `verify_jmp` meaning and transition policy
- sandbox/runtime adapters
- Wasm-specific policy and demo logic
- integration scripts and documentation

## Integration Rule

The parent repo should treat `salus/` as a backend, not as the primary product
surface. New Liberum concepts should default to the parent repo unless they are
unavoidably tied to HS-mode execution details.

## Current Demo Command

```bash
./scripts/build_liberum_all.sh
QEMU=~/repo/qemu ./scripts/run_tellus_wasmrt.sh
```

## Next Cleanup Step

The next meaningful extraction is moving `liberum-core` out of `salus/` and
replacing direct backend-facing policy logic with thin hook points.

## Bazel Seam

- parent repo workspace name: `liberum_parent`
- submodule import path: `@liberum_parent//...`
- first extracted package: `@liberum_parent//liberum-core`
