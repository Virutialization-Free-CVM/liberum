package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_doc", "rust_library", "rustfmt_test")

rust_library(
    name = "liberum-core",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "liberum_core",
    edition = "2021",
    deps = [
        "@salus-index//:arrayvec",
    ],
)

rust_clippy(
    name = "clippy",
    deps = ["liberum-core"],
)

rustfmt_test(
    name = "rustfmt",
    targets = ["liberum-core"],
)

rust_doc(
    name = "liberum-core-doc",
    crate = ":liberum-core",
)
