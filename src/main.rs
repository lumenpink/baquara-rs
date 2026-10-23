mod client;
mod db;
mod indexer;
mod mcp;

use clap::{Parser, Subcommand};
use client::OllamaClient;
use db::Database;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "baquara")]
#[command(author = "Lumen <lumen@baquara.local>")]
#[command(version = "0.1.0")]
#[command(about = "Motor de IA local, RAG semântico e servidor MCP do ecossistema Baquara (GPU)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Pergunta ou instrução direta (quando nenhum subcomando é passado)
    #[arg(trailing_var_arg = true)]
    query: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Faz uma pergunta técnica usando RAG e o modelo de código local
    Ask {
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
    },

    /// Busca semântica e léxica ultra-rápida no código (sem chamar o LLM)
    Search {
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
    },

    /// Gera uma mensagem de commit convencional a partir do git diff recebido via stdin
    Commit,

    /// Executa um prompt direto no LLM da GPU sem consultar o RAG
    Prompt {
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
    },

    /// Indexa incrementalmente uma pasta de código no banco vetorial
    Index {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Indexa todos os repositórios Git encontrados em /home/lumen/src
    IndexAll {
        #[arg(default_value = "/home/lumen/src")]
        path: PathBuf,
    },

    /// Exibe estatísticas da base vetorial do Baquara
    Status,

    /// Apaga o banco de dados vetorial local
    Clear,

    /// Inicia o servidor MCP stdio para integração com o Antigravity
    Mcp,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Se foi chamado diretamente com 'baquara mcp', roda sem print de terminal
    if let Some(Commands::Mcp) = &cli.command {
        let client = OllamaClient::new().map_err(|e| format!("{}", e))?;
        let db = Database::open().map_err(|e| format!("{}", e))?;
        mcp::run_mcp_server(&client, &db)?;
        return Ok(());
    }

    match &cli.command {
        Some(Commands::Ask { query }) => {
            cmd_ask(&query.join(" "))?;
        }
        Some(Commands::Search { query, limit }) => {
            cmd_search(&query.join(" "), *limit)?;
        }
        Some(Commands::Commit) => {
            cmd_commit()?;
        }
        Some(Commands::Prompt { query }) => {
            cmd_prompt(&query.join(" "))?;
        }
        Some(Commands::Index { path }) => {
            let client = OllamaClient::new()?;
            let mut db = Database::open()?;
            indexer::index_directory(path, &client, &mut db)?;
        }
        Some(Commands::IndexAll { path }) => {
            let client = OllamaClient::new()?;
            let mut db = Database::open()?;
            indexer::index_all_ecosystem(path, &client, &mut db)?;
        }
        Some(Commands::Status) => {
            cmd_status()?;
        }
        Some(Commands::Clear) => {
            let db = Database::open()?;
            db.clear()?;
            println!("✅ Base vetorial do Baquara apagada com sucesso.");
        }
        Some(Commands::Mcp) => unreachable!(),
        None => {
            if !cli.query.is_empty() {
                cmd_ask(&cli.query.join(" "))?;
            } else {
                // Se stdin não é um terminal (pipe), faz prompt automático
                if !atty_stdin() {
                    cmd_pipe()?;
                } else {
                    println!("Uso: baquara <ask|search|commit|index|index-all|status|mcp>");
                    println!("Tente: baquara --help ou baquara ask \"sua dúvida\"");
                }
            }
        }
    }

    Ok(())
}

fn atty_stdin() -> bool {
    libc_isatty()
}

fn libc_isatty() -> bool {
    // Verificação simples e portátil sem crates extras
    #[link(name = "c")]
    extern "C" {
        fn isatty(fd: i32) -> i32;
    }
    unsafe { isatty(0) != 0 }
}

fn cmd_search(query: &str, limit: usize) -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new()?;
    let db = Database::open()?;
    let q_vec = client.get_embedding(query)?;
    let hits = db.search_hybrid(query, &q_vec, limit)?;

    if hits.is_empty() {
        println!("Nenhum código encontrado. Indexe seus projetos com: baquara index <pasta>");
        return Ok(());
    }

    println!("\n🔍 Resultados da busca híbrida para '{}':", query);
    for h in hits {
        let home = std::env::var("HOME").unwrap_or_default();
        let display_path = if !home.is_empty() && h.file_path.starts_with(&home) {
            h.file_path.replace(&home, "~")
        } else {
            h.file_path.clone()
        };

        println!(
            "\n  📄 {} (Linhas {}-{}) - Score: {:.3}",
            display_path, h.start_line, h.end_line, h.score
        );
        println!("  {}", "-".repeat(60));
        for line in h.content.lines().take(10) {
            println!("    {}", line);
        }
        if h.content.lines().count() > 10 {
            println!("    ...");
        }
    }
    println!();
    Ok(())
}

