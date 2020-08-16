use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Html5QrcodeScanner;

    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, verbose: bool) -> Html5QrcodeScanner;

    #[wasm_bindgen(method)]
    pub fn render(
        this: &Html5QrcodeScanner,
        on_scan_success: &Closure<dyn FnMut(String)>,
        on_scan_failure: &Closure<dyn FnMut(JsValue)>,
    );
}
