package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "liberum-sandbox-wasm",
    srcs = ["src/lib.rs"],
    crate_name = "liberum_sandbox_wasm",
    edition = "2021",
    deps = [
        "//liberum-backend-salus",
        "//liberum-core",
    ],
)
