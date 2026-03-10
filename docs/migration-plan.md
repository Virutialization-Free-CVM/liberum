# Migration Plan

## Goal

Make the Liberum parent repo the home of component/sandbox policy and keep the
`salus/` submodule as a backend substrate.

## Phase 1

- parent repo becomes a Bazel workspace
- parent repo exposes `liberum-core`
- `salus/` can reference the parent repo via `@liberum_parent`

## Phase 2

- move actual Liberum policy code into parent `liberum-core`
- update `salus/BUILD` and `salus/test-workloads/BUILD` to consume it
- leave only thin backend hooks inside `salus/`
