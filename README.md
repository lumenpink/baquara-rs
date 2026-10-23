# 🐗 Baquara RS (`baquara`)
> **Motor nativo de IA local, RAG semântico híbrido e servidor MCP do ecossistema Baquara.**  
> Desenvolvido em **Rust** para máxima performance na GPU GeForce GTX 1050 (4 GB VRAM) via Ollama.

---

## 🗺️ Visão Geral

O `baquara` é uma ferramenta CLI unificada e servidor MCP escrita em Rust, projetada para conectar o desenvolvedor aos seus repositórios locais e à GPU doméstica sem dependências externas pesadas:

- **Embeddings:** `nomic-embed-text` (vetores de 768 dimensões com busca SIMD).
- **Raciocínio & Código:** `qwen2.5-coder:3b` rodando 100% na GPU via CUDA.
- **Banco Vetorial:** SQLite embutido (`~/.local/share/baquara/index.db`) com indexação incremental via SHA-256.
- **Busca Híbrida:** Similaridade cosseno + relevância léxica de identificadores de código + junção de blocos contíguos de funções.
- **MCP Server:** Expõe nativamente `baquara_search` e `baquara_ask` para o Antigravity via JSON-RPC stdio.

---

## 🚀 Instalação e Compilação

Para compilar e instalar o binário em `~/.local/bin/baquara`:

```bash
cd /home/lumen/src/baquara-rs
./install.sh
```

### Compilação Estática e Otimização Soberana (GOST 1.5.0)
- **Perfil de Release Otimizado:** `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` e `strip = true`.
- **Linking Estático musl:** Suporte hermético com `target-feature=+crt-static` para distribuição `static-pie` universal sem dependências de sistema.

---

## 💻 Uso no Terminal

### 1. Fazer perguntas sobre o ecossistema (RAG + GPU)
```bash
baquara ask "Como funciona o baking de diagramas no fuxico?"
# ou diretamente:
baquara "onde fica a rota de renderização do Joaju?"
```

### 2. Busca semântica e léxica instantânea (sem chamar o LLM)
```bash
baquara search "renderJoajuDiagram" --limit 5
```

### 3. Gerador automático de mensagens de commit (Conventional Commits)
```bash
git diff | baquara commit
```

### 4. Pergunta livre para o modelo local (sem RAG)
```bash
baquara prompt "Escreva uma função em Go para calcular hash SHA-256"
```

### 5. Indexação de repositórios
```bash
# Indexar um repositório específico
baquara index /home/lumen/src/fuxico/src

# Indexar todos os repositórios em /home/lumen/src
baquara index-all

# Ver status da base de vetores
baquara status

# Limpar o banco de dados vetorial
baquara clear
```

### 6. Modo Servidor MCP (Antigravity)
```bash
baquara mcp
```

## 📄 Licença

Este projeto é software livre licenciado sob a [GNU General Public License v3.0](LICENSE) (GPL-3.0-or-later).

| Componente | Licença | Arquivo |
| :--- | :--- | :--- |
| **Código-fonte e binários** | GPL-3.0-or-later | [LICENSE](LICENSE) |
| **Logotipos e marcas** | CC-BY-ND-4.0 | [LICENSE-LOGO](LICENSE-LOGO) |
| **Mídias e documentação** | CC-BY-SA-4.0 | [LICENSE-MEDIA](LICENSE-MEDIA) |
