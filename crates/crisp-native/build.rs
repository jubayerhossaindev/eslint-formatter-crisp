fn main() {
    // napi-sys v3 loads N-API symbols at runtime from the host Node process
    // via libloading, so the cdylib/lib does NOT need link-time libnode.
    // The @napi-rs/cli handles native compilation; no napi_build::setup()
    // (v2) or cargo:rustc-link-lib=dylib=node is required.
}
