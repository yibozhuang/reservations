use anyhow::Result;
use dotenv::dotenv;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;

pub mod proto {
    tonic::include_proto!("reservations");
}

pub mod db;
pub mod service;

use db::ReservationRepository;
use proto::reservation_service_server::ReservationServiceServer;
use service::ReservationServiceImpl;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Setup logging
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

    // Get server address from environment or use default
    let addr = env::var("SERVER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse::<SocketAddr>()?;

    tracing::info!("Connecting to database...");
    // Create database connection pool
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations to ensure database schema is up to date
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./db").run(&pool).await?;

    // Create repository
    let repository = Arc::new(ReservationRepository::new(pool));

    // Create gRPC service
    let reservation_service = ReservationServiceImpl::new(repository);

    // Create gRPC server
    tracing::info!("Starting gRPC server on {}", addr);
    Server::builder()
        .add_service(ReservationServiceServer::new(reservation_service))
        .serve(addr)
        .await?;

    Ok(())
}
