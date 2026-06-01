use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::{RngCore, rngs::OsRng};

use crate::error::{AppError, AppResult};

const NONCE_LEN: usize = 12;
const ENCRYPTED_PREFIX: &str = "enc:v1:";

#[derive(Clone)]
pub struct SecretBox {
    cipher: Aes256Gcm,
}

impl SecretBox {
    pub fn from_base64_key(value: &str) -> AppResult<Self> {
        let key = STANDARD
            .decode(value)
            .map_err(|err| AppError::Internal(format!("ENCRYPTION_KEY 不是有效 base64：{err}")))?;
        if key.len() != 32 {
            return Err(AppError::Internal(
                "ENCRYPTION_KEY 解码后必须是 32 字节".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> AppResult<String> {
        let mut nonce = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
            .map_err(|_| AppError::Internal("敏感配置加密失败".to_string()))?;

        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);
        Ok(format!("{ENCRYPTED_PREFIX}{}", STANDARD.encode(payload)))
    }

    pub fn decrypt(&self, payload: &str) -> AppResult<String> {
        let payload = payload.strip_prefix(ENCRYPTED_PREFIX).unwrap_or(payload);
        let payload = STANDARD
            .decode(payload)
            .map_err(|err| AppError::Internal(format!("敏感配置不是有效 base64：{err}")))?;
        if payload.len() <= NONCE_LEN {
            return Err(AppError::Internal("敏感配置密文长度无效".to_string()));
        }

        let (nonce, ciphertext) = payload.split_at(NONCE_LEN);
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| AppError::Internal("敏感配置解密失败".to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|err| AppError::Internal(format!("敏感配置不是有效 UTF-8：{err}")))
    }

    pub fn encrypt_if_plaintext(&self, value: &str) -> AppResult<String> {
        if value.starts_with(ENCRYPTED_PREFIX) {
            self.decrypt(value)?;
            return Ok(value.to_string());
        }

        if self.decrypt(value).is_ok() {
            return Ok(value.to_string());
        }

        self.encrypt(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret_box() -> SecretBox {
        SecretBox::from_base64_key("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap()
    }

    #[test]
    fn encrypt_uses_prefix_and_decrypts_to_plaintext() {
        let secrets = secret_box();
        let encrypted = secrets.encrypt("test_text").unwrap();

        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_eq!(secrets.decrypt(&encrypted).unwrap(), "test_text");
    }

    #[test]
    fn encrypt_if_plaintext_keeps_existing_ciphertext() {
        let secrets = secret_box();
        let encrypted = secrets.encrypt("test_text").unwrap();

        assert_eq!(secrets.encrypt_if_plaintext(&encrypted).unwrap(), encrypted);
        assert!(
            secrets
                .encrypt_if_plaintext("test_text")
                .unwrap()
                .starts_with(ENCRYPTED_PREFIX)
        );
    }
}
