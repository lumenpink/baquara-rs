# Especificação 03: Otimização de Tamanho Binário e Linking Estático musl

- **ID do Ticket:** #3
- **Status:** Aprovado
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0 (Rust)

## 1. Contexto & Motivação
O Baquara (`baquara`) é o motor de IA local, busca semântica (RAG) e servidor MCP do ecossistema.
Para garantir eficiência máxima de runtime e compilação estática universal em qualquer ambiente Linux:
1. Declara-se o perfil `[profile.release]` com `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` e `strip = true`.
2. Configura-se `.cargo/config.toml` forçando o linking estático da C runtime (`target-feature=+crt-static`) para targets musl.
3. Adiciona-se anatomia de raiz obrigatória (`.editorconfig`) e workflow canônico de CI (`.forgejo/workflows/ci.yml`).

## 2. Requisitos Técnicos
- `[profile.release]` configurado com `opt-level = 3` no `Cargo.toml`.
- `.cargo/config.toml` contendo `rustflags = ["-C", "target-feature=+crt-static"]` para `x86_64-unknown-linux-musl` e `aarch64-unknown-linux-musl`.
- Presença de `.editorconfig` e `.forgejo/workflows/ci.yml`.

## 3. Critérios de Aceitação
- Teste automatizado validando as diretivas em `Cargo.toml` e `.cargo/config.toml`.
- Executáveis compilados sem símbolos de depuração (`stripped`) e estáticos.
