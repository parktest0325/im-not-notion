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
    // MachineGuid via reg.exe: wmic is removed from Windows 11 24H2+
    let output = Command::new("reg")
        .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .context("Failed to execute reg command")?;
    let output_str = String::from_utf8(output.stdout).context("Failed to parse reg output")?;
    let guid = output_str
        .lines()
        .filter(|line| line.trim_start().starts_with("MachineGuid"))
        .find_map(|line| line.split_whitespace().last())
        .ok_or_else(|| anyhow::anyhow!("Failed to find MachineGuid"))?;
    Ok(guid.to_string())
}

/// 이전 버전이 키 유도에 쓰던 wmic 기반 device ID (복호화 폴백 전용)
#[cfg(target_os = "windows")]
fn get_legacy_device_id() -> Result<String> {
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

#[cfg(not(target_os = "windows"))]
fn get_legacy_device_id() -> Result<String> {
    Err(anyhow::anyhow!("No legacy device ID on this platform"))
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
        .map_err(|e| anyhow::anyhow!("Encryption failure: {}", e))?;

    let iv_base64 = BASE64_STANDARD.encode(iv);
    let ciphertext_base64 = BASE64_STANDARD.encode(ciphertext);
    Ok(format!("{}:{}", iv_base64, ciphertext_base64))
}

pub fn decrypt_string(encoded_data: &str) -> Result<String> {
    // IV와 암호화된 데이터를 분리
    let parts: Vec<&str> = encoded_data.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid encoded data format"));
    }
    let iv = BASE64_STANDARD.decode(parts[0])?;
    if iv.len() != 12 {
        return Err(anyhow::anyhow!("Invalid IV length: {}", iv.len()));
    }
    let ciphertext = BASE64_STANDARD.decode(parts[1])?;

    let device_id = get_device_id().context("Failed to get device ID")?;
    match decrypt_with_key(&generate_key(&device_id), &iv, &ciphertext) {
        Ok(plain) => Ok(plain),
        Err(err) => {
            // 이전 버전(wmic 키)으로 암호화된 데이터 마이그레이션 폴백
            if let Ok(legacy_id) = get_legacy_device_id() {
                if let Ok(plain) = decrypt_with_key(&generate_key(&legacy_id), &iv, &ciphertext) {
                    return Ok(plain);
                }
            }
            Err(err)
        }
    }
}

fn decrypt_with_key(
    key: &GenericArray<u8, typenum::U32>,
    iv: &[u8],
    ciphertext: &[u8],
) -> Result<String> {
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(iv);
    let decrypted_data = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failure: {}", e))?;
    String::from_utf8(decrypted_data).context("UTF-8 conversion failure")
}