fn cmd_ask(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new()?;
    let db = Database::open()?;
    let q_vec = client.get_embedding(query)?;
    let hits = db.search_hybrid(query, &q_vec, 6)?;

    if hits.is_empty() {
        println!("Nenhum código indexado ainda. Use: baquara index <pasta> ou baquara index-all");
        return Ok(());
    }

    let home = std::env::var("HOME").unwrap_or_default();
    println!("\n🔍 Trechos mais relevantes recuperados na GPU:");
    let mut context = String::new();
    for h in &hits {
        let display_path = if !home.is_empty() && h.file_path.starts_with(&home) {
            h.file_path.replace(&home, "~")
        } else {
            h.file_path.clone()
        };
        println!(
            "  • {} (L{}-{}) - Score: {:.3}",
            display_path, h.start_line, h.end_line, h.score
        );
        context.push_str(&format!(
            "--- Arquivo: {} (Linhas {}-{}) ---\n{}\n\n",
            h.file_path, h.start_line, h.end_line, h.content
        ));
    }

    let prompt = format!(
        "Você é um arquiteto e engenheiro de software sênior do ecossistema Baquara.\n\
         Use os seguintes trechos de código do projeto como referência exata para responder à dúvida do usuário.\n\
         Responda em português, de forma clara, técnica e concisa, citando os arquivos e linhas relevantes quando apropriado.\n\n\
         [TRECHOS DE CÓDIGO DO PROJETO]\n{}\n\n\
         [PERGUNTA DO USUÁRIO]\n{}",
        context, query
    );

    println!("\n🤖 Resposta da GPU (qwen2.5-coder:3b):");
    println!("{}", "-".repeat(60));
    let ans = client.generate(&prompt, None)?;
    println!("{}", ans);
    println!("{}", "-".repeat(60));
    Ok(())
}

fn cmd_commit() -> Result<(), Box<dyn std::error::Error>> {
    let mut diff = String::new();
    io::stdin().read_to_string(&mut diff)?;
    let diff = diff.trim();

    if diff.is_empty() {
        eprintln!("[baquara erro] O comando commit requer o diff via stdin. Exemplo: git diff | baquara commit");
        std::process::exit(1);
    }

    let prompt = format!(
        "Você é um assistente de desenvolvimento sênior.\n\
         Escreva uma mensagem de commit no padrão Conventional Commits (ex: feat(escopo): ..., fix(escopo): ...) concisa e direta em português, baseando-se no git diff fornecido.\n\
         Responda APENAS com a mensagem de commit (título e no máximo 2 linhas de corpo conciso se necessário), sem explicações ou blocos adicionais de markdown:\n\n\
         ```diff\n{}\n```",
        diff
    );

    let client = OllamaClient::new()?;
    let ans = client.generate(&prompt, None)?;
    println!("{}", ans.trim());
    Ok(())
}

fn cmd_prompt(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = OllamaClient::new()?;
    let ans = client.generate(query, None)?;
    println!("{}", ans);
    Ok(())
}

fn cmd_pipe() -> Result<(), Box<dyn std::error::Error>> {
    let mut content = String::new();
    io::stdin().read_to_string(&mut content)?;
    let content = content.trim();

    let prompt = format!(
        "Analise o seguinte conteúdo técnico:\n\n```\n{}\n```",
        content
    );
    let client = OllamaClient::new()?;
    let ans = client.generate(&prompt, None)?;
    println!("{}", ans);
    Ok(())
}

fn cmd_status() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open()?;
    let (files, chunks, size) = db.count_stats()?;
    let size_mb = size as f64 / (1024.0 * 1024.0);

    println!("📊 Status da IA e Base Vetorial Baquara:");
    println!("  • Arquivos indexados: {}", files);
    println!("  • Blocos de código (chunks): {}", chunks);
    println!("  • Tamanho do banco SQLite: {:.2} MB", size_mb);
    println!("  • Localização do banco: {:?}", Database::get_db_path());
    println!("  • Modelo de embeddings: {}", client::DEFAULT_EMBED);
    println!("  • Modelo de inferência: {}", client::DEFAULT_LLM);
    Ok(())
}
