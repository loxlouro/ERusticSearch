use rust_search::{api::routes::search_routes, common::config::Config, core::search::SearchEngine};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal::ctrl_c;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
    info!("Loaded configuration: {:?}", config);

    let engine = Arc::new(SearchEngine::new(&config)?);
    info!("Search engine initialized");

    let addr = SocketAddr::new(config.server.host, config.server.port);
    let routes = search_routes(engine);

    info!("Starting server on {}", addr);

    let server = warp::serve(routes).run(addr);
    let shutdown = async {
        match ctrl_c().await {
            Ok(()) => {
                info!("Shutdown signal received, stopping server...");
            }
            Err(err) => {
                error!("Error listening for shutdown signal: {}", err);
            }
        }
    };

    tokio::select! {
        _ = server => {
            info!("Server stopped");
        }
        _ = shutdown => {
            info!("Shutting down...");
        }
    }

    Ok(())
}
