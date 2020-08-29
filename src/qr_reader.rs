use crate::{crypto, html5_qrcode::Html5QrcodeScanner, subtle};
use uuid::Uuid;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::CryptoKey;
use yew::prelude::*;

pub struct QrReader {
    link: ComponentLink<Self>,
    qr_ref: NodeRef,
    reader_id: Uuid,
    scanner: Option<Html5QrcodeScanner>,
    scanned_closure: Closure<dyn FnMut(String)>,
    error_closure: Closure<dyn FnMut(JsValue)>,
    onpublickey: Callback<CryptoKey>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub onpublickey: Callback<CryptoKey>,
}

pub enum Msg {
    GotQRText(String),
}

impl Component for QrReader {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let inner_link = link.clone();
        Self {
            link,
            qr_ref: NodeRef::default(),
            reader_id: Uuid::new_v4(),
            scanner: None,
            scanned_closure: Closure::wrap(Box::new(move |text| {
                inner_link.send_message(Msg::GotQRText(text));
            }) as Box<dyn FnMut(String)>),
            error_closure: Closure::wrap(
                Box::new(move |err: JsValue| web_sys::console::log_1(&err))
                    as Box<dyn FnMut(JsValue)>,
            ),
            onpublickey: props.onpublickey,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GotQRText(text) => {
                if text.len() > 5 {
                    let binary: Vec<u8> = text.chars().map(|c| c as u8).collect();
                    if &binary[0..4] == b"PUB:" {
                        let onpublickey = self.onpublickey.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let public_key = &binary[4..];
                            let hash = hmac_sha256::Hash::hash(public_key);
                            web_sys::console::log_2(
                                &wasm_bindgen::JsValue::from_str("Public Key Hash"),
                                &wasm_bindgen::JsValue::from_str(
                                    &hash
                                        .iter()
                                        .map(|byte| format!("{:02X}", byte))
                                        .collect::<Vec<String>>()
                                        .join(":"),
                                ),
                            );
                            match crypto::import_public_key_raw(&subtle(), public_key).await {
                                Err(err) => {
                                    web_sys::console::log_2(
                                        &wasm_bindgen::JsValue::from_str("CRYPTO ERROR"),
                                        &err,
                                    );
                                    web_sys::window()
                                        .unwrap()
                                        .alert_with_message(
                                            &err.unchecked_into::<js_sys::Error>()
                                                .to_string()
                                                .as_string()
                                                .unwrap(),
                                        )
                                        .unwrap();
                                }
                                Ok(public_key) => {
                                    let hash_text = hash
                                        .iter()
                                        .map(|byte| format!("{:02X}", byte))
                                        .collect::<Vec<String>>()
                                        .join(":");

                                    if web_sys::window()
                                        .unwrap()
                                        .confirm_with_message(&format!(
                                            "Import public key with hash {}",
                                            hash_text
                                        ))
                                        .unwrap()
                                    {
                                        onpublickey.emit(public_key);
                                    }
                                }
                            }
                        });
                    } else if &binary[0..5] == b"SIGN:" {
                    }
                }
            }
        }
        false
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            let scanner = Html5QrcodeScanner::new(&format!("{}", self.reader_id), true);
            scanner.render(&self.scanned_closure, &self.error_closure);

            self.scanner = Some(scanner);
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class="reader" id={ format!("{}", self.reader_id) } ref=self.qr_ref.clone()>
            </div>
        }
    }
}
