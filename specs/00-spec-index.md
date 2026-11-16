# Spec 00 — Índice do Spec Kit & Critérios Globais

Este diretório contém a especificação executável do **Vagnarok File Manager** para uso por engenheiros e agentes de código.

## Arquivos da Especificação
1. [01-mockup-reconciliation.md](01-mockup-reconciliation.md) — Decisões oficiais que resolvem divergências entre [DESIGN.md](../DESIGN.md), `code.html` e o mockup inicial `screen.png`.
2. [02-design-system-tokens.md](02-design-system-tokens.md) — Paletas (`ObsidianFlow` Dark e `AetherLight` Light), tipografia (`Inter`, `JetBrains Mono`), espaçamento, raios e elevação para o crate `fm-theme`.
3. [03-architecture-contracts.md](03-architecture-contracts.md) — Contratos Rust de domínio (`fm-core`), trait `VfsProvider`, estado de abas/seleção, eventos assíncronos e cancelamento.
4. [04-ui-components-and-views.md](04-ui-components-and-views.md) — Comportamento e layout de cada região da interface (Header, Sidebar, Canvas virtualizado, Inspector, QuickLook e Status Bar).
5. [05-tech-stack-prototype.md](05-tech-stack-prototype.md) — Stack tecnológica definitiva (Iced + wgpu + Rust nativo), estrutura inicial de `Cargo.toml` e roadmap do protótipo (`P0` a `P3`).

## Critérios Globais de Aceitação (Definition of Done)
* **Zero WebView / Zero C++ pesado**: Toda interface em `iced` (`wgpu` + fallback `tiny-skia`) e todo processamento de sistema em Rust nativo.
* **Responsividade**: A thread de UI permanece livre durante varredura de diretórios, geração de thumbnails, cálculo de SHA-256 e leitura Git.
* **Paridade Linux + Windows**: Qualquer funcionalidade entregue no protótipo deve compilar e respeitar semântica de caminhos em ambos os sistemas.
* **Fidelidade Visual ao Mockup Reconciliado**: Suporte imediato à alternância entre *Aether Light* (visual de `screen.png`) e *Obsidian Flow* (especificação de `DESIGN.md`).
* **Commit e Notificação Obrigatórios**: Toda entrega com mudanças no repositório deve ser finalizada após as verificações com `git commit` imediato (seguindo Conventional Commits, sem incluir artefatos temporários ou builds) e o usuário deve ser expressamente avisado na resposta sobre o hash, a mensagem e os arquivos do(s) commit(s) realizado(s).

## Fluxo Git Obrigatório
1. Após concluir e validar qualquer alteração (código, documentação, specs, assets ou CI/CD), faça o `git commit` antes de encerrar a tarefa — salvo quando o usuário pedir explicitamente para não commitar.
2. Revise `git status` para não versionar arquivos temporários ou diretórios ignorados e utilize mensagens no padrão Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, etc.).
3. Sempre informe ao usuário na resposta final os commits criados (hash curto, mensagem e resumo do escopo).
