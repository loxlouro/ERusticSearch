use rust_search::{api::routes::create_routes, common::config::Config, SearchEngine};
use std::sync::Arc;
use tokio::signal::ctrl_c;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
    tracing::info!("Загружена конфигурация: {:?}", config);

    let search_engine = SearchEngine::new(&config)?;
    let search_engine = Arc::new(search_engine);
    let search_engine_clone = search_engine.clone();

    let routes = create_routes(search_engine);

    let addr = (config.server.host, config.server.port);
    tracing::info!(
        "Сервер запущен на http://{}:{}",
        config.server.host,
        config.server.port
    );

    let (_, _) = tokio::join!(
        async move {
            match ctrl_c().await {
                Ok(()) => {
                    tracing::info!("Получен сигнал завершения, закрываем движок...");
                    if let Err(e) = search_engine_clone.close().await {
                        tracing::error!("Ошибка при закрытии дви��ка: {}", e);
                    }
                }
                Err(err) => {
                    tracing::error!("Ошибка при ожидании Ctrl+C: {}", err);
                }
            }
        },
        warp::serve(routes).run(addr)
    );

    Ok(())
}
