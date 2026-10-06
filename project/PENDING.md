# Pendências do Projeto

> Arquivo central de pendências — **resolvidas e não resolvidas**.
> Toda pendência encontrada em qualquer arquivo do projeto deve ser registrada aqui com um ID único.
> Ao resolver uma, marcar como `✅ Resolvida` com a sessão em que foi corrigida.
>
> Última atualização: 2026-10-06 (Sessão 2)

---

## Não Resolvidas

### Decisões em Aberto

| ID | Item | Onde se originou | Prioridade |
|---|---|---|---|
| P9 | **Ordenação de commits MLS sem Delivery Service** — como peers resolvem commits concorrentes e como um device offline recupera commits perdidos (consequência de D2; assumir entrega at-least-once sem ordem garantida, ver D1) | D2 (Sessão 2) | Alta (Fase 1.5 / 3) |
| P10 | **Workspaces sem servidor** — (a) histórico compartilhado: MLS não dá histórico a quem entra depois, como sincronizar de peers/mailbox; (b) papéis/permissões (admin, membro) sem autoridade central, ex.: log de membership assinado; (c) custo de commits MLS em workspaces grandes; (d) onde guardar o estado do workspace | D4 (Sessão 2) | Média (Fase 5/6; molda o formato de mensagem desde a Fase 1) |
| P11 | **Solicitação de amizade sem servidor** — formato assinado pré-sessão MLS; onde entregar a solicitação com o destinatário offline (endpoint público = alvo de spam em relays/mailboxes); pesquisa por nome/identidade (depende de DHT, Fase 4); aceite que cria a sessão MLS | D5 (Sessão 2) | Média (Fase 2 para link/QR; Fase 4 para pesquisa) |
| P6 | **D6 — Privacidade de metadados** — quanto entra no MVP vs Fase 7 | Spec original (Sessão 1) | Baixa |
| P7 | **Nome definitivo** — `Kin` é provisório/"meio fixo"; pode mudar | Sessão 1 | Baixa |

---

## Resolvidas

| ID | Item | Resolvida em |
|---|---|---|
| P8 | **Estudar o prior art antes de codar** — achados em `STUDY.md`; 4 perguntas abertas listadas lá para a fase de decisões | ✅ Sessão 2 |
| P3 | **D3 — Stack de rede**: `rust-libp2p` (decisão do usuário; ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P2 | **D2 — E2EE**: MLS via `openmls` (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P4 | **D4 — Posicionamento**: híbrido pessoal (WhatsApp) + workspaces (Slack/Discord) com threads em todo o app (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P1 | **D1 — Mailbox**: outbox local → relays Nostr (Fase 3) → peers como mailbox (Fase 4) (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P5 | **D5 — Anti-spam / Sybil**: contato por consentimento no modelo de amizade, com níveis de descoberta configuráveis (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
