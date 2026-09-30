use crate::client::OllamaClient;
use crate::db::Database;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub const SUPPORTED_EXTS: &[&str] = &[
    "go", "rs", "php", "py", "js", "ts", "jsx", "tsx",
    "sh", "bash", "zsh", "md", "html", "css", "sql",
    "json", "yml", "yaml", "toml", "c", "h", "cpp"
];

pub const IGNORE_DIRS: &[&str] = &[
    ".git", "vendor", "node_modules", "target", "dist", "build",
    ".cache", "tmp", "__pycache__", ".venv", "venv", ".idea", ".vscode"
];

pub fn chunk_text(content: &str, max_lines: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut i = 0;
    let total = lines.len();

    while i < total {
        let end = (i + max_lines).min(total);
        let chunk_content = lines[i..end].join("\n");
        if !chunk_content.trim().is_empty() {
            chunks.push((i + 1, end, chunk_content));
        }
        if end == total {
            break;
        }
        i += max_lines.saturating_sub(overlap);
    }

    chunks
}

pub fn index_directory(target_path: &Path, client: &OllamaClient, db: &mut Database) -> Result<(usize, usize), String> {
    let target_path = target_path.canonicalize().map_err(|e| format!("Caminho inválido: {}", e))?;
    let exts: HashSet<&str> = SUPPORTED_EXTS.iter().copied().collect();

    println!("📁 Analisando arquivos em {:?}...", target_path);

    let mut files_to_process = Vec::new();
    for entry in WalkDir::new(&target_path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.file_type().is_dir() && (IGNORE_DIRS.contains(&name.as_ref()) || name.starts_with('.')) {
                return false;
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|x| x.to_str()) {
                if exts.contains(ext) {
                    files_to_process.push(entry.path().to_path_buf());
                }
            }
        }
    }

    let mut indexed = 0;
    let mut skipped = 0;

    for file_path in &files_to_process {
        let path_str = file_path.to_string_lossy().to_string();
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        if let Some(existing_hash) = db.get_file_sha(&path_str) {
            if existing_hash == hash {
                skipped += 1;
                continue;
            }
        }

        let mtime = fs::metadata(file_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        let _ = db.delete_file_chunks(&path_str);
        db.insert_file(&path_str, &hash, mtime)?;

        let chunks = chunk_text(&content, 60, 15);
        for (sline, eline, chunk_str) in chunks {
            let header = format!("// File: {} (Lines {}-{})\n", path_str, sline, eline);
            let embed_input = format!("{}{}", header, chunk_str);
            let vec = client.get_embedding(&embed_input)?;
            db.insert_chunk(&path_str, sline, eline, &chunk_str, &vec)?;
        }

        indexed += 1;
        if indexed % 10 == 0 {
            println!("  ⚡ Indexados {} arquivos...", indexed);
        }
    }

    println!("✅ Indexação em {:?} concluída: {} novos/atualizados, {} inalterados.", target_path, indexed, skipped);
    Ok((indexed, skipped))
}

pub fn index_all_ecosystem(base_path: &Path, client: &OllamaClient, db: &mut Database) -> Result<(), String> {
    let entries = fs::read_dir(base_path).map_err(|e| e.to_string())?;
    let mut repos = Vec::new();

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() && p.join(".git").exists() {
            repos.push(p);
        }
    }

    repos.sort();
    println!("🌐 Encontrados {} repositórios do ecossistema Baquará em {:?}:", repos.len(), base_path);
    for r in &repos {
        println!("  • {}", r.file_name().unwrap_or_default().to_string_lossy());
    }
    println!();

    for r in repos {
        let _ = index_directory(&r, client, db);
    }

    Ok(())
}
