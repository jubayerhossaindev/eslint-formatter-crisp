fn main() {
  // napi_build::setup() unconditionally panics for the `*-pc-windows-gnu`
  // target when libnode.dll isn't on disk (it emits `cargo:rustc-link-lib=node`
  // and aborts if the dll can't be found). That link dependency is unnecessary
  // for our addon: napi-sys already loads every N-API symbol at runtime via
  // `libloading` from the host `node.exe` process, so the cdylib doesn't need
  // any link-time reference to libnode. Skip the setup call when cross-building
  // for windows-gnu; macOS gets its `-undefined dynamic_lookup` from the same
  // function so we only bypass it on the gnu-windows target.
  let target = std::env::var("TARGET").unwrap_or_default();
  let is_windows_gnu = target.ends_with("-pc-windows-gnu");
  if !is_windows_gnu {
    napi_build::setup();
  }
}
