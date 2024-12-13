use crate::core::document::Document;
use warp::Filter;

pub fn json_body() -> impl Filter<Extract = (Document,), Error = warp::Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16)
        .and(warp::body::json())
        .map(|doc: Document| doc)
}
