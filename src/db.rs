use regex::Regex;
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub score: f32,
}

impl Database {
    pub fn get_db_path() -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/home/lumen/.local/share"))
            .join("baquara");
        let _ = fs::create_dir_all(&base);
        base.join("index.db")
    }

    pub fn open() -> Result<Self, String> {
        let path = Self::get_db_path();
        let conn = Connection::open(&path)
            .map_err(|e| format!("Falha ao abrir SQLite em {:?}: {}", path, e))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                sha256 TEXT NOT NULL,
                mtime REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB NOT NULL,
                FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_file ON chunks(file_path);
            ",
        )
        .map_err(|e| format!("Falha ao inicializar tabelas: {}", e))?;

        Ok(Self { conn })
    }

    pub fn get_file_sha(&self, path: &str) -> Option<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT sha256 FROM files WHERE path = ?")
            .ok()?;
        stmt.query_row(params![path], |row| row.get(0)).ok()
    }

    pub fn delete_file_chunks(&mut self, path: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM chunks WHERE file_path = ?", params![path])
            .map_err(|e| e.to_string())?;
        self.conn
            .execute("DELETE FROM files WHERE path = ?", params![path])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_file(&mut self, path: &str, sha: &str, mtime: f64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO files (path, sha256, mtime) VALUES (?, ?, ?)",
                params![path, sha, mtime],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_chunk(
        &mut self,
        file_path: &str,
        start_line: usize,
        end_line: usize,
        content: &str,
        embedding: &[f32],
    ) -> Result<(), String> {
        // Serialização segura e portável de floats para bytes
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for f in embedding {
            bytes.extend_from_slice(&f.to_le_bytes());
        }

        self.conn.execute(
            "INSERT INTO chunks (file_path, start_line, end_line, content, embedding) VALUES (?, ?, ?, ?, ?)",
            params![file_path, start_line as i64, end_line as i64, content, bytes],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn count_stats(&self) -> Result<(usize, usize, u64), String> {
        let n_files: usize = self
            .conn
            .query_row("SELECT count(*) FROM files", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let n_chunks: usize = self
            .conn
            .query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let size = fs::metadata(Self::get_db_path())
            .map(|m| m.len())
            .unwrap_or(0);
        Ok((n_files, n_chunks, size))
    }

    pub fn clear(&self) -> Result<(), String> {
        let path = Self::get_db_path();
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn search_hybrid(
        &self,
        query: &str,
        q_vec: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchHit>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT file_path, start_line, end_line, content, embedding FROM chunks")
            .map_err(|e| e.to_string())?;

        let re = Regex::new(r"[a-zA-Z0-9_\$]{3,}").unwrap();
        let terms: Vec<String> = re
            .find_iter(query)
            .map(|m| m.as_str().to_lowercase())
            .collect();

        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut scored_chunks = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let file_path: String = row.get(0).unwrap();
            let start_line: i64 = row.get(1).unwrap();
            let end_line: i64 = row.get(2).unwrap();
            let content: String = row.get(3).unwrap();
            let emb_bytes: Vec<u8> = row.get(4).unwrap();

            if !emb_bytes.len().is_multiple_of(4) {
                continue;
            }

            let mut vec = Vec::with_capacity(emb_bytes.len() / 4);
            for &chunk in emb_bytes.as_chunks::<4>().0 {
                vec.push(f32::from_le_bytes(chunk));
            }

            // Produto escalar (cosseno já normalizado)
            let v_sim: f32 = q_vec.iter().zip(&vec).map(|(a, b)| a * b).sum();

            // Boost léxico para identificadores exatos
            let c_lower = content.to_lowercase();
            let kw_hits = terms
                .iter()
                .filter(|t| c_lower.contains(t.as_str()))
                .count();
            let final_score = v_sim + (0.05 * (kw_hits.min(6) as f32));

            scored_chunks.push(SearchHit {
                file_path,
                start_line: start_line as usize,
                end_line: end_line as usize,
                content,
                score: final_score,
            });
        }

        // Ordenar por score decrescente
        scored_chunks.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_candidates: Vec<SearchHit> = scored_chunks.into_iter().take(top_k).collect();

        // Fusão contígua de chunks adjacentes do mesmo arquivo
        Ok(Self::merge_adjacent(top_candidates))
    }

    fn merge_adjacent(hits: Vec<SearchHit>) -> Vec<SearchHit> {
        use std::collections::BTreeMap;
        let mut by_file: BTreeMap<String, Vec<SearchHit>> = BTreeMap::new();
        for h in hits {
            by_file.entry(h.file_path.clone()).or_default().push(h);
        }

        let mut merged = Vec::new();
        for (_fpath, mut items) in by_file {
            items.sort_by_key(|x| x.start_line);
            let mut file_merged: Vec<SearchHit> = Vec::new();

            for item in items {
                if let Some(prev) = file_merged.last_mut() {
                    if item.start_line <= prev.end_line + 15 {
                        // Mesclar conteúdo
                        let _prev_lines: Vec<&str> = prev.content.lines().collect();
                        let item_lines: Vec<&str> = item.content.lines().collect();
                        let overlap = if prev.end_line >= item.start_line {
                            (prev.end_line - item.start_line + 1).min(item_lines.len())
                        } else {
                            0
                        };

                        let mut new_content = prev.content.clone();
                        new_content.push('\n');
                        new_content.push_str(&item_lines[overlap..].join("\n"));

                        prev.content = new_content;
                        prev.end_line = prev.end_line.max(item.end_line);
                        prev.score = prev.score.max(item.score);
                        continue;
                    }
                }
                file_merged.push(item);
            }
            merged.extend(file_merged);
        }

        merged.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        merged
    }
}
