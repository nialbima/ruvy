mod runtime;

use runtime::cleanup_ruby;
use std::{env, io, sync::OnceLock};

static USER_CODE: OnceLock<String> = OnceLock::new();

fn main() {
    let code = USER_CODE.get().unwrap();
    runtime::eval(code).unwrap();
    cleanup_ruby().unwrap();
}

#[export_name = "wizer.initialize"]
pub extern "C" fn load_user_code() {
    let _wasm_ctx = WasmCtx::new();

    runtime::init_ruby();

    // so, like, that's how you preload a Ruby file.
    // Doesn't that mean you'd just preload the gems/requires?
    // If Gemfile path, bundle install + `runtime::preload_files(bundle show --paths)`
    // If require, load.
    if let Ok(preload_path) = env::var("RUVY_PRELOAD_PATH") {
        runtime::preload_files(preload_path);
    }

    let contents = io::read_to_string(io::stdin()).unwrap();
    USER_CODE.set(contents).unwrap();
}

// RAII abstraction for calling Wasm ctors and dtors for exported non-main functions.
struct WasmCtx;

impl WasmCtx {
    #[must_use = "Failing to assign the return value will result in the wasm dtors being run immediately"]
    fn new() -> Self {
        unsafe { __wasm_call_ctors() };
        Self
    }
}

impl Drop for WasmCtx {
    fn drop(&mut self) {
        unsafe { __wasm_call_dtors() };
    }
}

extern "C" {
    // `__wasm_call_ctors` is generated by `wasm-ld` and invokes all of the global constructors.
    // In a Rust bin crate, the `_start` function will invoke this implicitly but no other exported
    // Wasm functions will invoke this.
    // If this is not invoked, access to environment variables and directory preopens will not be
    // available.
    // This should only be invoked at the start of exported Wasm functions that are not the `main`
    // function.
    // References:
    // - [Rust 1.67.0 stopped initializing the WASI environment for exported functions](https://github.com/rust-lang/rust/issues/107635)
    // - [Wizer header in Fastly's JS compute runtime](https://github.com/fastly/js-compute-runtime/blob/main/runtime/js-compute-runtime/third_party/wizer.h#L92)
    fn __wasm_call_ctors();

    fn __wasm_call_dtors();
}
