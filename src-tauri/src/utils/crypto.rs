use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, KeyInit},
    Aes256Gcm,
};
use anyhow::Result;
use base64::prelude::*;

const KEY: [u8; 32] = *b"dongkimimnotnotiondongkimimnotno"; // 32바이트
const IV: [u8; 12] = *b"imnotkimdong"; // 12바이트

// TODO: Panic to Error
pub fn encrypt_string(plain_data: &str) -> Result<String> {
    let cipher = Aes256Gcm::new(&KEY.into());
    let nonce = GenericArray::from_slice(&IV);
    let ciphertext = cipher
        .encrypt(nonce, plain_data.as_bytes())
        .expect("encryption failure"); // 암호화 실패 시 panic 발생
    Ok(BASE64_STANDARD.encode(&ciphertext)) // 암호화된 데이터를 Base64로 인코딩하여 반환
}

pub fn decrypt_string(encoded_data: &str) -> Result<String> {
    let cipher = Aes256Gcm::new(&KEY.into());
    let nonce = GenericArray::from_slice(&IV);
    let decoded_data = BASE64_STANDARD.decode(encoded_data)?;
    let decrypted_data = cipher
        .decrypt(nonce, decoded_data.as_ref())
        .expect("decryption failure");
    Ok(String::from_utf8(decrypted_data)?)
}
