use std::collections::HashMap;
use std::sync::Arc;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, BufReader};
use tokio::sync::RwLock;
use warp::Filter;
use serde::{Serialize, Deserialize};
use serde_json;

const STORAGE_FILE: &str = "documents.db";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Document {
    id: String,
    content: String,
    metadata: HashMap<String, String>,
}

#[derive(Clone)]
struct SearchEngine {
    documents: Arc<RwLock<HashMap<String, Document>>>,
}

impl SearchEngine {
    fn new() -> Self {
        let documents = match Self::load_documents() {
            Ok(docs) => docs,
            Err(e) => {
                eprintln!("Ошибка загрузки документов: {}", e);
                HashMap::new()
            }
        };
        
        SearchEngine {
            documents: Arc::new(RwLock::new(documents)),
        }
    }

    fn load_documents() -> io::Result<HashMap<String, Document>> {
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

    async fn save_documents(&self) -> io::Result<()> {
        let docs = self.documents.read().await;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(STORAGE_FILE)?;
        
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &*docs)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

    async fn add_document(&self, doc: Document) -> io::Result<()> {
        let mut docs = self.documents.write().await;
        docs.insert(doc.id.clone(), doc);
        drop(docs); // Освобождаем блокировку перед сохранением
        self.save_documents().await
    }

    async fn search(&self, query: &str) -> Vec<Document> {
        let docs = self.documents.read().await;
        docs.values()
            .filter(|doc| doc.content.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect()
    }
}

#[tokio::main]
async fn main() {
    let search_engine = SearchEngine::new();
    let search_engine = Arc::new(search_engine);

    // Роуты для API
    let search_engine_filter = warp::any().map(move || search_engine.clone());

    // POST /document - добавить документ
    let add_document = warp::post()
        .and(warp::path("document"))
        .and(warp::body::json())
        .and(search_engine_filter.clone())
        .and_then(handle_add_document);

    // GET /search?q=query - поиск документов
    let search = warp::get()
        .and(warp::path("search"))
        .and(warp::query::<HashMap<String, String>>())
        .and(search_engine_filter.clone())
        .and_then(handle_search);

    let routes = add_document.or(search).recover(handle_rejection);

    println!("Сервер запущен на http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_add_document(
    document: Document,
    search_engine: Arc<SearchEngine>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Получен документ: {:?}", document);
    match search_engine.add_document(document).await {
        Ok(_) => Ok(warp::reply::json(&"Документ успешно добавлен")),
        Err(e) => Err(warp::reject::custom(StorageError(e)))
    }
}

async fn handle_search(
    query: HashMap<String, String>,
    search_engine: Arc<SearchEngine>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let q = query.get("q").cloned().unwrap_or_default();
    let results = search_engine.search(&q).await;
    Ok(warp::reply::json(&results))
}

#[derive(Debug)]
struct StorageError(io::Error);
impl warp::reject::Reject for StorageError {}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let message = if err.is_not_found() {
        "Путь не найден"
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        println!("Ошибка десериализации: {:?}", e);
        "Неверный формат JSON"
    } else if let Some(e) = err.find::<StorageError>() {
        println!("Ошибка сохранения: {:?}", e);
        "Ошибка сохранения документа"
    } else {
        "Внутренняя ошибка сервера"
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "error": message
        })),
        warp::http::StatusCode::BAD_REQUEST,
    ))
}
