// local_engine/src/model_loader/tests/model_loader_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{File, self};
    use std::io::Write;
    use assert_matches::assert_matches;

    const TEST_MODEL_DATA: &[u8] = b"SCORIA AI test model data v1.0";

    #[tokio::test]
    async fn test_load_valid_local_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("test.onnx");
        let mut file = File::create(&model_path).unwrap();
        file.write_all(TEST_MODEL_DATA).unwrap();

        let loader = ModelLoader::new(temp_dir.path());
        let result = loader.load_model("test", ModelSource::Local(model_path)).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().data, TEST_MODEL_DATA);
    }

    #[tokio::test]
    async fn test_load_invalid_hash_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("corrupted.onnx");
        let mut file = File::create(&model_path).unwrap();
        file.write_all(b"corrupted data").unwrap();

        let loader = ModelLoader::new(temp_dir.path());
        let result = loader.load_model(
            "test", 
            ModelSource::IPFS(
                "QmValidCID".into(), 
                blake3::hash(TEST_MODEL_DATA)
            )
        ).await;

        assert_matches!(result, Err(ModelLoaderError::InvalidHash));
    }

    #[tokio::test]
    async fn test_cache_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ModelLoader::new(temp_dir.path());
        
        // First load (download)
        let result1 = loader.load_model(
            "cached_model",
            ModelSource::IPFS("QmTestCID".into(), blake3::hash(TEST_MODEL_DATA))
        ).await;
        assert!(result1.is_ok());

        // Second load (cache hit)
        let result2 = loader.load_model(
            "cached_model",
            ModelSource::IPFS("QmTestCID".into(), blake3::hash(TEST_MODEL_DATA))
        ).await;
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap().path, result2.unwrap().path);
    }

    #[tokio::test]
    async fn test_concurrent_loading() {
        let temp_dir = TempDir::new().unwrap();
        let loader = Arc::new(ModelLoader::new(temp_dir.path()));
        
        let handles: Vec<_> = (0..10).map(|i| {
            let loader = loader.clone();
            tokio::spawn(async move {
                loader.load_model(
                    &format!("concurrent_model_{}", i),
                    ModelSource::IPFS(
                        "QmConcurrentCID".into(), 
                        blake3::hash(TEST_MODEL_DATA)
                    )
                ).await
            })
        }).collect();

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_network_failure_retry() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ModelLoader::builder()
            .with_retries(3)
            .build(temp_dir.path());

        let result = loader.load_model(
            "invalid_model",
            ModelSource::IPFS(
                "QmInvalidCID".into(), 
                blake3::hash(b"")
            )
        ).await;

        assert_matches!(result, Err(ModelLoaderError::NetworkFailure));
    }

    #[tokio::test]
    async fn test_large_model_handling() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("large_model.onnx");
        let mut file = File::create(&model_path).unwrap();
        let large_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
        file.write_all(&large_data).unwrap();

        let loader = ModelLoader::new(temp_dir.path());
        let result = loader.load_model(
            "large_model",
            ModelSource::Local(model_path)
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().data.len(), 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_malformed_model_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("malformed.onnx");
        let mut file = File::create(&model_path).unwrap();
        file.write_all(b"malformed content").unwrap();

        let loader = ModelLoader::builder()
            .with_integrity_check(true)
            .build(temp_dir.path());

        let result = loader.load_model(
            "malformed_model",
            ModelSource::IPFS(
                "QmMalformedCID".into(),
                blake3::hash(b"valid data")
            )
        ).await;

        assert_matches!(result, Err(ModelLoaderError::ValidationFailed));
    }

    #[tokio::test]
    async fn test_version_rollback_protection() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ModelLoader::new(temp_dir.path());

        // Initial version
        let result1 = loader.load_model(
            "versioned_model",
            ModelSource::IPFS(
                "QmVersion1".into(),
                blake3::hash(b"v1")
            )
        ).await;
        assert!(result1.is_ok());

        // Attempt to load older version
        let result2 = loader.load_model(
            "versioned_model",
            ModelSource::IPFS(
                "QmVersion2".into(),
                blake3::hash(b"v0")
            )
        ).await;

        assert_matches!(result2, Err(ModelLoaderError::VersionConflict));
    }

    #[tokio::test]
    async fn test_cache_poisoning_protection() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ModelLoader::new(temp_dir.path());

        // First valid load
        let result1 = loader.load_model(
            "secure_model",
            ModelSource::IPFS(
                "QmSecureCID".into(),
                blake3::hash(TEST_MODEL_DATA)
            )
        ).await;
        assert!(result1.is_ok());

        // Manually corrupt cache
        let cache_path = loader.cache_dir.join("secure_model.onnx");
        fs::write(cache_path, "corrupted").unwrap();

        // Subsequent load should detect corruption
        let result2 = loader.load_model(
            "secure_model",
            ModelSource::IPFS(
                "QmSecureCID".into(),
                blake3::hash(TEST_MODEL_DATA)
            )
        ).await;

        assert_matches!(result2, Err(ModelLoaderError::CacheIntegrityFailure));
    }
}
