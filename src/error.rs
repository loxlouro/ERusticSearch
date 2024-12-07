use std::fmt;
use warp::reject;

#[derive(Debug)]
pub struct StorageError(pub std::io::Error);

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ошибка хранилища: {}", self.0)
    }
}

impl reject::Reject for StorageError {} 