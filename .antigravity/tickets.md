# Active Tickets - baquara-rs
> Synced at: 2026-10-23 02:01:10 UTC
> Ceiling: 10240 bytes (strictly uncut tickets)

## [#3] fix(build): otimizar tamanho de binario e habilitar linking estatico musl
- **Status**: `open`
- **Labels**: none
- **Link**: https://git.2lp.in/lumen/baquara-rs/issues/3

### Description
1. Adicionar perfil [profile.release] otimizado para reducao drastica de tamanho de binario (opt-level = 3, lto = 'fat', codegen-units = 1, panic = 'abort', strip = true).
2. Adicionar configuracao de linking estatico (.cargo/config.toml) com -C target-feature=+crt-static para x86_64 e aarch64 musl.
3. Adicionar .editorconfig e workflow ci.yml com estagios de test e build-release separados.

---
## [#2] feat(baquara-rs): atualizacao automatica xdg e publicacao na banca
- **Status**: `open`
- **Labels**: none
- **Link**: https://git.2lp.in/lumen/baquara-rs/issues/2

### Description
Adequar a esteira de CI/CD do repositório para compilar exclusivamente no runner soberano e publicar os artefatos diretamente na Banca (https://banca.2lp.in/downloads/baquara-rs/), mantendo retenção de 5 versões estáveis e 10 nightlies via Cloudflare R2, sem hospedar binários no Codeberg.

Implementar na CLI a funcionalidade de auto-atualização atômica (baquara-rs update e baquara-rs check-update) com estrita conformidade com a especificação OpenDesktop (XDG Base Directory) e o Perfil Lumen da norma GOST:

1. Configuração do usuário em $XDG_CONFIG_HOME/baquara-rs/config.toml (fallback ~/.config/baquara-rs/config.toml).
2. Persistência de cache e estado de verificação em $XDG_STATE_HOME/baquara-rs/update_check.json (fallback ~/.local/state/baquara-rs/update_check.json).
3. Suporte no config.toml a:
   - channel = "stable" | "nightly"
   - pinned = true | false
   - pin = "<versao>"
   - auto_check = true | false
   - check_interval_hours = 24
   - auto_update = false (desabilitado por padrão; quando habilitado e não-pinado, aplica a atualização atômica para o próximo arranque)
4. Verificação periódica não-bloqueante no startup de comandos normais da CLI, com emissão de alerta discreto exclusivamente em stderr caso haja versão mais recente no canal (desde que não esteja pinado).
5. Subcomandos de configuração:
   - baquara-rs config show / baquara-rs config get
   - baquara-rs config pin [<version>]
   - baquara-rs config unpin
   - baquara-rs config set-auto-check <true|false>
   - baquara-rs config set-channel <stable|nightly>
   - baquara-rs config set-auto-update <true|false>
6. Atualização atômica com arquivo temporário no mesmo sistema de arquivos, permissão de execução e substituição não-destrutiva em ~/.local/bin/baquara-rs com backup e rollback automático em caso de erro.

Expected Behavior & Acceptance Criteria:
- Pipeline CI/CD publicando na Banca (https://banca.2lp.in/downloads/baquara-rs/) com retenção 5 stable / 10 nightly.
- Remoção de publicação de binários pesados no Codeberg Releases.
- Configuração em $XDG_CONFIG_HOME/baquara-rs/config.toml e cache em $XDG_STATE_HOME/baquara-rs/update_check.json.
- Respeito à ordem de precedência: flag --channel > env BAQUARA_RS_CHANNEL > config.toml > stable.
- Respeito a travamento de versão (pinned = true ou pin), bloqueando updates sem --force e silenciando alertas no startup.
- Checagem rápida não-bloqueante no startup com alerta em stderr.
- Mecanismo de auto_update opcional/configurável para substituição transparente em segundo plano.
- Subcomandos baquara-rs update, baquara-rs check-update e baquara-rs config ... funcionais.
- Atualização atômica do binário em ~/.local/bin/baquara-rs com rollback.

Tasks / DD-TDD Checklist (9 Fases GOST):
- Fase 1 - Ticket: Issue formalizada e escopo delimitado
- Fase 2 - Branch: Branch feat/<id>-updater-opendesktop-xdg criada a partir de main
- Fase 3 - Doc/Spec: Contrato preliminar e especificação (docs/spec-<id>-updater.md)
- Fase 4 - Red: Teste automatizado falho comprovando parsing XDG, channels, auto_update e pinning
- Fase 5 - Green: Implementação mínima funcional com teste passando
- Fase 6 - Refactor: Código limpo, idiomático e com Zero Warnings
- Fase 7 - Mutation: Mutantes críticos eliminados com asserções robustas
- Fase 8 - Quality Gate: Cobertura >= 90%, linter/clippy e formatação validados
- Fase 9 - Close: Changelog e documentação atualizados; commit com (closes #)

---
*Migrado do Codeberg (#issue-2 por @lumenpink)*

---
