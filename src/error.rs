use std::fmt;
use warp::reject;
use anyhow::Error as AnyhowError;

#[derive(Debug)]
pub struct StorageError(pub AnyhowError);

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ошибка хранилища: {}", self.0)
    }
}

impl reject::Reject for StorageError {}

impl From<AnyhowError> for StorageError {
    fn from(err: AnyhowError) -> Self {
        StorageError(err)
    }
} 