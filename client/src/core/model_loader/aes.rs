// client/src/crypto/aes.rs

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use argon2::{self, Config, ThreadMode, Variant, Version};
use hex;
use std::{
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

/// Hardware-accelerated AES implementation
#[derive(Clone)]
pub struct Aes256GcmProvider {
    use_hardware: bool,
}

impl Aes256GcmProvider {
    /// Initialize AES provider with hardware acceleration detection
    pub fn new() -> Self {
        Self {
            use_hardware: is_aesni_supported(),
        }
    }

    /// Encrypt data with AES-256-GCM and Argon2 key derivation
    pub fn encrypt(&self, plaintext: &[u8], password: &str, aad: &[u8]) -> Result<Vec<u8>, AesError> {
        // Key derivation with Argon2
        let salt = Argon2Salt::generate();
        let key = self.derive_key(password, &salt)?;

        // Initialize cipher
        let cipher = self.init_cipher(&key)?;

        // Generate random 96-bit nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Hardware-accelerated encryption
        let ciphertext = if self.use_hardware {
            unsafe { self.encrypt_ni(cipher, nonce, plaintext, aad)? }
        } else {
            cipher.encrypt(&nonce, plaintext)?
        };

        // Build output format: [Argon2 salt (16B)] [nonce (12B)] [ciphertext] [tag (16B)]
        let mut output = Vec::with_capacity(16 + 12 + ciphertext.len() + 16);
        output.extend_from_slice(&salt);
        output.extend_from_slice(nonce.as_slice());
        output.extend(ciphertext);
        Ok(output)
    }

    /// Decrypt data with authentication checks
    pub fn decrypt(&self, ciphertext: &[u8], password: &str, aad: &[u8]) -> Result<Vec<u8>, AesError> {
        // Parse ciphertext components
        if ciphertext.len() < 16 + 12 + 16 {
            return Err(AesError::InvalidLength);
        }

        let salt = &ciphertext[0..16];
        let nonce = Nonce::from_slice(&ciphertext[16..28]);
        let encrypted_data = &ciphertext[28..ciphertext.len() - 16];
        let tag = &ciphertext[ciphertext.len() - 16..];

        // Reconstruct full ciphertext with tag
        let mut full_ciphertext = Vec::from(encrypted_data);
        full_ciphertext.extend_from_slice(tag);

        // Derive key
        let key = self.derive_key(password, salt)?;

        // Initialize cipher
        let cipher = self.init_cipher(&key)?;

        // Hardware-accelerated decryption
        let plaintext = if self.use_hardware {
            unsafe { self.decrypt_ni(cipher, nonce, &full_ciphertext, aad)? }
        } else {
            cipher.decrypt(nonce, full_ciphertext.as_slice())?
        };

        Ok(plaintext)
    }

    /// Key derivation with Argon2id
    fn derive_key(&self, password: &str, salt: &[u8]) -> Result<[u8; 32], AesError> {
        let config = Config {
            variant: Variant::Argon2id,
            version: Version::Version13,
            mem_cost: 19456,  // 19 MB
            time_cost: 3,
            lanes: 4,
            thread_mode: ThreadMode::Parallel,
            secret: &[],
            ad: &[],
            hash_length: 32,
        };

        let key = argon2::hash_encoded(password.as_bytes(), salt, &config)
            .map_err(|_| AesError::KeyDerivationFailed)?;
        
        hex::decode(key)
            .map_err(|_| AesError::KeyDecodingFailed)?
            .try_into()
            .map_err(|_| AesError::InvalidKeyLength)
    }

    /// Initialize cipher with key
    fn init_cipher(&self, key: &[u8; 32]) -> Result<Aes256Gcm, AesError> {
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        Ok(Aes256Gcm::new(key))
    }

    /// Hardware-accelerated encryption using AES-NI
    unsafe fn encrypt_ni(
        &self,
        cipher: Aes256Gcm,
        nonce: Nonce,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, AesError> {
        let mut out_len = MaybeUninit::uninit();
        let mut out = vec![0u8; plaintext.len() + 16];
        
        let res = aesni_gcm_encrypt(
            plaintext.as_ptr(),
            plaintext.len(),
            aad.as_ptr(),
            aad.len(),
            cipher.key().as_ptr(),
            nonce.as_slice().as_ptr(),
            out.as_mut_ptr(),
            out_len.as_mut_ptr(),
        );

        if res != 0 {
            return Err(AesError::EncryptionFailed);
        }

        let out_len = out_len.assume_init();
        out.truncate(out_len);
        Ok(out)
    }

    /// Hardware-accelerated decryption using AES-NI
    unsafe fn decrypt_ni(
        &self,
        cipher: Aes256Gcm,
        nonce: &Nonce,
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, AesError> {
        let mut out_len = MaybeUninit::uninit();
        let mut out = vec![0u8; ciphertext.len()];

        let res = aesni_gcm_decrypt(
            ciphertext.as_ptr(),
            ciphertext.len(),
            aad.as_ptr(),
            aad.len(),
            cipher.key().as_ptr(),
            nonce.as_slice().as_ptr(),
            out.as_mut_ptr(),
            out_len.as_mut_ptr(),
        );

        if res != 0 {
            return Err(AesError::DecryptionFailed);
        }

        let out_len = out_len.assume_init();
        out.truncate(out_len);
        Ok(out)
    }
}

/// Check AES-NI support at runtime
fn is_aesni_supported() -> bool {
    static HAS_AESNI: AtomicBool = AtomicBool::new(false);
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        let result = unsafe {
            let mut ecx: u32 = 0;
            std::arch::asm!(
                "cpuid",
                inout("eax") 1 => _,
                inout("ecx") ecx => ecx,
                lateout("edx") _,
                lateout("ebx") _,
            );
            (ecx & (1 << 25)) != 0
        };
        HAS_AESNI.store(result, Ordering::Relaxed);
    });

    HAS_AESNI.load(Ordering::Relaxed)
}

/// Custom error types
#[derive(Debug, thiserror::Error)]
pub enum AesError {
    #[error("Encryption operation failed")]
    EncryptionFailed,
    #[error("Decryption operation failed")]
    DecryptionFailed,
    #[error("Invalid ciphertext length")]
    InvalidLength,
    #[error("Key derivation failed")]
    KeyDerivationFailed,
    #[error("Key decoding failed")]
    KeyDecodingFailed,
    #[error("Invalid key length")]
    InvalidKeyLength,
}

// FFI bindings for AES-NI acceleration (Linux/macOS x86_64)
#[link(name = "crypto", kind = "static")]
extern "C" {
    fn aesni_gcm_encrypt(
        plaintext: *const u8,
        plaintext_len: usize,
        aad: *const u8,
        aad_len: usize,
        key: *const u8,
        nonce: *const u8,
        out: *mut u8,
        out_len: *mut usize,
    ) -> i32;

    fn aesni_gcm_decrypt(
        ciphertext: *const u8,
        ciphertext_len: usize,
        aad: *const u8,
        aad_len: usize,
        key: *const u8,
        nonce: *const u8,
        out: *mut u8,
        out_len: *mut usize,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_encryption_cycle() {
        let aes = Aes256GcmProvider::new();
        let plaintext = b"SCORIA AI confidential model parameters";
        let password = "strong_password_!@#";

        // Encrypt
        let ciphertext = aes.encrypt(plaintext, password, b"auth_data")
            .expect("Encryption failed");
        
        // Decrypt
        let decrypted = aes.decrypt(&ciphertext, password, b"auth_data")
            .expect("Decryption failed");

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_tamper_protection() {
        let aes = Aes256GcmProvider::new();
        let plaintext = b"Critical security data";
        let mut ciphertext = aes.encrypt(plaintext, "password", b"aad")
            .expect("Encryption failed");

        // Tamper with ciphertext
        ciphertext[30] ^= 0x01;

        let result = aes.decrypt(&ciphertext, "password", b"aad");
        assert!(matches!(result, Err(AesError::DecryptionFailed)));
    }
}
