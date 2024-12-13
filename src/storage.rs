use crate::models::Document;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
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
            Ok(bincode::deserialize_from(reader)?)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(e.into()),
    }
}
