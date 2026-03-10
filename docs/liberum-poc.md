# Liberum Minimal PoC Plan

## Goal

Primary deadline scope is one end-to-end confidential execution demo with these properties:

1. only finalized legitimate entries are accepted
2. synthetic trap or interrupt causes a trusted exit path
3. one lightweight sandbox type, initially Wasm, can run as a confidential component

This is explicitly not a full VM replacement yet. The first milestone is a component-centric control plane with enough evidence that the hard part is being built.

## Minimal Architecture

The minimum path should be described as:

`host/test driver -> Liberum monitor -> entry shim -> runtime core -> verify_jmp -> guest image/Wasm -> synthetic trap/interrupt -> exit shim -> host`

The component model should be:

- `entry_shim`: tiny fixed landing pad
- `runtime_core`: trusted runtime wrapper for sandbox setup and policy checks
- `guest_image`: sandbox payload, initially a Wasm blob or prevalidated runtime target
- `exit_shim`: forced return target for trusted exits

Salus internals remain below this line as implementation substrate:

- page ownership / measurement extension
- trap decoding
- minimal run loop / CPU context switching

The external API and logs should speak in terms of `component` and `sandbox`, not `VM` or `vCPU`.

## Suggested Modules

Deadline-one module split:

- `src/liberum.rs`
  - component descriptor
  - finalize and verify policy
  - trusted-exit state
  - demo log events
- existing `src/vm.rs`
  - temporary execution backend
  - trap decoding and resumable exit causes
  - measurement finalization plumbing already present
- existing `src/vm_cpu.rs`
  - temporary execution context switch path
  - synthetic interrupt / trap injection hooks
- future `src/liberum_backend.rs`
  - maps Liberum API onto current Salus VM machinery without leaking VM semantics outward
- future `test-workloads`
  - smallest no-I/O Wasm sample and synthetic trap trigger

## Minimal Descriptor

Required descriptor fields for the first milestone:

- `component_id`
- `component_type`
- `sandbox_type`
- `entry_pc`
- `exit_pc`
- `code_region`
- `finalized`
- `measurement`

Fields that can wait:

- rich policy flags
- nested component ownership
- hostcall permission bitmap
- full attestation report metadata

## State Machine

The state machine should stay small:

`Created -> Loaded -> Finalized -> Running -> Exited`

Key invariants:

- `verify_jmp()` only succeeds for finalized descriptors
- target PC must match registered `entry_pc`
- target PC must be inside frozen code region
- measurement must match frozen digest
- confidential run sets an active sandbox flag
- synthetic interrupt or trap never returns directly to host while that flag is set
- forced exit always lands on registered exit shim

## Shortest Implementation Path

1. Introduce the component-centric API and descriptor store.
2. Wire `finalize_component()` to the existing measurement finalization flow.
3. Implement `verify_jmp()` as a monitor helper, not a real ISA instruction yet.
4. Register exactly one entry shim and one exit shim.
5. Wrap existing run path so the demo enters via `entry_shim -> runtime_core`.
6. Trigger a synthetic interrupt or synthetic trap while confidential flag is active.
7. On trap detection, redirect to `exit_shim` and emit explicit logs.
8. Run a no-I/O Wasm sample through the same path.

## What To Reuse From Salus

Reuse as-is or nearly as-is:

- `attestation::manager`
  - measurement extension and finalize flow
- `src/vm.rs::finalize()`
  - finalization ordering and entry capture
- `src/vm.rs::run_vcpu()`
  - temporary execution loop and exit reason handling
- `src/vm_cpu.rs`
  - low-level context switch and trap extraction
- `src/trap.rs`
  - host-side trap diagnostics and unexpected-trap visibility

## What To Hide Or Rename

Do not expose these concepts in the new top-level API:

- `vm_create`
- `vm_run`
- `vcpu`
- guest physical memory as the main abstraction
- VM entry / VM exit naming

Internal adaptation is acceptable, but the new control plane should present:

- `create_component`
- `load_component`
- `finalize_component`
- `register_entry_shim`
- `register_exit_shim`
- `authorize_transition` or `verify_jmp`
- `begin_confidential_run`
- `force_trusted_exit`

## Dummy Implementations Allowed

Safe shortcuts for the first deadline:

- `verify_jmp()` as a Rust helper or monitor call
- synthetic trap source instead of full hardware interrupt virtualization
- hardcoded Wasm sample path
- hardcoded one-sandbox enum variant
- fake attestation CDI inputs, as already done in current `Vm`
- minimal measurement comparison with precomputed digest bytes
- no hostcall / no WASI

## Defer To Later Milestones

Push these out of the first deadline:

- ISA-accurate `verify_jmp` instruction semantics
- QEMU or hardware-backed instruction extensions
- full trap classification and reinjection model
- multiple sandbox runtimes
- performance work
- full attestation evidence and report plumbing
- generalized host ABI
- polished memory encryption story

## Demo Logging

The demo should log these events in order:

1. component create/load/finalize
2. entry shim registration
3. exit shim registration
4. `verify_jmp accepted` for legal path
5. `verify_jmp rejected` for tampered entry or non-finalized component
6. `entered confidential`
7. `forced trusted exit` with explicit reason and exit PC
8. Wasm completion or trusted exit completion

The audience should be able to read the log and confirm the four target behaviors without inspecting internals.

## Immediate Next Code Steps

- make `src/liberum.rs` the single source of truth for descriptor semantics
- add an adapter layer that maps one Liberum component to the current single-VM execution path
- add one synthetic trap injection point in the current run loop
- add one test workload that exercises legal entry, illegal entry, and forced trusted exit
