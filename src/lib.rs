#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod core;
mod shaders;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("Couldn't initialize logger");
        wasm_bindgen_futures::spawn_local(core::window::run());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(core::window::run());
    }
}
