# Liberum

Liberum is the parent repository for the component/sandbox-oriented
confidential execution prototype. The current implementation uses Salus as a
backend substrate, but the repo boundary is already split so that Liberum owns
the product narrative, integration entrypoints, and future sandbox-specific
code.

## Repository Role

- `salus/`
  - backend submodule
  - low-level HS-mode substrate, CoVE lifecycle, page tracking, measurement
- parent repo
  - Liberum-specific docs
  - build/run entrypoints
  - future component policy and sandbox/runtime code

At the moment, the executable PoC still builds in `salus/`. This parent repo is
the integration surface and the destination for code that should stop living in
the backend.

The parent repo is now also a Bazel workspace and exposes a parent-owned
`liberum-core/` package. The `salus/` submodule is wired to it via
`@liberum_parent`, so future extractions can move code without changing the repo
boundary again.

## Quick Start

Initialize the submodule if needed:

```bash
git submodule update --init --recursive
```

Build the current minimal Liberum demo:

```bash
./scripts/build_liberum_all.sh
```

Run the Tellus-based Wasm demo:

```bash
QEMU=~/repo/qemu ./scripts/run_tellus_wasmrt.sh
```

The current execution path is:

`salus backend -> tellus_wasmrt host workload -> wasmrt_guest confidential guest`

## Repo Layout

- `docs/`
  - architecture and ownership notes for the parent repo
- `scripts/`
  - parent-level wrappers around submodule build and run flows
- `salus/`
  - backend implementation submodule

See [docs/repo-layout.md](/home/funera1/repo/vfc/liberum/docs/repo-layout.md)
for the split between parent repo and backend.

## Current Status

- The minimal demo runs without Linux using a Tellus-style host workload.
- A tiny guest-side Wasm interpreter runs an embedded `.wasm` payload.
- Salus is still the execution backend.
- Liberum-specific code is only partially extracted; more code still lives in
  `salus/` and should move out over time.

## Near-Term Cleanup

- move `liberum-core` out of `salus/`
- move sandbox/runtime adapters out of `salus/`
- keep only thin backend hooks inside the submodule
