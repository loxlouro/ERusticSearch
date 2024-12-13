use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, BufReader};
use crate::models::Document;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub async fn save_documents(docs: &HashMap<String, Document>, path: &str) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, &docs)?;
    Ok(())
}

pub fn load_documents(path: &str) -> Result<HashMap<String, Document>> {
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match bincode::deserialize_from(reader) {
                Ok(docs) => Ok(docs),
                Err(e) => {
                    tracing::warn!("Ошибка десериализации: {}, создаем новое хранилище", e);
                    Ok(HashMap::new())
                }
            }
        }
        Err(e) => {
            tracing::info!("Файл хранилища не найден: {}, создаем новый", e);
            Ok(HashMap::new())
        }
    }
} 