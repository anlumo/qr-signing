use js_sys::{Array, Uint8Array};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, Blob, CryptoKeyPair, Event, Url};

mod html5_qrcode;
use html5_qrcode::Html5QrcodeScanner;
mod crypto;
mod qr_generator;

thread_local! {
    static SCANNER: Html5QrcodeScanner = html5_qrcode::Html5QrcodeScanner::new(
        "reader",
        true,
    );
    static KEY_PAIR: RefCell<Option<CryptoKeyPair>> = RefCell::new(None);
}

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let scanned_closure = Closure::wrap(Box::new(scanned) as Box<dyn FnMut(String)>);
    let error_closure = Closure::wrap(
        Box::new(move |err: JsValue| console::log_1(&err)) as Box<dyn FnMut(JsValue)>
    );

    SCANNER.with(|scanner| {
        scanner.render(&scanned_closure, &error_closure);
    });

    scanned_closure.forget();
    error_closure.forget();

    let document = web_sys::window().unwrap().document().unwrap();

    if let Some(generate_button) = document.get_element_by_id("generate") {
        let event_handler =
            Closure::wrap(
                Box::new(|| wasm_bindgen_futures::spawn_local(generate_keypair()))
                    as Box<dyn FnMut()>,
            );

        generate_button
            .add_event_listener_with_callback("click", event_handler.as_ref().unchecked_ref())
            .unwrap();

        event_handler.forget();
    }

    if let Some(sign_form) = document.get_element_by_id("sign") {
        let event_handler = Closure::wrap(Box::new(|event: Event| {
            event.prevent_default();
            if let Some(input) = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("text")
            {
                let input: web_sys::HtmlInputElement = input.unchecked_into();
                let text = input.value();
                wasm_bindgen_futures::spawn_local(async move {
                    sign_text(&text).await.expect("Failed signing");
                });
            }
            false
        }) as Box<dyn FnMut(Event) -> bool>);

        sign_form
            .add_event_listener_with_callback("submit", event_handler.as_ref().unchecked_ref())
            .unwrap();

        event_handler.forget();
    }

    Ok(())
}

fn scanned(text: String) {
    console::log_2(&JsValue::from_str("SCANNED:"), &JsValue::from_str(&text));
}

async fn generate_keypair() {
    if let Some(window) = web_sys::window() {
        let subtle = window
            .crypto()
            .expect("No WebCrypto support found!")
            .subtle();

        match crypto::generate_keypair(&subtle).await {
            Ok(new_key_pair) => {
                KEY_PAIR.with(|key_pair| {
                    *key_pair.borrow_mut() = Some(new_key_pair);
                });
            }
            Err(err) => {
                console::error_2(&JsValue::from_str("Failed generating keypair:"), &err);
            }
        }
    }
}

async fn sign_text(text: &str) -> Result<(), JsValue> {
    let subtle = web_sys::window()
        .unwrap()
        .crypto()
        .expect("No WebCrypto support found!")
        .subtle();

    if let Some(key_pair) = KEY_PAIR.with(|key_pair| key_pair.borrow().clone()) {
        match crypto::sign(&subtle, &key_pair, text).await {
            Err(err) => {
                console::error_1(&err);
            }
            Ok(data) => {
                let mut array = Uint8Array::new(data.as_ref()).to_vec(); // always 132 bytes
                array.extend_from_slice(text.as_bytes()); // append original text
                
                let svg = qr_generator::encode_data(&array).expect("Text too long");

                let mut options = web_sys::BlobPropertyBag::new();
                options.type_("image/svg+xml");
                let blob = Blob::new_with_str_sequence_and_options(
                    &Array::of1(&JsValue::from_str(&svg)),
                    &options,
                )?;

                let blob_url = Url::create_object_url_with_blob(&blob)?;

                let a: web_sys::HtmlAnchorElement = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .create_element("A")
                    .unwrap()
                    .unchecked_into();
                a.set_href(&blob_url);
                a.set_download("signed_qr.svg");
                a.click();

                Url::revoke_object_url(&blob_url)?;
            }
        }
    }
    Ok(())
}
