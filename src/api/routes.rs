use super::handlers;
use crate::core::search::SearchEngine;
use std::sync::Arc;
use warp::Filter;

pub fn search_routes(
    engine: Arc<SearchEngine>,
) -> impl Filter<Extract = impl warp::Reply, Error = std::convert::Infallible> + Clone {
    let search = warp::path("search")
        .and(warp::get())
        .and(warp::query())
        .and(with_engine(engine.clone()))
        .and_then(handlers::handle_search);

    let add = warp::path("documents")
        .and(warp::post())
        .and(handlers::json_body())
        .and(with_engine(engine))
        .and_then(handlers::handle_add_document);

    search.or(add).recover(handlers::handle_rejection)
}

fn with_engine(
    engine: Arc<SearchEngine>,
) -> impl Filter<Extract = (Arc<SearchEngine>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || engine.clone())
}
