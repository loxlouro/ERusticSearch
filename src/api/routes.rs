use super::handlers::{handle_add_document, handle_rejection, handle_search};
use super::middleware::json_body;
use crate::core::search::SearchEngine;
use std::sync::Arc;
use warp::Filter;

pub fn create_routes(
    search_engine: Arc<SearchEngine>,
) -> impl warp::Filter<Extract = impl warp::Reply> + Clone {
    let search_engine_filter = warp::any().map(move || search_engine.clone());

    let add_document = warp::post()
        .and(warp::path("document"))
        .and(json_body())
        .and(search_engine_filter.clone())
        .and_then(handle_add_document);

    let search = warp::get()
        .and(warp::path("search"))
        .and(warp::query())
        .and(search_engine_filter.clone())
        .and_then(handle_search);

    add_document.or(search).recover(handle_rejection)
}
