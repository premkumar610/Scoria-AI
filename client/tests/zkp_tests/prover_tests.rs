// local_engine/src/zk/tests/prover_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::{Bn254, Fr};
    use ark_circom::CircomBuilder;
    use ark_groth16::ProvingKey;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use tempfile::TempDir;
    use std::time::Duration;

    const TEST_CIRCUIT_WASM: &[u8] = include_bytes!("../../test_data/circuit.wasm");
    const TEST_PROVING_KEY: &[u8] = include_bytes!("../../test_data/proving_key.bin");

    #[tokio::test]
    async fn test_valid_proof_generation() {
        let temp_dir = TempDir::new().unwrap();
        let prover = setup_prover(temp_dir.path()).await;

        let inputs = vec![
            ("a".to_string(), Value::Number(3.into())),
            ("b".to_string(), Value::Number(11.into()))
        ];

        let proof = prover.generate_proof("multiplier", inputs).await.unwrap();
        
        assert!(proof.verify(&prover.verification_key).await.unwrap());
    }

    #[tokio::test]
    async fn test_invalid_input_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let prover = setup_prover(temp_dir.path()).await;

        let invalid_inputs = vec![
            ("a".to_string(), Value::String("text".into())) // Should be number
        ];

        let result = prover.generate_proof("multiplier", invalid_inputs).await;
        
        assert_matches!(result, Err(ProverError::CircuitSynthesis(_)));
    }

    #[tokio::test]
    async fn test_proof_serialization() {
        let temp_dir = TempDir::new().unwrap();
        let prover = setup_prover(temp_dir.path()).await;

        let inputs = vec![
            ("a".to_string(), Value::Number(5.into())),
            ("b".to_string(), Value::Number(7.into()))
        ];

        let proof = prover.generate_proof("multiplier", inputs).await.unwrap();
        let serialized = proof.serialize().unwrap();
        let deserialized = Groth16Proof::deserialize(&serialized).unwrap();
        
        assert_eq!(proof, deserialized);
    }

    #[tokio::test]
    async fn test_gpu_acceleration() {
        let temp_dir = TempDir::new().unwrap();
        let mut prover = setup_prover(temp_dir.path()).await;
        prover.enable_gpu(true);

        let inputs = vec![
            ("a".to_string(), Value::Number(2.into())),
            ("b".to_string(), Value::Number(13.into()))
        ];

        let start = Instant::now();
        let proof = prover.generate_proof("multiplier", inputs).await.unwrap();
        let duration = start.elapsed();

        assert!(duration < Duration::from_secs(2));
        assert!(proof.gpu_accelerated);
    }

    #[tokio::test]
    async fn test_memory_constraints() {
        let temp_dir = TempDir::new().unwrap();
        let prover = setup_prover(temp_dir.path()).await;

        let large_inputs = (0..1000)
            .map(|i| (format!("input_{}", i), Value::Number(i.into())))
            .collect();

        let result = prover.generate_proof("large_circuit", large_inputs).await;
        
        assert_matches!(result, Err(ProverError::MemoryLimitExceeded));
    }

    #[tokio::test]
    async fn test_concurrent_proving() {
        let temp_dir = TempDir::new().unwrap();
        let prover = Arc::new(setup_prover(temp_dir.path()).await);

        let handles: Vec<_> = (0..10).map(|i| {
            let prover = prover.clone();
            tokio::spawn(async move {
                let inputs = vec![
                    ("a".to_string(), Value::Number(i.into())),
                    ("b".to_string(), Value::Number((i+1).into()))
                ];
                prover.generate_proof("multiplier", inputs).await
            })
        }).collect();

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_circuit_verification() {
        let temp_dir = TempDir::new().unwrap();
        let prover = setup_prover(temp_dir.path()).await;

        let mut builder = CircomBuilder::new();
        builder.push_input("a", 3);
        builder.push_input("b", 5);
        let circuit = builder.build().unwrap();

        let cs = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs).unwrap();
        
        assert!(cs.is_satisfied().unwrap());
        assert_eq!(cs.num_constraints(), 4);
    }

    #[tokio::test]
    async fn test_proof_benchmark() {
        let temp_dir = TempDir::new().unwrap();
        let prover = setup_prover(temp_dir.path()).await;

        let inputs = vec![
            ("a".to_string(), Value::Number(7.into())),
            ("b".to_string(), Value::Number(19.into()))
        ];

        let stats = prover.benchmark("multiplier", inputs, 100).await.unwrap();
        
        assert!(stats.avg_time < Duration::from_millis(500));
        assert!(stats.max_memory < 500_000); // < 500MB
    }

    async fn setup_prover(tmp_dir: &Path) -> Groth16Prover<Bn254> {
        let circuit_path = tmp_dir.join("circuit.wasm");
        std::fs::write(&circuit_path, TEST_CIRCUIT_WASM).unwrap();

        let pk_path = tmp_dir.join("proving_key.bin");
        std::fs::write(&pk_path, TEST_PROVING_KEY).unwrap();

        Groth16Prover::new(
            circuit_path,
            pk_path,
            ZkConfig {
                max_memory_mb: 1024,
                gpu_enabled: false,
                ..Default::default()
            }
        ).await.unwrap()
    }
}
