use js_sys::{Array, ArrayBuffer, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKey, CryptoKeyPair, SubtleCrypto};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EcKeyGenParams {
    name: String,
    named_curve: String,
}

pub async fn generate_keypair(subtle: &SubtleCrypto) -> Result<CryptoKeyPair, JsValue> {
    let key_pair: CryptoKeyPair = JsFuture::from(
        subtle.generate_key_with_object(
            &JsValue::from_serde(&EcKeyGenParams {
                name: "ECDSA".to_owned(),
                named_curve: "P-521".to_owned(),
            })
            .expect("Failed serializing options")
            .into(),
            true,
            &Array::of2(&JsValue::from_str("sign"), &JsValue::from_str("verify")),
        )?,
    )
    .await?
    .unchecked_into();

    Ok(key_pair)
}

pub async fn export_public_key(
    subtle: &SubtleCrypto,
    key_pair: &CryptoKeyPair,
) -> Result<ArrayBuffer, JsValue> {
    let public_key: CryptoKey =
        Reflect::get(&key_pair, &JsValue::from_str("publicKey"))?.unchecked_into();
    let public_key_data = JsFuture::from(subtle.export_key("spki", &public_key)?).await?;

    Ok(public_key_data.unchecked_into())
}
