use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, AeadCore, Nonce};

/// AES-256-GCM encryption service for label images at rest.
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create from a base64-encoded 32-byte key.
    pub fn new(key_base64: &str) -> Result<Self, EncryptionError> {
        use base64::Engine;
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(key_base64)
            .map_err(|_| EncryptionError::InvalidKey)?;

        if key_bytes.len() != 32 {
            return Err(EncryptionError::InvalidKey);
        }

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|_| EncryptionError::InvalidKey)?;

        Ok(Self { cipher })
    }

    /// Encrypt data, returning nonce (12 bytes) prepended to ciphertext.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| EncryptionError::EncryptFailed)?;

        let mut output = nonce.to_vec();
        output.extend(ciphertext);
        Ok(output)
    }

    /// Decrypt data where the first 12 bytes are the nonce.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if data.len() < 12 {
            return Err(EncryptionError::DecryptFailed);
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| EncryptionError::DecryptFailed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Invalid encryption key (must be 32 bytes, base64-encoded)")]
    InvalidKey,

    #[error("Encryption failed")]
    EncryptFailed,

    #[error("Decryption failed")]
    DecryptFailed,
}
