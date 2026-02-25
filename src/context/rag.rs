use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use super::RagConfig;

#[derive(Debug, Clone)]
pub struct RagIndex {
    config: RagConfig,
    chunks: Arc<RwLock<Vec<TextChunk>>>,
    embeddings: Arc<RwLock<Vec<Vec<f32>>>>,
}

#[derive(Debug, Clone)]
struct TextChunk {
    file_path: PathBuf,
    content: String,
    start_line: usize,
}

impl RagIndex {
    pub fn new(config: RagConfig) -> Self {
        Self {
            config,
            chunks: Arc::new(RwLock::new(Vec::new())),
            embeddings: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn build_index(
        &self,
        http_client: &reqwest::Client,
        base_url: &str,
        api_key: &str,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut all_chunks = Vec::new();

        for index_path in &self.config.index_paths {
            let path = Path::new(index_path);
            if path.is_dir() {
                self.collect_files(path, &mut all_chunks)?;
            } else if path.is_file() {
                self.chunk_file(path, &mut all_chunks)?;
            }
        }

        tracing::info!(chunks = all_chunks.len(), "built RAG index");

        // Compute embeddings
        let model = &self.config.model;
        let mut all_embeddings = Vec::new();
        for chunk_batch in all_chunks.chunks(32) {
            let texts: Vec<&str> = chunk_batch.iter().map(|c| c.content.as_str()).collect();
            let embeddings =
                compute_embeddings(&texts, http_client, base_url, api_key, model).await?;
            all_embeddings.extend(embeddings);
        }

        // Validate lengths match — embedding API may return fewer results on error
        if all_chunks.len() != all_embeddings.len() {
            tracing::warn!(
                chunks = all_chunks.len(),
                embeddings = all_embeddings.len(),
                "chunk/embedding count mismatch, truncating to shorter"
            );
            let min_len = all_chunks.len().min(all_embeddings.len());
            all_chunks.truncate(min_len);
            all_embeddings.truncate(min_len);
        }

        *self.chunks.write().await = all_chunks;
        *self.embeddings.write().await = all_embeddings;

        Ok(())
    }

    pub async fn search(
        &self,
        query: &str,
        http_client: &reqwest::Client,
        base_url: &str,
        api_key: &str,
    ) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let model = &self.config.model;
        let query_embedding =
            compute_embeddings(&[query], http_client, base_url, api_key, model).await?;
        let query_vec = match query_embedding.first() {
            Some(v) => v,
            None => return Ok(Vec::new()),
        };

        let chunks = self.chunks.read().await;
        let embeddings = self.embeddings.read().await;

        if chunks.is_empty() || embeddings.is_empty() {
            return Ok(Vec::new());
        }

        // Compute cosine similarity
        let mut scores: Vec<(usize, f32)> = embeddings
            .iter()
            .enumerate()
            .map(|(i, emb)| (i, cosine_similarity(query_vec, emb)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = scores
            .into_iter()
            .take(self.config.top_k)
            .filter(|(_, score)| *score > 0.3)
            .filter_map(|(idx, _)| chunks.get(idx))
            .map(|chunk| {
                format!(
                    "// File: {}:{}\n{}",
                    chunk.file_path.display(),
                    chunk.start_line,
                    chunk.content
                )
            })
            .collect();

        Ok(top_k)
    }

    fn collect_files(&self, dir: &Path, chunks: &mut Vec<TextChunk>) -> Result<()> {
        let extensions = [
            "rs", "ts", "tsx", "js", "jsx", "java", "py", "go", "md", "toml", "yaml", "yml",
        ];

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "target"
                    || name == "dist"
                {
                    continue;
                }
                self.collect_files(&path, chunks)?;
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        self.chunk_file(&path, chunks)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn chunk_file(&self, path: &Path, chunks: &mut Vec<TextChunk>) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let chunk_lines = self.config.chunk_size / 40; // rough chars-per-line estimate
        let chunk_lines = chunk_lines.max(10);

        for (i, window) in lines.chunks(chunk_lines).enumerate() {
            let text = window.join("\n");
            if text.trim().is_empty() {
                continue;
            }
            chunks.push(TextChunk {
                file_path: path.to_path_buf(),
                content: text,
                start_line: i * chunk_lines + 1,
            });
        }

        Ok(())
    }
}

async fn compute_embeddings(
    texts: &[&str],
    http_client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<Vec<Vec<f32>>> {
    let url = format!("{}/embeddings", base_url.trim_end_matches('/'));

    let body = json!({
        "model": model,
        "input": texts,
    });

    let mut req = http_client.post(&url).json(&body);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }

    let resp: Value = req.send().await?.json().await?;

    let embeddings = resp
        .get("data")
        .and_then(|d| d.as_array())
        .map(|data| {
            data.iter()
                .filter_map(|item| {
                    item.get("embedding").and_then(|e| e.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(embeddings)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}
