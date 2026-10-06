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
| P1 | **D1 — Fallback/mailbox do dia zero** — como entregar mensagem offline com só 2 usuários sem servidor próprio. Hipótese: outbox local + relays Nostr como fallback, circuit relay v2 só para hole punching. Detalhe em `ARCHITECTURE.md` | Spec original (Sessão 1) | Alta (molda a abstração de transporte desde a Fase 1) |
| P2 | **D2 — E2EE** — MLS (`openmls`) vs Double Ratchet + sender keys. Hipótese: MLS para tudo | Spec original (Sessão 1) | Alta (bloqueia Fase 1) |
| P3 | **D3 — Stack de rede** — `rust-libp2p` vs `iroh` | Spec original (Sessão 1) | Alta (bloqueia Fase 1) |
| P4 | **D4 — Posicionamento do produto** — WhatsApp-like vs Slack/Discord-like vs broadcast 1→N | Spec original (Sessão 1) | Média |
| P5 | **D5 — Anti-spam / Sybil** — rate limit, PoW, convites/trust graph, reputação TruthID, depósito | Spec original (Sessão 1) | Média (relevante a partir da Fase 4) |
| P6 | **D6 — Privacidade de metadados** — quanto entra no MVP vs Fase 7 | Spec original (Sessão 1) | Baixa |
| P7 | **Nome definitivo** — `Kin` é provisório/"meio fixo"; pode mudar | Sessão 1 | Baixa |

---

## Resolvidas

| ID | Item | Resolvida em |
|---|---|---|
| P8 | **Estudar o prior art antes de codar** — achados em `STUDY.md`; 4 perguntas abertas listadas lá para a fase de decisões | ✅ Sessão 2 |
