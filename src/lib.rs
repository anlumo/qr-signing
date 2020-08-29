#![recursion_limit = "512"]

use wasm_bindgen::prelude::*;
use web_sys::console;
use yew::prelude::*;

mod html5_qrcode;
use html5_qrcode::Html5QrcodeScanner;
mod app;
mod crypto;
mod qr_generator;

thread_local! {
    static SCANNER: Html5QrcodeScanner = html5_qrcode::Html5QrcodeScanner::new(
        "reader",
        true,
    );
}

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    App::<app::Main>::new().mount_to_body();

    // let scanned_closure = Closure::wrap(Box::new(scanned) as Box<dyn FnMut(String)>);
    // let error_closure = Closure::wrap(
    //     Box::new(move |err: JsValue| console::log_1(&err)) as Box<dyn FnMut(JsValue)>
    // );

    // SCANNER.with(|scanner| {
    //     scanner.render(&scanned_closure, &error_closure);
    // });

    // scanned_closure.forget();
    // error_closure.forget();

    Ok(())
}

fn scanned(text: String) {
    console::log_2(&JsValue::from_str("SCANNED:"), &JsValue::from_str(&text));
}
