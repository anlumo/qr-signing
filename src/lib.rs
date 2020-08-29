#![recursion_limit = "512"]

use wasm_bindgen::prelude::*;
use yew::prelude::*;

mod app;
mod crypto;
mod html5_qrcode;
mod qr_generator;
mod qr_reader;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    App::<app::Main>::new().mount_to_body();

    Ok(())
}

fn subtle() -> web_sys::SubtleCrypto {
    let window = web_sys::window().unwrap();
    window
        .crypto()
        .expect("No WebCrypto support found!")
        .subtle()
}
