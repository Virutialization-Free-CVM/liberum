package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "liberum-tellus-wasmrt",
    srcs = ["src/lib.rs"],
    crate_name = "liberum_tellus_wasmrt",
    edition = "2021",
    deps = [
        "//liberum-core",
        "@salus//device-tree",
        "@salus//riscv-regs",
        "@salus//s-mode-utils",
        "@salus//sbi-rs",
        "@salus//test-workloads",
    ],
)
