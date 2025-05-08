// indexer/src/main.rs

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    let config = Config::load("config/prod.toml")
        .context("Failed to load configuration")?;

    // Initialize database connection pool
    let db_pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    // Initialize Solana client
    let solana_client = RpcClient::new_with_commitment(
        config.solana.rpc_endpoint.clone(),
        CommitmentConfig::confirmed()
    );

    // Initialize Kafka producer
    let kafka_producer = FutureProducer::new(
        config.kafka.to_client_config()
            .set("message.timeout.ms", "30000")
            .set("compression.codec", "zstd")
    )?;

    // Start health check server
    let health_server = start_health_server(config.monitoring.health_check_port);

    // Create shutdown signal channel
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    // Spawn main indexing tasks
    let tasks = join!(
        spawn_block_processor(
            solana_client.clone(),
            db_pool.clone(),
            kafka_producer.clone(),
            shutdown_tx.clone(),
        ),
        spawn_rpc_listener(
            solana_client.clone(),
            kafka_producer.clone(),
            shutdown_tx.clone(),
        ),
        spawn_stream_consumer(
            db_pool.clone(),
            shutdown_tx.clone(),
        ),
    );

    // Handle graceful shutdown
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Received SIGINT, initiating shutdown");
        },
        _ = shutdown_rx.recv() => {
            tracing::warn!("Internal shutdown signal received");
        },
    }

    // Graceful termination
    tracing::info!("Draining resources...");
    kafka_producer.flush(None).await?;
    db_pool.close().await;
    health_server.abort();

    // Wait for tasks completion
    let (block_res, rpc_res, stream_res) = tasks;
    block_res??;
    rpc_res??;
    stream_res??;

    tracing::info!("Indexer shutdown complete");
    Ok(())
}

async fn spawn_block_processor(
    client: RpcClient,
    db_pool: PgPool,
    kafka_producer: FutureProducer,
    shutdown: Sender<()>,
) -> anyhow::Result<()> {
    // Implementation details...
}

async fn spawn_rpc_listener(
    client: RpcClient,
    kafka_producer: FutureProducer,
    shutdown: Sender<()>,
) -> anyhow::Result<()> {
    // WebSocket subscription logic...
}

async fn spawn_stream_consumer(
    db_pool: PgPool,
    shutdown: Sender<()>,
) -> anyhow::Result<()> {
    // Kafka consumer logic...
}

fn start_health_server(port: u16) -> JoinHandle<()> {
    // HTTP endpoint implementation...
}
