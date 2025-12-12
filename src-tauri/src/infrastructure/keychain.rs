use crate::models::{AwsCredentials, GitCredentials};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Keychain-specific errors
#[derive(Error, Debug)]
pub enum KeychainError {
    #[error("OS keychain access failed: {0}")]
    KeychainAccessFailed(String),
    
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Credential not found: {0}")]
    CredentialNotFound(String),
    
    #[error("File operation failed: {0}")]
    FileOperationFailed(String),
    
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
}

impl From<serde_json::Error> for KeychainError {
    fn from(err: serde_json::Error) -> Self {
        KeychainError::SerializationFailed(err.to_string())
    }
}

/// Secure credential storage service
pub struct KeychainService {
    keyring: keyring::Entry,
    fallback_path: PathBuf,
    encryption_key: Vec<u8>,
}

impl KeychainService {
    const SERVICE_NAME: &'static str = "deployotron";
    const AWS_KEY_NAME: &'static str = "aws_credentials";
    const GIT_KEY_NAME: &'static str = "git_credentials";
    const ENCRYPTION_KEY_NAME: &'static str = "encryption_key";
    
    /// Create a new keychain service instance
    pub fn new() -> Self {
        let keyring = keyring::Entry::new(Self::SERVICE_NAME, Self::ENCRYPTION_KEY_NAME)
            .expect("Failed to create keyring entry");
        
        let fallback_path = Self::get_fallback_path()
            .expect("Failed to determine fallback path");
        
        // Get or create encryption key
        let encryption_key = Self::get_or_create_encryption_key(&keyring);
        
        Self {
            keyring,
            fallback_path,
            encryption_key,
        }
    }
    
