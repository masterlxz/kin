# Diretriz de código (IMPORTANTE — sempre seguir)

**Todo código novo deve ser escrito em inglês — sem exceção.**
- Strings visíveis ao usuário (UI, mensagens de erro, labels, placeholders): inglês
- Nomes de variáveis, funções, classes, arquivos: inglês
- Comentários no código: podem ficar em português (não são visíveis ao usuário e facilitam o aprendizado)
- Esta regra vale para todos os arquivos: `.rs`, `.ts`, `.tsx`, `.dart`

**I18n (múltiplos idiomas) está planejado para uma fase futura:**
Hoje o app é 100% inglês. Quando houver demanda, a estratégia é extrair todas as strings visíveis para arquivos de tradução.

---

# Diretriz de ensino (IMPORTANTE — ler antes de cada sessão)

O usuário é um desenvolvedor experiente (Rust, TypeScript, React, Python, Ruby, Solidity). O objetivo do projeto é construir um sistema funcional e bem arquitetado, não aprender conceitos básicos.

**Regras para o Claude:**
- Explicar decisões de arquitetura antes de implementar
- Oferecer opções com trade-offs quando houver decisão em aberto
- Código limpo, bem tipado, testado
- Manter consistência com a stack já usada nos outros projetos (TruthID, Warden, Anchor)

---

# Diretrizes específicas do Kin (vindas da spec original)

- A spec é **referência, não plano de implementação fechado**. As decisões em aberto (D1–D6 em
  `ARCHITECTURE.md`) precisam ser levantadas com opções e trade-offs **antes** de gerar código, e o
  usuário decide.
- Trabalhar **por fases** (`PHASE.md`). Cada fase precisa ser testável isoladamente antes da próxima.
- **Não introduzir blockchain, token ou consenso global** em nenhuma fase anterior à Fase 7.
- **Preferir bibliotecas maduras a protocolos próprios** (libp2p, MLS, WebRTC). Protocolo próprio só
  onde não houver alternativa.
- **Escopo**: é fácil virar "reinventar a internet". Seguir as fases e respeitar o Definition of Done
  (`PHASE.md`). Ao chegar nele, parar de adicionar infraestrutura e avaliar.
