use crate::{crypto, qr_generator::encode_data};
use js_sys::Reflect;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::CryptoKey;
use yew::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug)]
enum AppKey {
    None,
    Public(CryptoKey),
    Pair(CryptoKey, CryptoKey),
}

impl AppKey {
    fn is_none(&self) -> bool {
        self == &Self::None
    }
    fn is_pair(&self) -> bool {
        if let Self::Pair(_, _) = self {
            true
        } else {
            false
        }
    }
}

pub struct Main {
    link: ComponentLink<Self>,
    key: AppKey,
    qr_key: NodeRef,
    open_file: NodeRef,
    public_hash: Option<String>,
}

pub enum Msg {
    ImportKeyPair,
    ExportKeyPair,
    GenerateKeyPair,
    KeyPairSelected,
    SetKeyPair(CryptoKey, CryptoKey),
    SetPublicHash([u8; 32]),
}

fn subtle() -> web_sys::SubtleCrypto {
    let window = web_sys::window().unwrap();
    window
        .crypto()
        .expect("No WebCrypto support found!")
        .subtle()
}

impl Component for Main {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            key: AppKey::None,
            qr_key: NodeRef::default(),
            open_file: NodeRef::default(),
            public_hash: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ImportKeyPair => {
                let open_file = self.open_file.cast::<web_sys::HtmlInputElement>().unwrap();
                open_file.click();
            }
            Msg::ExportKeyPair => {
                if let AppKey::Pair(public_key, private_key) = &self.key {
                    let public_key = public_key.clone();
                    let private_key = private_key.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let subtle = subtle();
                        let (public_key, private_key) = futures::join!(
                            crypto::export_key(&subtle, &public_key),
                            crypto::export_key(&subtle, &private_key)
                        );
                        let public_key = public_key.unwrap();
                        let private_key = private_key.unwrap();
                        let key_pair_json = js_sys::Object::new();
                        Reflect::set(
                            &key_pair_json,
                            &JsValue::from_str("public"),
                            public_key.unchecked_ref(),
                        )
                        .unwrap();
                        Reflect::set(
                            &key_pair_json,
                            &JsValue::from_str("private"),
                            private_key.unchecked_ref(),
                        )
                        .unwrap();
                        Reflect::set(
                            &key_pair_json,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("qr_key_pair"),
                        )
                        .unwrap();

                        let json = js_sys::JSON::stringify(key_pair_json.unchecked_ref())
                            .unwrap()
                            .as_string()
                            .unwrap();
                        let mut data_url = "data:application/json,".to_owned();
                        data_url.push_str(&json);

                        let a: web_sys::HtmlAnchorElement = web_sys::window()
                            .unwrap()
                            .document()
                            .unwrap()
                            .create_element("A")
                            .unwrap()
                            .unchecked_into();
                        a.set_href(&data_url);
                        a.set_download("qr_key.json");
                        a.click();
                    });
                }
            }
            Msg::GenerateKeyPair => {
                let link = self.link.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let subtle = subtle();

                    match crypto::generate_keypair(&subtle).await {
                        Ok((public_key, private_key)) => {
                            link.send_message(Msg::SetKeyPair(public_key, private_key))
                        }
                        Err(err) => {
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
                    }
                });
            }
            Msg::SetKeyPair(public_key, private_key) => {
                let local_public_key = public_key.clone();
                self.key = AppKey::Pair(public_key, private_key);
                let qr_div = self.qr_key.cast::<web_sys::Element>().unwrap();
                let link = self.link.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let public_key = crypto::export_key_raw(&subtle(), &local_public_key)
                        .await
                        .unwrap();
                    let public_key_u8 = js_sys::Uint8Array::new(public_key.unchecked_ref());
                    let public_key = public_key_u8.to_vec();
                    let mut data = Vec::new();
                    data.extend_from_slice(b"PUB:");
                    data.extend_from_slice(&public_key);

                    link.send_message(Msg::SetPublicHash(hmac_sha256::Hash::hash(&public_key)));

                    let qr_svg = encode_data(&data).unwrap();
                    qr_div.set_inner_html(&qr_svg);
                });
            }
            Msg::KeyPairSelected => {
                let element = self.open_file.cast::<web_sys::HtmlInputElement>().unwrap();

                if let Some(files) = element.files() {
                    if let Some(file) = files.get(0) {
                        let link = self.link.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let data = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                                .await
                                .unwrap();
                            let text_decoder =
                                web_sys::TextDecoder::new_with_label("utf-8").unwrap();
                            match text_decoder.decode_with_buffer_source(data.unchecked_ref()) {
                                Err(err) => {
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
                                Ok(text) => match js_sys::JSON::parse(&text) {
                                    Err(err) => {
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
                                    Ok(json) => {
                                        if Some("qr_key_pair".to_owned())
                                            == Reflect::get(&json, &JsValue::from_str("type"))
                                                .unwrap()
                                                .as_string()
                                        {
                                            let public =
                                                Reflect::get(&json, &JsValue::from_str("public"))
                                                    .unwrap();
                                            let private =
                                                Reflect::get(&json, &JsValue::from_str("private"))
                                                    .unwrap();
                                            if public.is_falsy() || private.is_falsy() {
                                                web_sys::window()
                                                    .unwrap()
                                                    .alert_with_message(
                                                        "This file is not a key pair!",
                                                    )
                                                    .unwrap();
                                            }
                                            let subtle = subtle();
                                            match crypto::import_public_key(
                                                &subtle,
                                                public.unchecked_ref(),
                                            )
                                            .await
                                            {
                                                Err(err) => {
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
                                                Ok(public_key) => match crypto::import_private_key(
                                                    &subtle,
                                                    private.unchecked_ref(),
                                                )
                                                .await
                                                {
                                                    Err(err) => {
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
                                                    Ok(private_key) => {
                                                        link.send_message(Msg::SetKeyPair(
                                                            public_key,
                                                            private_key,
                                                        ));
                                                    }
                                                },
                                            }
                                        } else {
                                            web_sys::window()
                                                .unwrap()
                                                .alert_with_message("This file is not a key pair!")
                                                .unwrap();
                                        }
                                    }
                                },
                            }
                        });
                    }
                }
            }
            Msg::SetPublicHash(hash) => {
                let text = hash
                    .iter()
                    .map(|byte| format!("{:02X}", byte))
                    .collect::<Vec<String>>()
                    .join(":");
                self.public_hash = Some(text);
            }
        }
        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <header>
                    <button onclick=self.link.callback(|_| Msg::GenerateKeyPair) class="mdi-set mdi-briefcase-outline" title="Generate Key Pair"></button>
                    <button onclick=self.link.callback(|_| Msg::ImportKeyPair) class="mdi-set mdi-briefcase-upload" title="Import Key Pair"></button>
                    <button onclick=self.link.callback(|_| Msg::ExportKeyPair) class="mdi-set mdi-briefcase-download" title="Export Key Pair" disabled={ !self.key.is_pair() }></button>
                    <div class="key_qr" ref=self.qr_key.clone()></div>
                </header>
                <div class="hash">
                    { "Public Key Fingerprint:" }
                    <br />
                    {
                        self.public_hash.as_deref().unwrap_or("<no public key loaded>")
                    }
                </div>
                <div id="reader"></div>
                <input type="file" accept="application/json" ref=self.open_file.clone() onchange=self.link.callback(|_| Msg::KeyPairSelected) multiple=false />
            </div>
        }
    }
}
