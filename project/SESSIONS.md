# Log de Sessões

> **Nota**: Este log foi criado junto com o projeto. As sessões serão registradas aqui conforme o trabalho avança.
>
> Última atualização: 2026-10-06 (Sessão 2)

---

### 2026-09-27 — Sessão 1

- **Objetivo**: registrar o projeto a partir da spec `SPEC-rede-comunicacao-p2p.md` (rede de
  comunicação P2P descentralizada, mensageiro como primeira aplicação).
- **Decisões do usuário**:
  - nome **Kin** ("meio fixo, mas podemos mudar depois"); "TruthNet" já tinha sido rejeitado;
  - estrutura `project/` igual à dos outros projetos do workspace (TruthID, Warden, Anchor);
  - repositório público no GitHub (`masterlxz/kin`), como os outros.
- **Feito**:
  - conteúdo da spec distribuído em `project/` (`CONTEXT.md` = visão/PRD, `PHASE.md` = fases,
    `ARCHITECTURE.md` = decisões tomadas + D1–D6 em aberto, `ROADMAP.md` = prior art/riscos/blockchain,
    `PENDING.md` = P1–P8);
  - adicionada uma **Fase 0** (estudo de prior art + decisões) antes da Fase 1;
  - spec original removida (o conteúdo vive em `project/`);
  - README, LICENSE (MIT), `.gitignore`, primeiro commit e push.
- **Próximo passo**: nenhum código ainda. O projeto espera na fila; quando começar, Fase 0.

---

### 2026-10-06 — Sessão 2

- **Objetivo**: começar aos poucos; concluir **todo o estudo** (Fase 0.1) antes de tomar qualquer decisão.
- **Decisões do usuário**: estudar tudo primeiro, depois decidir (D1–D6 seguem em aberto).
- **Feito**:
  - comparativo preliminar `rust-libp2p` vs `iroh` (D3), em conversa: iroh chegou à 1.0 e
    resolve hole punching + relay de fábrica; libp2p leva vantagem em DHT/gossip (Fase 4);
    **nenhuma decisão tomada**;
  - estudo dos 8 itens de prior art registrado em `STUDY.md` (Briar, Berty/Wesh, Jami, Session,
    SimpleX, Nostr, Matrix P2P/Pinecone, libp2p), com síntese por decisão e 4 perguntas abertas;
  - achado de maior peso: medição do DCUtR em escala (IMC '26) dá ~70% de sucesso de hole punching,
    ou seja, ~30% dos pares precisam de relay que carregue tráfego;
  - etapa 0.1 marcada como concluída; P8 resolvida.
- **Decisão D3** (mesma sessão): usuário escolheu **`rust-libp2p`** (assistente recomendava iroh).
  Custos e mitigações em `ARCHITECTURE.md`; etapa 0.2 e P3 resolvidas.
- **Decisão D2** (mesma sessão): usuário escolheu **MLS via `openmls`** (recomendação do assistente).
  Risco principal: ordenação de commits sem Delivery Service, registrado como P9. Etapa 0.3 e P2
  resolvidas.
- **Decisão D4** (mesma sessão): usuário rejeitou as opções puras e definiu um **híbrido**: pessoal
  estilo WhatsApp + workspaces estilo Slack/Discord, com **threads em todo o app**. Threads entram no
  formato de mensagem desde a Fase 1; workspaces na Fase 5/6. Pendência P10 aberta (histórico,
  papéis, custo MLS sem servidor). Etapa 0.4 e P4 resolvidas.
- **Decisão D1** (mesma sessão): usuário escolheu **outbox local → relays Nostr (Fase 3) → peers como
  mailbox (Fase 4)** (recomendação do assistente). P9 passa a assumir entrega at-least-once sem ordem.
  Etapa 0.5 e P1 resolvidas. **Fase 0 concluída** (só D6 segue aberta, de prioridade baixa).
- **Decisão D5** (mesma sessão): usuário preferiu **contato por consentimento, como amizade**:
  níveis de descoberta configuráveis (Público / só por link / Fechado); desconhecido só pode enviar
  solicitação de amizade, que cai numa caixa própria, não exige resposta e não avisa quem enviou;
  grupos/workspaces exigem convite e aceite. Rate limit por identidade como camada mínima; PoW/depósito
  descartados. P11 aberta (solicitação sem servidor). P5 resolvida.
- **Ressalva**: parte do `STUDY.md` vem de conhecimento prévio do modelo (marcada no texto) e precisa
  ser verificada na documentação oficial antes de virar premissa.
- **Próximo passo**: Fase 1.1 (setup do workspace Rust com os módulos identity, transport, overlay,
  crypto e chat). D6 (metadados) pode esperar até a Fase 4. Ainda sem código.
