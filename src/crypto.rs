use js_sys::{Array, ArrayBuffer, Object, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKey, SubtleCrypto};

const CURVE: &str = "P-256";
pub const SIGNATURE_SIZE: usize = 64;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EcKeyGenParams {
    name: String,
    named_curve: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EcdsaParams {
    name: String,
    hash: String,
}

pub async fn generate_keypair(subtle: &SubtleCrypto) -> Result<(CryptoKey, CryptoKey), JsValue> {
    let key_pair = JsFuture::from(
        subtle.generate_key_with_object(
            &JsValue::from_serde(&EcKeyGenParams {
                name: "ECDSA".to_owned(),
                named_curve: CURVE.to_owned(),
            })
            .expect("Failed serializing options")
            .into(),
            true,
            &Array::of2(&JsValue::from_str("sign"), &JsValue::from_str("verify")),
        )?,
    )
    .await?;
    let public_key: CryptoKey =
        Reflect::get(&key_pair, &JsValue::from_str("publicKey"))?.unchecked_into();
    let private_key: CryptoKey =
        Reflect::get(&key_pair, &JsValue::from_str("privateKey"))?.unchecked_into();

    Ok((public_key, private_key))
}

pub async fn export_key(subtle: &SubtleCrypto, key: &CryptoKey) -> Result<Object, JsValue> {
    let key_data = JsFuture::from(subtle.export_key("jwk", key)?).await?;
    Ok(key_data.unchecked_into())
}

pub async fn export_key_raw(
    subtle: &SubtleCrypto,
    key: &CryptoKey,
) -> Result<ArrayBuffer, JsValue> {
    let key_data = JsFuture::from(subtle.export_key("raw", key)?).await?;
    Ok(key_data.unchecked_into())
}

pub async fn sign(
    subtle: &SubtleCrypto,
    private_key: &CryptoKey,
    text: &str,
) -> Result<ArrayBuffer, JsValue> {
    let mut text = text.to_owned();

    let signed_bytes = JsFuture::from(
        subtle.sign_with_object_and_u8_array(
            &JsValue::from_serde(&EcdsaParams {
                name: "ECDSA".to_owned(),
                hash: "SHA-256".to_owned(),
            })
            .expect("Failed serializing options")
            .into(),
            &private_key,
            unsafe { text.as_bytes_mut() },
        )?,
    )
    .await?;

    Ok(signed_bytes.unchecked_into())
}

pub async fn import_public_key(
    subtle: &SubtleCrypto,
    key: &js_sys::Object,
) -> Result<CryptoKey, JsValue> {
    wasm_bindgen_futures::JsFuture::from(
        subtle.import_key_with_object(
            "jwk",
            key,
            JsValue::from_serde(&EcKeyGenParams {
                name: "ECDSA".to_owned(),
                named_curve: CURVE.to_owned(),
            })
            .unwrap()
            .unchecked_ref(),
            true,
            &Array::of1(&JsValue::from_str("verify")),
        )?,
    )
    .await
    .map(|key| key.unchecked_into())
}

pub async fn import_public_key_raw(
    subtle: &SubtleCrypto,
    key: &[u8],
) -> Result<CryptoKey, JsValue> {
    let u8array = js_sys::Uint8Array::from(key);
    wasm_bindgen_futures::JsFuture::from(
        subtle.import_key_with_object(
            "raw",
            &u8array,
            JsValue::from_serde(&EcKeyGenParams {
                name: "ECDSA".to_owned(),
                named_curve: CURVE.to_owned(),
            })
            .unwrap()
            .unchecked_ref(),
            true,
            &Array::of1(&JsValue::from_str("verify")),
        )?,
    )
    .await
    .map(|key| key.unchecked_into())
}

pub async fn import_private_key(
    subtle: &SubtleCrypto,
    key: &js_sys::Object,
) -> Result<CryptoKey, JsValue> {
    wasm_bindgen_futures::JsFuture::from(
        subtle.import_key_with_object(
            "jwk",
            key,
            JsValue::from_serde(&EcdsaParams {
                name: "ECDSA".to_owned(),
                hash: "SHA-256".to_owned(),
            })
            .unwrap()
            .unchecked_ref(),
            true,
            &Array::of1(&JsValue::from_str("sign")),
        )?,
    )
    .await
    .map(|key| key.unchecked_into())
}

pub async fn verify(
    subtle: &SubtleCrypto,
    public_key: &CryptoKey,
    signature: &[u8],
    data: &[u8],
) -> Result<bool, JsValue> {
    let mut signature = signature.to_vec();
    let mut data = data.to_vec();
    wasm_bindgen_futures::JsFuture::from(
        subtle.verify_with_object_and_u8_array_and_u8_array(
            JsValue::from_serde(&EcdsaParams {
                name: "ECDSA".to_owned(),
                hash: "SHA-256".to_owned(),
            })
            .unwrap()
            .unchecked_ref(),
            public_key,
            &mut signature,
            &mut data,
        )?,
    )
    .await
    .map(|flag| flag.is_truthy())
}