    /// Get the fallback storage directory path
    fn get_fallback_path() -> Result<PathBuf, KeychainError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| KeychainError::FileOperationFailed(
                "Could not determine data directory".to_string()
            ))?;
        
        let path = data_dir.join("deployotron").join("credentials");
        
        // Ensure directory exists
        fs::create_dir_all(&path)
            .map_err(|e| KeychainError::FileOperationFailed(e.to_string()))?;
        
        Ok(path)
    }
    
    /// Get or create the master encryption key
    fn get_or_create_encryption_key(keyring: &keyring::Entry) -> Vec<u8> {
        // Try to get existing key from OS keychain
        match keyring.get_password() {
            Ok(key_b64) => {
                // Decode existing key
                BASE64.decode(key_b64.as_bytes())
                    .unwrap_or_else(|_| Self::generate_new_key(keyring))
            }
            Err(_) => {
                // Generate and store new key
                Self::generate_new_key(keyring)
            }
        }
    }
    
    /// Generate a new encryption key and store in OS keychain
    fn generate_new_key(keyring: &keyring::Entry) -> Vec<u8> {
        let rng = SystemRandom::new();
        let mut key = vec![0u8; 32]; // AES-256 requires 32 bytes
        rng.fill(&mut key).expect("Failed to generate random key");
        
        // Store in OS keychain (best effort)
        let key_b64 = BASE64.encode(&key);
        let _ = keyring.set_password(&key_b64);
        
        key
    }
    
    /// Encrypt data using AES-256-GCM
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, KeychainError> {
        let rng = SystemRandom::new();
        
        // Generate a random nonce (12 bytes for GCM)
        let mut nonce_bytes = vec![0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|e| KeychainError::EncryptionFailed(e.to_string()))?;
        
        let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| KeychainError::EncryptionFailed("Invalid nonce".to_string()))?;
        
        // Create encryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| KeychainError::EncryptionFailed(e.to_string()))?;
        let key = LessSafeKey::new(unbound_key);
        
        // Encrypt the data
        let mut in_out = plaintext.to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|e| KeychainError::EncryptionFailed(e.to_string()))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes;
        result.extend_from_slice(&in_out);
        
        Ok(result)
    }
    
    /// Decrypt data using AES-256-GCM
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, KeychainError> {
        if ciphertext.len() < 12 {
            return Err(KeychainError::DecryptionFailed(
                "Ciphertext too short".to_string()
            ));
        }
        
        // Extract nonce and ciphertext
        let (nonce_bytes, encrypted_data) = ciphertext.split_at(12);
        
        let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|_| KeychainError::DecryptionFailed("Invalid nonce".to_string()))?;
        
        // Create decryption key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key)
            .map_err(|e| KeychainError::DecryptionFailed(e.to_string()))?;
        let key = LessSafeKey::new(unbound_key);
        
        // Decrypt the data
        let mut in_out = encrypted_data.to_vec();
        let plaintext = key.open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|e| KeychainError::DecryptionFailed(e.to_string()))?;
        
        Ok(plaintext.to_vec())
    }
    
    /// Store credentials in OS keychain with encrypted fallback
    fn store_credential(&self, key: &str, value: &str) -> Result<(), KeychainError> {
        let entry = keyring::Entry::new(Self::SERVICE_NAME, key)
            .map_err(|e| KeychainError::KeychainAccessFailed(e.to_string()))?;
        
        // Try OS keychain first
        match entry.set_password(value) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Fallback to encrypted file storage
                let encrypted = self.encrypt(value.as_bytes())?;
                let encoded = BASE64.encode(&encrypted);
                
                let file_path = self.fallback_path.join(format!("{}.enc", key));
                fs::write(&file_path, encoded)
                    .map_err(|e| KeychainError::FileOperationFailed(e.to_string()))?;
                
                Ok(())
            }
        }
    }
    
    /// Retrieve credentials from OS keychain or encrypted fallback
    fn get_credential(&self, key: &str) -> Result<String, KeychainError> {
        let entry = keyring::Entry::new(Self::SERVICE_NAME, key)
            .map_err(|e| KeychainError::KeychainAccessFailed(e.to_string()))?;
        
        // Try OS keychain first
        match entry.get_password() {
            Ok(value) => Ok(value),
            Err(_) => {
                // Fallback to encrypted file storage
                let file_path = self.fallback_path.join(format!("{}.enc", key));
                
                if !file_path.exists() {
                    return Err(KeychainError::CredentialNotFound(key.to_string()));
                }
                
                let encoded = fs::read_to_string(&file_path)
                    .map_err(|e| KeychainError::FileOperationFailed(e.to_string()))?;
                
                let encrypted = BASE64.decode(encoded.as_bytes())
                    .map_err(|e| KeychainError::DecryptionFailed(e.to_string()))?;
                
                let decrypted = self.decrypt(&encrypted)?;
                
                String::from_utf8(decrypted)
                    .map_err(|e| KeychainError::DecryptionFailed(e.to_string()))
            }
        }
    }
    
    /// Delete credentials from both OS keychain and fallback
    fn delete_credential(&self, key: &str) -> Result<(), KeychainError> {
        let entry = keyring::Entry::new(Self::SERVICE_NAME, key)
            .map_err(|e| KeychainError::KeychainAccessFailed(e.to_string()))?;
        
        // Delete from OS keychain (ignore errors)
        let _ = entry.delete_password();
        
        // Delete from fallback storage
        let file_path = self.fallback_path.join(format!("{}.enc", key));
        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| KeychainError::FileOperationFailed(e.to_string()))?;
        }
        
        Ok(())
    }
    
    // ===== AWS Credentials =====
    
    /// Store AWS credentials
    pub fn store_aws_credentials(&self, credentials: &AwsCredentials) -> Result<(), KeychainError> {
        let json = serde_json::to_string(credentials)?;
        self.store_credential(Self::AWS_KEY_NAME, &json)
    }
    
    /// Retrieve AWS credentials
    pub fn get_aws_credentials(&self) -> Result<AwsCredentials, KeychainError> {
        let json = self.get_credential(Self::AWS_KEY_NAME)?;
        let credentials = serde_json::from_str(&json)?;
        Ok(credentials)
    }
    
    /// Delete AWS credentials
    pub fn delete_aws_credentials(&self) -> Result<(), KeychainError> {
        self.delete_credential(Self::AWS_KEY_NAME)
    }
    
    // ===== Git Credentials =====
    
    /// Store Git credentials
    pub fn store_git_credentials(&self, credentials: &GitCredentials) -> Result<(), KeychainError> {
        let json = serde_json::to_string(credentials)?;
        self.store_credential(Self::GIT_KEY_NAME, &json)
    }
    
    /// Retrieve Git credentials
    pub fn get_git_credentials(&self) -> Result<GitCredentials, KeychainError> {
        let json = self.get_credential(Self::GIT_KEY_NAME)?;
        let credentials = serde_json::from_str(&json)?;
        Ok(credentials)
    }
    
    /// Delete Git credentials
    pub fn delete_git_credentials(&self) -> Result<(), KeychainError> {
        self.delete_credential(Self::GIT_KEY_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let service = KeychainService::new();
        let plaintext = b"sensitive data here";
        
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_aws_credentials_roundtrip() {
        let service = KeychainService::new();
        let credentials = AwsCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            region: "us-east-1".to_string(),
        };
        
        // Store and retrieve
        service.store_aws_credentials(&credentials).unwrap();
        let retrieved = service.get_aws_credentials().unwrap();
        
        assert_eq!(credentials.access_key_id, retrieved.access_key_id);
        assert_eq!(credentials.secret_access_key, retrieved.secret_access_key);
        assert_eq!(credentials.region, retrieved.region);
        
        // Cleanup
        service.delete_aws_credentials().unwrap();
    }

    #[test]
    fn test_git_credentials_roundtrip() {
        let service = KeychainService::new();
        let credentials = GitCredentials {
            username: "testuser".to_string(),
            token: "ghp_exampletoken123".to_string(),
            provider: "github".to_string(),
        };
        
        // Store and retrieve
        service.store_git_credentials(&credentials).unwrap();
        let retrieved = service.get_git_credentials().unwrap();
        
        assert_eq!(credentials.username, retrieved.username);
        assert_eq!(credentials.token, retrieved.token);
        assert_eq!(credentials.provider, retrieved.provider);
        
        // Cleanup
        service.delete_git_credentials().unwrap();
    }

    #[test]
    fn test_credential_not_found() {
        let service = KeychainService::new();
        
        // Ensure credentials don't exist
        let _ = service.delete_aws_credentials();
        
        let result = service.get_aws_credentials();
        assert!(matches!(result, Err(KeychainError::CredentialNotFound(_))));
    }
}
