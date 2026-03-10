# Liberum TODO

This file tracks the current Liberum PoC status against the primary deadline.

## Goal

Deliver one end-to-end demo that shows:

- finalized legitimate components are allowed to enter
- tampered or otherwise illegitimate entry is rejected
- execution-time trap or interrupt is converted into a trusted exit
- a minimal Wasm workload runs inside the confidential sandbox path

## Current Status

### Repo split and ownership

- [x] Create a parent `liberum/` repo with `salus/` as a backend submodule
- [x] Add parent-level build and run entrypoints
- [x] Make the parent repo a Bazel workspace
- [x] Wire the Salus submodule to parent-owned packages via `@liberum_parent`
- [x] Move `liberum-core` ownership to the parent repo
- [x] Move unused sandbox/backend demo libraries to the parent repo
- [x] Move Tellus Wasm host orchestration to the parent repo
- [x] Move Wasm guest demo logic to the parent repo
- [x] Remove duplicate Salus-side files that already moved to the parent repo

### Minimal Liberum component model

- [x] Define component descriptors
- [x] Define minimal component lifecycle
- [x] Define authorized transition graph
- [x] Define confidential execution context
- [x] Implement `create_component`
- [x] Implement `load_component`
- [x] Implement `finalize_component`
- [x] Implement `register_entry_shim`
- [x] Implement `register_exit_shim`
- [x] Implement `authorize_transition`
- [x] Implement `verify_jmp`
- [x] Implement `begin_confidential_run`
- [x] Implement `force_trusted_exit`

### Minimal Wasm demo

- [x] Build a Linux-free Tellus-style host route
- [x] Launch a confidential guest through the existing CoVE backend
- [x] Run a minimal embedded Wasm module inside the guest
- [x] Print a guest-side challenge-response using host nonce + guest secret + Wasm result
- [x] Provide a one-command parent build wrapper
- [x] Provide a normal demo run script
- [x] Provide a reject-path demo run script

### Evidence for the primary deadline

- [x] Success path: normal entry reaches guest-side Wasm execution
- [x] Reject path: tampered entry PC is refused before guest execution
- [x] Forced-exit path: trap is converted into a trusted-exit log path
- [ ] Host-side validation of the returned challenge-response value
- [ ] Single document that explains the three demo paths with expected logs

## In Progress

- [ ] Keep Salus-side Liberum code limited to thin wrappers and backend hooks
- [ ] Reduce ad hoc demo wiring still living in `salus/test-workloads/BUILD`

## Next Priority

### 1. Tighten the demo claims

- [ ] Add explicit host-side check that the guest-reported sandbox response matches the expected value
- [ ] Make the reject-path expected log easy to compare with the success path
- [ ] Add a dedicated forced-exit demo mode instead of relying on the current generic trap path

### 2. Strengthen `verify_jmp` evidence

- [x] Entry PC tamper reject demo
- [ ] Unfinalized component reject demo
- [ ] Measurement mismatch reject demo

### 3. Clarify the trusted-exit story

- [ ] Distinguish normal completion from forced trusted exit more clearly in logs
- [ ] Add an exit-shim-specific observable event or log marker
- [ ] Decide whether to keep trusted-exit as demo semantics only or add a thinner backend hook

### 4. Clean up parent/submodule boundaries

- [ ] Move remaining Liberum-specific docs references out of the Salus narrative
- [ ] Audit scripts so the parent repo is always the primary entrypoint
- [ ] Keep Salus-side changes focused on workload wrappers, BUILD wiring, and backend hooks only

### 5. Stretch work after the primary deadline

- [ ] Replace the tiny Wasm interpreter with a more realistic runtime core
- [ ] Evaluate a minimal WAMR-based runtime path
- [ ] Replace helper-style `verify_jmp` with a more realistic monitor or ISA path
- [ ] Strengthen trap/interrupt handling beyond the current PoC path
- [ ] Explore encrypted component loading or stronger memory protection semantics

## Salus Side: What Should Remain

- low-level HS-mode execution backend
- CoVE lifecycle substrate
- page ownership and memory tracking
- trap and context-switch machinery
- thin workload wrappers
- thin BUILD and WORKSPACE hooks into parent-owned Liberum packages

## Liberum Side: What Should Own the Model

- component descriptors and state machine
- transition policy and `verify_jmp` semantics
- trusted-exit meaning
- Wasm demo orchestration
- host and guest demo libraries
- top-level docs and scripts
