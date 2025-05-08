// contracts/programs/model_registry/src/utils.rs

use anchor_lang::prelude::*;
use sha3::{Digest, Keccak256, Sha3_256};
use blake3::Hasher as Blake3;
use solana_program::keccak::hashv;
use std::io::{Read, Write};

/// ZK-friendly hashing utilities using algebraic constructions
pub mod zk_hashes {
    use super::*;
    use poseidon_rs::Poseidon;
    use ff::PrimeField;
    use paired::bls12_381::Fr;

    /// Generates Poseidon hash for ZK circuits (BLS12-381)
    pub fn poseidon_hash(inputs: &[Fr]) -> Result<Fr> {
        let poseidon = Poseidon::new();
        poseidon.hash(inputs).map_err(|e| {
            msg!("Poseidon hash error: {:?}", e);
            ModelRegistryError::ZkVerificationFailure.into()
        })
    }

    /// Converts bytes to field elements for circuit input
    pub fn bytes_to_field_elements(data: &[u8]) -> Result<Vec<Fr>> {
        data.chunks(32)
            .map(|chunk| {
                let mut buf = [0u8; 32];
                buf[..chunk.len()].copy_from_slice(chunk);
                Fr::from_repr(buf).ok_or_else(|| {
                    msg!("Invalid bytes for field element conversion");
                    ModelRegistryError::CurveParameterMismatch.into()
                })
            })
            .collect()
    }
}

/// Standard hash functions for on-chain verification
pub mod onchain_hashes {
    use super::*;

    /// Blake3 hash optimized for Solana's runtime
    pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Blake3::new();
        hasher.update(data);
        let mut output = [0u8; 32];
        output.copy_from_slice(&hasher.finalize()[..32]);
        output
    }

    /// Keccak256 hash compatible with EVM chains
    pub fn keccak256(data: &[u8]) -> [u8; 32] {
        Keccak256::digest(data).into()
    }

    /// Solana-optimized parallel hash
    pub fn solana_hash(data: &[&[u8]]) -> [u8; 32] {
        hashv(data).to_bytes()
    }
}

/// Merkle tree operations for version history
pub mod merkle_utils {
    use super::*;
    use merlin::Transcript;
    use bulletproofs::PedersenGens;

    /// Generate Merkle root from version hashes
    pub fn merkle_root(versions: &[[u8; 32]]) -> Result<[u8; 32]> {
        if versions.is_empty() {
            return Err(ModelRegistryError::InvalidHashLength.into());
        }

        let mut leaves = versions.to_vec();
        while leaves.len() > 1 {
            leaves = leaves.chunks(2).map(|pair| {
                let mut hasher = Blake3::new();
                hasher.update(pair[0]);
                if pair.len() > 1 {
                    hasher.update(pair[1]);
                } else {
                    hasher.update(&[0u8; 32]);
                }
                let mut output = [0u8; 32];
                output.copy_from_slice(&hasher.finalize()[..32]);
                output
            }).collect();
        }

        Ok(leaves[0])
    }

    /// Generate Merkle proof for specific version
    pub fn merkle_proof(versions: &[[u8; 32]], index: usize) -> Result<Vec<[u8; 32]>> {
        let mut proof = Vec::new();
        let mut current_index = index;
        let mut current_level = versions.to_vec();

        while current_level.len() > 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < current_level.len() {
                proof.push(current_level[sibling_index]);
            }

            current_index /= 2;
            current_level = current_level.chunks(2)
                .map(|pair| {
                    let mut hasher = Blake3::new();
                    hasher.update(pair[0]);
                    if pair.len() > 1 {
                        hasher.update(pair[1]);
                    } else {
                        hasher.update(&[0u8; 32]);
                    }
                    let mut output = [0u8; 32];
                    output.copy_from_slice(&hasher.finalize()[..32]);
                    output
                })
                .collect();
        }

        Ok(proof)
    }
}

/// Streaming hash for large model files
pub struct ModelHasher {
    blake3: Blake3,
    sha3: Sha3_256,
    keccak: Keccak256,
}

impl ModelHasher {
    pub fn new() -> Self {
        Self {
            blake3: Blake3::new(),
            sha3: Sha3_256::new(),
            keccak: Keccak256::new(),
        }
    }

    /// Update hashers with model chunk
    pub fn update(&mut self, data: &[u8]) {
        self.blake3.update(data);
        self.sha3.update(data);
        self.keccak.update(data);
    }

    /// Finalize all hashes
    pub fn finalize(self) -> ([u8; 32], [u8; 32], [u8; 32]) {
        (
            self.blake3.finalize().into(),
            self.sha3.finalize().into(),
            self.keccak.finalize().into()
        )
    }
}

/// File verification utilities
pub mod file_utils {
    use super::*;
    use std::path::Path;

    /// Memory-mapped file hashing
    pub fn hash_large_file(path: &Path) -> Result<[u8; 32]> {
        let file = std::fs::File::open(path)
            .map_err(|_| ModelRegistryError::ModelSizeExceeded)?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file) }
            .map_err(|_| ModelRegistryError::ModelSizeExceeded)?;

        let mut hasher = Blake3::new();
        for chunk in mmap.chunks(1024 * 1024 * 64) { // 64MB chunks
            hasher.update(chunk);
        }
        
        let mut output = [0u8; 32];
        output.copy_from_slice(&hasher.finalize()[..32]);
        Ok(output)
    }

    /// Verify file against multiple hashes
    pub fn verify_file(
        path: &Path,
        blake3_hash: &[u8; 32],
        sha3_hash: &[u8; 32]
    ) -> Result<bool> {
        let computed_blake3 = hash_large_file(path)?;
        let computed_sha3 = {
            let mut hasher = Sha3_256::new();
            let mut file = std::fs::File::open(path)?;
            let mut buffer = [0; 1024];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 { break; }
                hasher.update(&buffer[..count]);
            }
            hasher.finalize().into()
        };

        Ok(computed_blake3 == *blake3_hash && computed_sha3 == *sha3_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::io::Write;

    #[test]
    fn test_streaming_hasher() {
        let mut hasher = ModelHasher::new();
        hasher.update(b"SCORIA");
        hasher.update(b"AI");
        let (b, s, k) = hasher.finalize();
        
        assert_eq!(b, Blake3::hash(b"SCORIAAI"));
        assert_eq!(s, Sha3_256::digest(b"SCORIAAI").into());
        assert_eq!(k, Keccak256::digest(b"SCORIAAI").into());
    }

    #[test]
    fn test_large_file_hashing() {
        let mut path = temp_dir();
        path.push("test_model.bin");
        
        // Generate 128MB test file
        let mut file = std::fs::File::create(&path).unwrap();
        let data = vec![0xAAu8; 1024 * 1024]; // 1MB chunk
        for _ in 0..128 {
            file.write_all(&data).unwrap();
        }
        
        let hash = file_utils::hash_large_file(&path).unwrap();
        let expected = Blake3::hash(&vec![0xAA; 1024 * 1024 * 128]);
        assert_eq!(hash, expected.into());
    }
}
