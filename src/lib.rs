use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;
use web_sys::console;

mod html5_qrcode;
use html5_qrcode::Html5QrcodeScanner;
mod crypto;

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

    wasm_bindgen_futures::spawn_local(async move {
        if let Some(window) = web_sys::window() {
            let subtle = window
                .crypto()
                .expect("No WebCrypto support found!")
                .subtle();

            match crypto::generate_keypair(&subtle).await {
                Ok(key_pair) => {
                    let public_key = crypto::export_public_key(&subtle, &key_pair)
                        .await
                        .expect("Failed exporting public key");
                    console::log_1(Uint8Array::new(public_key.as_ref()).as_ref());
                }
                Err(err) => {
                    console::error_2(&JsValue::from_str("Failed generating keypair:"), &err);
                }
            }
        }
    });

    let scanned_closure = Closure::wrap(Box::new(scanned) as Box<dyn FnMut(String)>);
    let error_closure = Closure::wrap(
        Box::new(move |err: JsValue| console::log_1(&err)) as Box<dyn FnMut(JsValue)>
    );

    SCANNER.with(|scanner| {
        scanner.render(&scanned_closure, &error_closure);
    });

    scanned_closure.forget();
    error_closure.forget();

    Ok(())
}

fn scanned(text: String) {
    console::log_2(&JsValue::from_str("SCANNED:"), &JsValue::from_str(&text));
}
