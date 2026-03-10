# Ownership Split

## Principle

Put substrate in `salus/`. Put model and policy in the Liberum parent repo.

## `salus/` Submodule

- HS-mode execution backend
- trap entry and context switching
- CoVE guest lifecycle substrate
- page ownership and memory management
- measurement plumbing
- hardware-facing initialization

## Parent Repo

- component model
- sandbox model
- `liberum-sandbox-wasm` demo library
- `liberum-backend-salus` backend adapter library
- `liberum-tellus-wasmrt` host workload library
- `liberum-wasmrt-guest` guest payload library
- transition policy
- `verify_jmp` meaning
- trusted-exit meaning
- Wasm/runtime integration
- demo orchestration
- project documentation

## Thin Hooks Allowed in `salus/`

- `before_enter`
- `after_exit`
- `on_finalize`
- `on_trap`

These hooks should export facts to Liberum. They should not own Liberum policy.
