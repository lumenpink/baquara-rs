use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

pub const DEFAULT_LLM: &str = "qwen2.5-coder:3b";
pub const DEFAULT_EMBED: &str = "nomic-embed-text";

#[derive(Clone, Debug)]
pub struct OllamaClient {
    pub base_url: String,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

impl OllamaClient {
    pub fn new() -> Result<Self, String> {
        let url = Self::resolve_url().ok_or_else(|| {
            "Não foi possível conectar ao Ollama (nem em localhost:11434 nem no container Docker 'ollama').".to_string()
        })?;
        Ok(Self { base_url: url })
    }

    fn resolve_url() -> Option<String> {
        // 1. Tenta conexões de host direto
        for candidate in &["http://127.0.0.1:11434", "http://localhost:11434"] {
            if ureq::get(&format!("{}/api/tags", candidate))
                .timeout(Duration::from_millis(400))
                .call()
                .is_ok()
            {
                return Some(candidate.to_string());
            }
        }

        // 2. Tenta descobrir o IP da rede interna do Docker do container ollama
        let output = Command::new("docker")
            .args([
                "inspect",
                "-f",
                "{{range .NetworkSettings.Networks}}{{.IPAddress}} {{end}}",
                "ollama",
            ])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for ip in stdout.split_whitespace() {
            let candidate = format!("http://{}:11434", ip.trim());
            if ureq::get(&format!("{}/api/tags", candidate))
                .timeout(Duration::from_millis(400))
                .call()
                .is_ok()
            {
                return Some(candidate);
            }
        }

        None
    }

    pub fn get_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        let payload = EmbedRequest {
            model: DEFAULT_EMBED,
            prompt: text,
        };

        let resp: EmbedResponse = ureq::post(&format!("{}/api/embeddings", self.base_url))
            .timeout(Duration::from_secs(30))
            .send_json(&payload)
            .map_err(|e| format!("Erro ao gerar embedding no Ollama: {}", e))?
            .into_json()
            .map_err(|e| format!("Falha ao decodificar JSON do embedding: {}", e))?;

        let mut vec = resp.embedding;
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }
        Ok(vec)
    }

    pub fn generate(&self, prompt: &str, model: Option<&str>) -> Result<String, String> {
        let m = model.unwrap_or(DEFAULT_LLM);
        let payload = GenerateRequest {
            model: m,
            prompt,
            stream: false,
        };

        let resp: GenerateResponse = ureq::post(&format!("{}/api/generate", self.base_url))
            .timeout(Duration::from_secs(180))
            .send_json(&payload)
            .map_err(|e| format!("Erro ao gerar resposta no Ollama: {}", e))?
            .into_json()
            .map_err(|e| format!("Falha ao decodificar JSON do generate: {}", e))?;

        Ok(resp.response.trim().to_string())
    }
}
