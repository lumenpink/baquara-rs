use crate::client::OllamaClient;
use crate::db::Database;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse<'a> {
    jsonrpc: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

pub fn run_mcp_server(client: &OllamaClient, db: &Database) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
            handle_request(&req, client, db, &mut writer)?;
        }

        line.clear();
    }

    Ok(())
}

fn handle_request(
    req: &JsonRpcRequest,
    client: &OllamaClient,
    db: &Database,
    writer: &mut impl Write,
) -> io::Result<()> {
    match req.method.as_str() {
        "initialize" => {
            let res = json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "baquara",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {}
                }
            });
            send_response(req.id.clone(), Some(res), None, writer)?;
        }
        "notifications/initialized" => {
            // Apenas confirmação do cliente, não requer resposta
        }
        "tools/list" => {
            let res = json!({
                "tools": [
                    {
                        "name": "baquara_search",
                        "description": "Busca semântica e léxica ultra-rápida no código-fonte indexado do ecossistema Baquara (GPU). Retorna arquivos, números de linhas e trechos exatos de código.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Termo, função, conceito ou pergunta para buscar no código"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Número máximo de trechos a retornar (padrão: 5)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "baquara_ask",
                        "description": "Consulta o RAG local do ecossistema Baquara. Busca os trechos de código mais relevantes e gera uma resposta técnica sintetizada pelo modelo local qwen2.5-coder:3b na GPU.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Dúvida ou pergunta arquitetural sobre o código do ecossistema"
                                }
                            },
                            "required": ["query"]
                        }
                    }
                ]
            });
            send_response(req.id.clone(), Some(res), None, writer)?;
        }
        "tools/call" => {
            let params = req.params.as_ref().and_then(|p| p.as_object());
            let name = params
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let args = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or(json!({}));

            let tool_res = match name {
                "baquara_search" => {
                    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(5) as usize;

                    match client.get_embedding(query) {
                        Ok(q_vec) => match db.search_hybrid(query, &q_vec, limit) {
                            Ok(hits) => {
                                let mut text = format!(
                                    "Resultados da busca no ecossistema Baquara para: '{}'\n\n",
                                    query
                                );
                                for h in hits {
                                    text.push_str(&format!(
                                        "--- Arquivo: {} (Linhas {}-{}, Score: {:.3}) ---\n{}\n\n",
                                        h.file_path, h.start_line, h.end_line, h.score, h.content
                                    ));
                                }
                                json!({ "content": [{ "type": "text", "text": text }] })
                            }
                            Err(e) => {
                                json!({ "isError": true, "content": [{ "type": "text", "text": format!("Erro no banco de dados: {}", e) }] })
                            }
                        },
                        Err(e) => {
                            json!({ "isError": true, "content": [{ "type": "text", "text": format!("Erro ao gerar embedding: {}", e) }] })
                        }
                    }
                }
                "baquara_ask" => {
                    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                    match client.get_embedding(query) {
                        Ok(q_vec) => match db.search_hybrid(query, &q_vec, 6) {
                            Ok(hits) => {
                                let mut context = String::new();
                                for h in &hits {
                                    context.push_str(&format!(
                                        "--- Arquivo: {} (Linhas {}-{}) ---\n{}\n\n",
                                        h.file_path, h.start_line, h.end_line, h.content
                                    ));
                                }

                                let prompt = format!(
                                    "Você é um arquiteto e engenheiro de software sênior do ecossistema Baquara.\n\
                                     Use os seguintes trechos de código reais recuperados como base para responder à pergunta.\n\
                                     Responda em português, de forma clara, técnica e concisa, citando os arquivos e linhas relevantes.\n\n\
                                     [TRECHOS DE CÓDIGO]\n{}\n\n\
                                     [PERGUNTA]\n{}",
                                    context, query
                                );

                                match client.generate(&prompt, None) {
                                    Ok(ans) => {
                                        json!({ "content": [{ "type": "text", "text": ans }] })
                                    }
                                    Err(e) => {
                                        json!({ "isError": true, "content": [{ "type": "text", "text": format!("Erro na inferência: {}", e) }] })
                                    }
                                }
                            }
                            Err(e) => {
                                json!({ "isError": true, "content": [{ "type": "text", "text": format!("Erro na busca: {}", e) }] })
                            }
                        },
                        Err(e) => {
                            json!({ "isError": true, "content": [{ "type": "text", "text": format!("Erro de embedding: {}", e) }] })
                        }
                    }
                }
                _ => {
                    json!({ "isError": true, "content": [{ "type": "text", "text": format!("Ferramenta desconhecida: {}", name) }] })
                }
            };

            send_response(req.id.clone(), Some(tool_res), None, writer)?;
        }
        _ => {
            let err = json!({
                "code": -32601,
                "message": format!("Método não suportado: {}", req.method)
            });
            send_response(req.id.clone(), None, Some(err), writer)?;
        }
    }

    Ok(())
}

fn send_response(
    id: Option<Value>,
    result: Option<Value>,
    error: Option<Value>,
    writer: &mut impl Write,
) -> io::Result<()> {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result,
        error,
    };
    let json_bytes = serde_json::to_vec(&resp)?;
    writer.write_all(&json_bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
