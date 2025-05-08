#!/usr/bin/env bash
set -eo pipefail

# Environment Configuration
export SCORIA_ENV="production"
export RUST_LOG="info,solana_client=warn,hyper=error"
export DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
export SOLANA_CLUSTER="mainnet-beta"
export SOLANA_RPC="https://api.mainnet-beta.solana.com"
export AWS_REGION="us-west-2"
export GPU_ENABLED="true"

# Security Parameters
export KEY_MANAGEMENT="vault"
export TLS_CERT_PATH="/etc/ssl/certs/scoria-fullchain.pem"
export TLS_KEY_PATH="/etc/ssl/private/scoria-privkey.pem"

# Performance Tuning
export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld"
export WASM_MAX_MEMORY="4gb"
export WASM_THREADS=$(( $(nproc) * 2 ))

# Directory Configuration
BASE_DIR=$(dirname "$(readlink -f "\$0")")
BIN_DIR="$BASE_DIR/target/release"
CONFIG_DIR="$BASE_DIR/config"
LOG_DIR="/var/log/scoria"
PID_DIR="/var/run/scoria"

# Dependency Check
declare -a DEPS=(
    "cargo" "ld.lld" "postgres" "nvidia-smi" 
    "solana" "jq" "tmux" "aws"
)

check_dependencies() {
    for dep in "${DEPS[@]}"; do
        if ! command -v $dep &> /dev/null; then
            echo "CRITICAL: Missing dependency $dep"
            exit 1
        fi
    done
}

# Database Initialization
init_database() {
    psql $DATABASE_URL <<-EOSQL
        CREATE ROLE IF NOT EXISTS ${DB_USER} WITH LOGIN PASSWORD '${DB_PASS}';
        CREATE DATABASE IF NOT EXISTS ${DB_NAME} WITH OWNER ${DB_USER};
        GRANT ALL PRIVILEGES ON DATABASE ${DB_NAME} TO ${DB_USER};
EOSQL

    diesel migration run --database-url $DATABASE_URL
}

# Hardware Validation
validate_gpu() {
    if [ "$GPU_ENABLED" = "true" ]; then
        if ! nvidia-smi --query-gpu=driver_version --format=noheader; then
            echo "GPU acceleration required but not available"
            exit 1
        fi
    fi
}

# Security Setup
setup_tls() {
    if [ ! -f $TLS_CERT_PATH ] || [ ! -f $TLS_KEY_PATH ]; then
        openssl req -x509 -nodes -days 365 \
            -newkey rsa:4096 -sha256 \
            -keyout $TLS_KEY_PATH \
            -out $TLS_CERT_PATH \
            -subj "/CN=scoria-indexer"
    fi
}

# Process Management
start_service() {
    mkdir -p $LOG_DIR $PID_DIR
    tmux new-session -d -s scoria_indexer \
        "$BIN_DIR/indexer \
            --config $CONFIG_DIR/solana_cluster.toml \
            --log-dir $LOG_DIR \
            --pid-file $PID_DIR/indexer.pid \
            --tls-cert $TLS_CERT_PATH \
            --tls-key $TLS_KEY_PATH \
        2>&1 | tee -a $LOG_DIR/indexer.log"
}

# Main Execution
main() {
    check_dependencies
    validate_gpu
    setup_tls
    init_database
    
    echo "Compiling indexer..."
    cargo build --release --features "gpu-accel,aws-s3"
    
    echo "Starting SCORIA Indexer..."
    start_service
    
    echo "Monitoring logs..."
    tail -f $LOG_DIR/indexer.log
}

main "$@"
