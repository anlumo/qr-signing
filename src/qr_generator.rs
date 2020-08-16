use qrcodegen::{QrCode, QrCodeEcc};

pub fn encode_data(data: &[u8]) -> Result<String, ()> {
    QrCode::encode_binary(data, QrCodeEcc::Low)
        .map_err(|_| ())
        .map(|qr| qr.to_svg_string(5))
}
