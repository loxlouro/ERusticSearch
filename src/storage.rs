use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, BufReader};
use crate::models::Document;

pub const STORAGE_FILE: &str = "documents.db";

pub fn load_documents() -> io::Result<HashMap<String, Document>> {
    match File::open(STORAGE_FILE) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match bincode::deserialize_from(reader) {
                Ok(docs) => Ok(docs),
                Err(_) => Ok(HashMap::new())
            }
        }
        Err(_) => Ok(HashMap::new())
    }
}

pub async fn save_documents(docs: &HashMap<String, Document>) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(STORAGE_FILE)?;
    
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, &docs)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
} 