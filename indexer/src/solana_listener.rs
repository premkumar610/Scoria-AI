// indexer/src/solana_listener.rs

use solana_client::{
    nonblocking::websocket::WebSocketRpcClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::{
    sync::{mpsc, Mutex},
    time::{sleep, Duration},
};
use tracing::{error, info, instrument, warn};

const RECONNECT_BACKOFF: [u64; 5] = [1, 2, 5, 10, 30]; // Seconds
const MAX_RETRIES: usize = 10;

#[derive(Clone)]
pub struct SolanaEventListener {
    ws_client: Arc<Mutex<Option<WebSocketRpcClient>>>,
    config: Arc<EventListenerConfig>,
    db_pool: PgPool,
    kafka_producer: FutureProducer,
}

impl SolanaEventListener {
    pub async fn new(
        config: EventListenerConfig,
        db_pool: PgPool,
        kafka_producer: FutureProducer,
    ) -> Self {
        Self {
            ws_client: Arc::new(Mutex::new(None)),
            config: Arc::new(config),
            db_pool,
            kafka_producer,
        }
    }

    #[instrument(skip_all)]
    pub async fn run(&self, shutdown: mpsc::Sender<()>) -> anyhow::Result<()> {
        let mut retry_count = 0;
        loop {
            match self.connect().await {
                Ok(mut client) => {
                    retry_count = 0;
                    if let Err(e) = self.process_events(&mut client).await {
                        error!(error = %e, "Event processing error");
                    }
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count > MAX_RETRIES {
                        error!("Max connection retries exceeded");
                        shutdown.send(()).await?;
                        break;
                    }
                    
                    let delay = Duration::from_secs(RECONNECT_BACKOFF[retry_count - 1]);
                    warn!(retries = retry_count, delay_secs = delay.as_secs(), "Reconnecting...");
                    sleep(delay).await;
                }
            }
        }
        Ok(())
    }

    async fn connect(&self) -> anyhow::Result<WebSocketRpcClient> {
        let client = WebSocketRpcClient::new_with_commitment(
            &self.config.ws_endpoint,
            CommitmentConfig::confirmed(),
        )
        .await?;

        let filter = RpcTransactionLogsFilter::Mentions(vec![self.config.program_id.to_string()]);
        let config = RpcTransactionLogsConfig {
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        };

        client
            .logs_subscribe(filter, config)
            .await
            .context("Failed to subscribe to logs")?;

        info!("WebSocket connected successfully");
        Ok(client)
    }

    #[instrument(skip(client))]
    async fn process_events(&self, client: &mut WebSocketRpcClient) -> anyhow::Result<()> {
        let mut stream = client.logs_notifications().await?;
        
        while let Some(notification) = stream.next().await {
            let logs = notification?.value.logs.join("\n");
            
            let event = match parse_logs(&logs) {
                Some(e) => e,
                None => continue,
            };

            self.handle_event(event).await?;
        }

        Ok(())
    }

    async fn handle_event(&self, event: ProgramEvent) -> anyhow::Result<()> {
        // Database transaction
        let mut tx = self.db_pool.begin().await?;

        // Store raw event
        sqlx::query!(
            r#"INSERT INTO events (signature, slot, data) VALUES (\$1, \$2, \$3)"#,
            event.signature,
            event.slot,
            serde_json::to_value(&event)?
        )
        .execute(&mut *tx)
        .await?;

        // Process event type
        match event.inner {
            ProgramEventType::ModelRegistered(model) => {
                self.handle_model_registration(&mut tx, model).await?;
            }
            ProgramEventType::ModelUpdated(update) => {
                self.handle_model_update(&mut tx, update).await?;
            }
            ProgramEventType::ModelDeleted(deletion) => {
                self.handle_model_deletion(&mut tx, deletion).await?;
            }
        }

        // Commit transaction
        tx.commit().await?;

        // Publish to Kafka
        let record = FutureRecord::to(&self.config.kafka_topic)
            .payload(&serde_json::to_vec(&event)?)
            .key(&event.signature);
        
        self.kafka_producer
            .send(record, Duration::from_secs(30))
            .await??;

        metrics::increment_counter!("events_processed_total", "type" => event.event_type());
        Ok(())
    }

    async fn handle_model_registration(
        &self,
        tx: &mut PgConnection,
        model: ModelRegistration,
    ) -> anyhow::Result<()> {
        // Insert into registry
        sqlx::query!(
            r#"INSERT INTO models (id, owner, metadata, created_at)
               VALUES (\$1, \$2, \$3, NOW())
               ON CONFLICT (id) DO NOTHING"#,
            model.id,
            model.owner,
            model.metadata
        )
        .execute(&mut *tx)
        .await?;

        // Insert permissions
        for perm in model.permissions {
            sqlx::query!(
                r#"INSERT INTO model_permissions (model_id, user_id, access_level)
                   VALUES (\$1, \$2, \$3)
                   ON CONFLICT (model_id, user_id) DO UPDATE
                   SET access_level = EXCLUDED.access_level"#,
                model.id,
                perm.user,
                perm.access_level as i32
            )
            .execute(&mut *tx)
            .await?;
        }

        Ok(())
    }

    // Additional handlers for updates/deletions...
}

// Event parsing implementation
fn parse_logs(logs: &str) -> Option<ProgramEvent> {
    // Custom parsing logic matching program IDL
}

// Metrics and health checks...
