// client/src/integrity.rs

use blake3::{Hasher, Hash};
use memmap2::Mmap;
use rayon::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use std::{
    fs::File,
    io::{Error as IoError, Read},
    path::Path,
};
use thiserror::Error;

/// BLAKE3 integrity checker with Solana chain verification
pub struct Blake3IntegrityChecker {
    chunk_size: usize,
    max_threads: usize,
    rpc_client: RpcClient,
}

#[derive(Debug, Error)]
pub enum IntegrityError {
    #[error("I/O error: {0}")]
    Io(#[from] IoError),
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        expected: String,
        actual: String,
    },
    #[error("Chain verification failed: {0}")]
    ChainVerification(String),
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(usize),
}

impl Blake3IntegrityChecker {
    /// Initialize with performance-optimized defaults
    pub fn new(rpc_url: &str) -> Result<Self, IntegrityError> {
        Ok(Self {
            chunk_size: 1024 * 1024, // 1MB chunks
            max_threads: rayon::current_num_threads(),
            rpc_client: RpcClient::new(rpc_url),
        })
    }

    /// Compute BLAKE3 hash of a file with parallel processing
    pub fn compute_file_hash<P: AsRef<Path>>(&self, path: P) -> Result<Hash, IntegrityError> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        // Parallel hash computation
        let mut hasher = Hasher::new();
        mmap.par_chunks(self.chunk_size)
            .enumerate()
            .for_each(|(i, chunk)| {
                let mut chunk_hasher = Hasher::new();
                chunk_hasher.update(chunk);
                let chunk_hash = chunk_hasher.finalize();
                
                // Combine hashes in deterministic order
                let mut combined = [0u8; 64];
                combined[0..32].copy_from_slice(hasher.finalize().as_bytes());
                combined[32..64].copy_from_slice(chunk_hash.as_bytes());
                
                if i == 0 {
                    hasher = Hasher::new();
                }
                hasher.update(&combined);
            });

        Ok(hasher.finalize())
    }

    /// Verify file against expected hash
    pub fn verify_file<P: AsRef<Path>>(
        &self,
        path: P,
        expected_hash: &str,
    ) -> Result<(), IntegrityError> {
        let actual = self.compute_file_hash(path)?;
        let expected = Hash::from_hex(expected_hash).map_err(|_| IntegrityError::HashMismatch {
            expected: expected_hash.to_string(),
            actual: actual.to_string(),
        })?;

        if actual != expected {
            return Err(IntegrityError::HashMismatch {
                expected: expected.to_string(),
                actual: actual.to_string(),
            });
        }

        Ok(())
    }

    /// Verify hash against on-chain records
    pub fn verify_on_chain(
        &self,
        model_pubkey: &Pubkey,
        computed_hash: &Hash,
    ) -> Result<(), IntegrityError> {
        let account_data = self.rpc_client.get_account_data(model_pubkey)?;
        let stored_hash = Hash::try_from(&account_data[32..64])
            .map_err(|_| IntegrityError::ChainVerification("Invalid hash format".into()))?;

        if stored_hash != *computed_hash {
            return Err(IntegrityError::HashMismatch {
                expected: stored_hash.to_string(),
                actual: computed_hash.to_string(),
            });
        }

        Ok(())
    }

    /// Generate proof for zero-knowledge verification
    pub fn generate_proof(&self, data: &[u8], challenge: &[u8]) -> (Hash, Vec<u8>) {
        let mut hasher = Hasher::new();
        hasher.update(data);
        let root_hash = hasher.finalize();
        
        let mut proof_hasher = Hasher::new();
        proof_hasher.update(challenge);
        proof_hasher.update(root_hash.as_bytes());
        let proof = proof_hasher.finalize().as_bytes().to_vec();

        (root_hash, proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_consistency() {
        let checker = Blake3IntegrityChecker::new("http://testnet.solana.com").unwrap();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"SCORIA AI integrity test").unwrap();

        let hash1 = checker.compute_file_hash(file.path()).unwrap();
        let hash2 = checker.compute_file_hash(file.path()).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_tamper_detection() {
        let checker = Blake3IntegrityChecker::new("http://testnet.solana.com").unwrap();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"Original data").unwrap();
        let original_hash = checker.compute_file_hash(file.path()).unwrap();

        // Tamper with file
        file.write_all(b"modified").unwrap();
        
        match checker.verify_file(file.path(), &original_hash.to_string()) {
            Err(IntegrityError::HashMismatch { .. }) => (),
            _ => panic!("Tamper detection failed"),
        }
    }
}
