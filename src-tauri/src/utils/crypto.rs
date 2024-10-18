use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, KeyInit}, aes::cipher::typenum, Aes256Gcm, Nonce
};
use anyhow::{Result, Context};
use base64::prelude::*;
use std::process::Command;
use sha2::{Sha256, Digest};
use rand::Rng;

#[cfg(target_os = "macos")]
fn get_device_id() -> Result<String> {
    let output = Command::new("ioreg")
        .arg("-rd1")
        .arg("-c")
        .arg("IOPlatformExpertDevice")
        .output()
        .context("Failed to execute ioreg command")?;
    let output_str = String::from_utf8(output.stdout).context("Failed to parse ioreg output")?;
    let uuid_line = output_str
        .lines()
        .find(|line| line.contains("IOPlatformUUID"))
        .ok_or_else(|| anyhow::anyhow!("Failed to find IOPlatformUUID"))?;
    let uuid = uuid_line
        .split('=')
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse IOPlatformUUID"))?
        .trim()
        .trim_matches('"');
    Ok(uuid.to_string())
}

#[cfg(target_os = "windows")]
fn get_device_id() -> Result<String> {
    use std::os::windows::process::CommandExt;
    let output = Command::new("wmic")
        .arg("csproduct")
        .arg("get")
        .arg("UUID")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .context("Failed to execute wmic command")?;
    let output_str = String::from_utf8(output.stdout).context("Failed to parse wmic output")?;
    let uuid_line = output_str
        .lines()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Failed to find UUID"))?;
    Ok(uuid_line.trim().to_string())
}

fn generate_key(device_id: &str) -> GenericArray<u8, typenum::U32> {
    let mut hasher = Sha256::default();
    hasher.update(device_id.as_bytes());
    let result = hasher.finalize();
    GenericArray::clone_from_slice(&result[0..32])
}

fn generate_iv() -> [u8; 12] {
    let mut rng = rand::thread_rng();
    let mut iv = [0u8; 12];
    rng.fill(&mut iv);
    iv
}

pub fn encrypt_string(plain_data: &str) -> Result<String> {
    let device_id = get_device_id().context("Failed to get device ID")?;
    let key = generate_key(&device_id);
    let iv = generate_iv();

    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(&iv);
    let ciphertext = cipher
        .encrypt(nonce, plain_data.as_bytes())
        .expect("encryption failure"); // 암호화 실패 시 panic 발생

    // IV를 Base64로 인코딩하여 암호화된 데이터와 함께 반환
    let iv_base64 = BASE64_STANDARD.encode(&iv);
    let ciphertext_base64 = BASE64_STANDARD.encode(&ciphertext);
    Ok(format!("{}:{}", iv_base64, ciphertext_base64))
}

pub fn decrypt_string(encoded_data: &str) -> Result<String> {
    let device_id = get_device_id().context("Failed to get device ID")?;
    let key = generate_key(&device_id);

    // IV와 암호화된 데이터를 분리
    let parts: Vec<&str> = encoded_data.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid encoded data format"));
    }
    let iv = BASE64_STANDARD.decode(parts[0])?;
    let ciphertext = BASE64_STANDARD.decode(parts[1])?;

    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(&iv);
    let decrypted_data = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .expect("decryption failure");
    Ok(String::from_utf8(decrypted_data).context("UTF-8 conversion failure")?)
}