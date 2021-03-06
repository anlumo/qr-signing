use crate::{crypto, qr_generator::encode_data, qr_reader::QrReader, subtle};
use js_sys::Reflect;
use std::io::Write;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CryptoKey, Url};
use yew::prelude::*;
use zip::{write::FileOptions, ZipWriter};

#[derive(Clone, PartialEq, Eq, Debug)]
enum AppKey {
    None,
    Public(CryptoKey),
    Pair(CryptoKey, CryptoKey),
}

impl AppKey {
    fn is_pair(&self) -> bool {
        matches!(self, Self::Pair(_, _))
    }
    fn public_key(&self) -> Option<CryptoKey> {
        match self {
            Self::None => None,
            Self::Public(public_key) => Some(public_key.clone()),
            Self::Pair(public_key, _) => Some(public_key.clone()),
        }
    }
}

pub struct Main {
    link: ComponentLink<Self>,
    key: AppKey,
    qr_key: NodeRef,
    open_file: NodeRef,
    open_text: NodeRef,
    public_hash: Option<String>,
}

pub enum Msg {
    ImportKeyPair,
    ExportKeyPair,
    GenerateKeyPair,
    KeyPairSelected,
    SetKeyPair(CryptoKey, CryptoKey),
    SetPublicKey(CryptoKey),
    SetPublicHash([u8; 32]),
    Sign,
    TextFileSelected,
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
            open_text: NodeRef::default(),
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
                self.key = AppKey::Pair(public_key, private_key);
                self.update_hash_and_qr();
            }
            Msg::SetPublicKey(public_key) => {
                self.key = AppKey::Public(public_key);
                self.update_hash_and_qr();
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
            Msg::Sign => {
                let open_text = self.open_text.cast::<web_sys::HtmlInputElement>().unwrap();
                open_text.click();
            }
            Msg::TextFileSelected => {
                let element = self.open_text.cast::<web_sys::HtmlInputElement>().unwrap();

                if let AppKey::Pair(_, private_key) = &self.key {
                    if let Some(files) = element.files() {
                        if let Some(file) = files.get(0) {
                            let private_key = private_key.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                let data =
                                    wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                                        .await
                                        .unwrap();
                                let data = js_sys::Uint8Array::new(&data).to_vec();
                                match String::from_utf8(data) {
                                    Err(_) => {
                                        web_sys::window()
                                            .unwrap()
                                            .alert_with_message(
                                                "The provided file is not UTF-8 encoded text.",
                                            )
                                            .unwrap();
                                    }
                                    Ok(text) => {
                                        let subtle = subtle();
                                        let lines: Vec<_> = text.lines().collect();
                                        match futures::future::join_all(
                                            lines.iter().map(|line| {
                                                crypto::sign(&subtle, &private_key, line)
                                            }),
                                        )
                                        .await
                                        .into_iter()
                                        .collect::<Result<Vec<_>, _>>()
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
                                            Ok(signed) => {
                                                let mut data: Vec<u8> = Vec::new();
                                                {
                                                    let mut cursor =
                                                        std::io::Cursor::new(&mut data);
                                                    let mut zip = ZipWriter::new(&mut cursor);
                                                    for (idx, entry) in
                                                        signed.into_iter().enumerate()
                                                    {
                                                        let bytes = js_sys::Uint8Array::new(&entry)
                                                            .to_vec();
                                                        let mut signed_data = b"SIGN:".to_vec();
                                                        signed_data.extend_from_slice(&bytes);
                                                        signed_data.extend_from_slice(
                                                            lines[idx].as_bytes(),
                                                        );

                                                        let svg =
                                                            encode_data(&signed_data).unwrap();
                                                        zip.start_file(
                                                            format!("signed_{}.svg", idx + 1),
                                                            FileOptions::default(),
                                                        )
                                                        .unwrap();
                                                        zip.write_all(svg.as_bytes()).unwrap();
                                                    }
                                                    zip.finish().unwrap();
                                                }
                                                let buffer =
                                                    js_sys::Uint8Array::from(data.as_slice());
                                                let blob = web_sys::Blob::new_with_blob_sequence(
                                                    &js_sys::Array::of1(&buffer),
                                                )
                                                .unwrap();
                                                let blob_url =
                                                    Url::create_object_url_with_blob(&blob)
                                                        .unwrap();
                                                let a: web_sys::HtmlAnchorElement =
                                                    web_sys::window()
                                                        .unwrap()
                                                        .document()
                                                        .unwrap()
                                                        .create_element("A")
                                                        .unwrap()
                                                        .unchecked_into();
                                                a.set_href(&blob_url);
                                                a.set_download("signed.zip");
                                                a.click();
                                                Url::revoke_object_url(&blob_url).unwrap();
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
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
                    <button onclick=self.link.callback(|_| Msg::Sign) class="mdi-set mdi-feather" title="Batch sign text" disabled={ !self.key.is_pair() }></button>
                    <div class="key_qr" ref=self.qr_key.clone()></div>
                </header>
                <div class="hash">
                    { "Public Key Fingerprint:" }
                    <br />
                    {
                        self.public_hash.as_deref().unwrap_or("<no public key loaded>")
                    }
                </div>
                <QrReader onpublickey=self.link.callback(|msg: CryptoKey| Msg::SetPublicKey(msg)) public_key={ self.key.public_key() }/>
                <input class="hidden" type="file" accept="application/json" ref=self.open_file.clone() onchange=self.link.callback(|_| Msg::KeyPairSelected) multiple=false />
                <input class="hidden" type="file" accept="text/plain" ref=self.open_text.clone() onchange=self.link.callback(|_| Msg::TextFileSelected) multiple=false />
            </div>
        }
    }
}

impl Main {
    fn update_hash_and_qr(&self) {
        let public_key = match &self.key {
            AppKey::Pair(public_key, _) => public_key.clone(),
            AppKey::Public(public_key) => public_key.clone(),
            AppKey::None => return,
        };
        let qr_div = self.qr_key.cast::<web_sys::Element>().unwrap();
        let link = self.link.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let public_key = crypto::export_key_raw(&subtle(), &public_key)
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
}
