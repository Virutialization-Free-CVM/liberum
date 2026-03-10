package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "liberum-wasmrt-guest",
    srcs = ["src/lib.rs"],
    crate_name = "liberum_wasmrt_guest",
    edition = "2021",
    deps = [
        "@salus//s-mode-utils",
        "@salus//sbi-rs",
    ],
)
