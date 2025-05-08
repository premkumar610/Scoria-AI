// local_engine/src/db/redis.rs

use deadpool::managed::{Manager, Pool, RecycleResult};
use redis::{aio::Connection, Client, Cmd, IntoConnectionInfo, RedisResult};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, instrument};

/// Redis node configuration
#[derive(Debug, Clone)]
pub enum RedisConfig {
    SingleNode {
        url: String,
        read_only: bool,
    },
    Sentinel {
        sentinel_urls: Vec<String>,
        service_name: String,
        read_only: bool,
    },
}

/// Redis connection manager with health checks
#[derive(Clone)]
pub struct RedisManager {
    client: Client,
    config: RedisConfig,
}

impl Manager for RedisManager {
    type Type = Connection;
    type Error = redis::RedisError;

    async fn create(&self) -> Result<Connection, Self::Error> {
        let mut conn = self.client.get_async_connection().await?;
        
        // Set read-only mode for replicas
        if let RedisConfig::SingleNode { read_only, .. } | 
            RedisConfig::Sentinel { read_only, .. } = &self.config 
        {
            if *read_only {
                Cmd::new().arg("READONLY").query_async(&mut conn).await?;
            }
        }
        
        Ok(conn)
    }

    async fn recycle(&self, conn: &mut Connection) -> RecycleResult<Self::Error> {
        // Health check using PING
        match timeout(Duration::from_secs(2), Cmd::new().arg("PING").query_async(conn)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                error!(error = %e, "Redis connection health check failed");
                Err(e.into())
            },
            Err(_) => {
                error!("Redis health check timeout");
                Err(deadpool::managed::RecycleError::Message(
                    "Health check timeout".into(),
                ))
            },
        }
    }
}

/// Redis connection pool with failover support
pub struct RedisPool {
    primary_pool: Pool<RedisManager>,
    replica_pool: Option<Pool<RedisManager>>,
}

impl RedisPool {
    /// Initialize connection pool with config
    #[instrument(skip_all)]
    pub async fn new(config: RedisConfig) -> RedisResult<Self> {
        let (primary_manager, replica_manager) = match &config {
            RedisConfig::SingleNode { url, read_only } => {
                let client = Client::open(url.as_str())?;
                let primary_manager = RedisManager {
                    client: client.clone(),
                    config: config.clone(),
                };
                
                // Initialize replica pool if needed
                let replica_manager = if *read_only {
                    None
                } else {
                    Some(RedisManager {
                        client,
                        config: RedisConfig::SingleNode { 
                            url: url.clone(), 
                            read_only: true 
                        },
                    })
                };
                
                (primary_manager, replica_manager)
            }
            RedisConfig::Sentinel { 
                sentinel_urls, 
                service_name, 
                read_only 
            } => {
                let client = Client::open(
                    redis::ConnectionInfo {
                        addr: redis::ConnectionAddr::RedisSentinel {
                            hosts: sentinel_urls.clone(),
                            service_name: service_name.clone(),
                        },
                        redis: Default::default(),
                    }
                )?;
                
                let primary_manager = RedisManager {
                    client: client.clone(),
                    config: config.clone(),
                };
                
                let replica_manager = if *read_only {
                    None
                } else {
                    Some(RedisManager {
                        client,
                        config: RedisConfig::Sentinel { 
                            sentinel_urls: sentinel_urls.clone(), 
                            service_name: service_name.clone(), 
                            read_only: true 
                        },
                    })
                };
                
                (primary_manager, replica_manager)
            }
        };

        // Build pools with exponential backoff
        let primary_pool = Pool::builder(primary_manager)
            .max_size(20)
            .create_timeout(Some(Duration::from_secs(5)))
            .wait_timeout(Some(5)) // 5 seconds
            .recycle_timeout(Some(Duration::from_secs(300)))
            .post_create(Box::new(|_, _| async { info!("New Redis connection created"); }))
            .build()?;

        let replica_pool = replica_manager.map(|manager| {
            Pool::builder(manager)
                .max_size(10)
                .create_timeout(Some(Duration::from_secs(5)))
                .wait_timeout(Some(2)) // 2 seconds
                .build()
                .expect("Failed to build replica pool")
        });

        Ok(Self {
            primary_pool,
            replica_pool,
        })
    }

    /// Get connection with read/write preference
    #[instrument(skip(self))]
    pub async fn get_conn(&self, read_only: bool) -> RedisResult<Connection> {
        if read_only {
            if let Some(replica_pool) = &self.replica_pool {
                return self.get_conn_with_retry(replica_pool, 3).await;
            }
        }
        
        self.get_conn_with_retry(&self.primary_pool, 3).await
    }

    /// Retry logic with exponential backoff
    async fn get_conn_with_retry(
        &self,
        pool: &Pool<RedisManager>,
        retries: usize,
    ) -> RedisResult<Connection> {
        let mut attempt = 0;
        let mut backoff = Duration::from_millis(100);
        
        loop {
            match pool.get().await {
                Ok(conn) => return Ok(conn),
                Err(e) if attempt < retries => {
                    error!(
                        error = %e,
                        attempt,
                        "Failed to get Redis connection, retrying..."
                    );
                    attempt += 1;
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

/// Example usage:
/// 
/// let pool = RedisPool::new(config).await?;
/// let mut conn = pool.get_conn(false).await?; // Write connection
/// redis::cmd("SET").arg("key").arg("value").query_async(&mut conn).await?;
/// 
/// let mut read_conn = pool.get_conn(true).await?; // Read connection
/// let value: String = redis::cmd("GET").arg("key").query_async(&mut read_conn).await?;
