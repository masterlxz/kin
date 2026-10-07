# Pendências do Projeto

> Arquivo central de pendências — **resolvidas e não resolvidas**.
> Toda pendência encontrada em qualquer arquivo do projeto deve ser registrada aqui com um ID único.
> Ao resolver uma, marcar como `✅ Resolvida` com a sessão em que foi corrigida.
>
> Última atualização: 2026-10-07 (Sessão 3)

---

## Não Resolvidas

### Decisões em Aberto

| ID | Item | Onde se originou | Prioridade |
|---|---|---|---|
| P9 | **Ordenação de commits MLS sem Delivery Service** (1.5: mensagens fora de ordem/duplicadas/de época futura já tratadas e testadas em 1:1; **continua aberto** para commits concorrentes e para grupos) — como peers resolvem commits concorrentes e como um device offline recupera commits perdidos (consequência de D2; assumir entrega at-least-once sem ordem garantida, ver D1) | D2 (Sessão 2) | Alta (Fase 1.5 / 3) |
| P10 | **Workspaces sem servidor** — (a) histórico compartilhado: MLS não dá histórico a quem entra depois, como sincronizar de peers/mailbox; (b) papéis/permissões (admin, membro) sem autoridade central, ex.: log de membership assinado; (c) custo de commits MLS em workspaces grandes; (d) onde guardar o estado do workspace | D4 (Sessão 2) | Média (Fase 5/6; molda o formato de mensagem desde a Fase 1) |
| P11 | **Solicitação de amizade sem servidor** — formato assinado pré-sessão MLS; onde entregar a solicitação com o destinatário offline (endpoint público = alvo de spam em relays/mailboxes); pesquisa por nome/identidade (depende de DHT, Fase 4); aceite que cria a sessão MLS | D5 (Sessão 2) | Média (Fase 2 para link/QR; Fase 4 para pesquisa) |
| P12 | **Vínculo Kin ↔ TruthID** — (a) como verificar a prova "chave Kin pertence à identidade TruthID" (RPC da Base vs prova off-chain assinada); (b) raiz da hierarquia de chaves no modo TruthID (wallet → delegação ao Kin); (c) como o Kin fala com o TruthID (crate, serviço local ou só formato de chave compatível) | D7 (Sessão 3) | Baixa (Fase 1 usa só o provedor standalone; resolver antes de expor o vínculo) |
| P13 | **Chave mestra sem cifra em repouso** — `StandaloneIdentity::save` grava o keypair em arquivo com permissão 0600, mas sem senha/keystore do SO. Definir proteção (senha, keyring do SO, Secure Enclave/Keystore no mobile) antes de qualquer uso além de desenvolvimento | Fase 1.2 (Sessão 3) | Média (antes de distribuir o app) |
| P16 | **Re-handshake após perda de estado** — se um lado perde o banco (ou reinstala), o outro continua com a conversa antiga: `on_hello`/`on_welcome` ignoram quem já tem conversa, o handshake não se refaz, o remetente recebe `Delivered` (ack de transporte) mas o destino só emite `Dropped`, e quem tem o estado ainda reanuncia `ConversationReady`. Precisa de um jeito de detectar conversa divergente (ex.: reset assinado / nova conversa que substitui a antiga, ligado a P9) | Fase 1.7 (Sessão 3) | Média (antes de distribuir o app; teste `peer_that_lost_its_database_is_not_rehandshaked_yet` documenta o estado atual) |
| P6 | **D6 — Privacidade de metadados** — quanto entra no MVP vs Fase 7 | Spec original (Sessão 1) | Baixa |
| P7 | **Nome definitivo** — `Kin` é provisório/"meio fixo"; pode mudar | Sessão 1 | Baixa |

---

## Resolvidas

| ID | Item | Resolvida em |
|---|---|---|
| P15 | **Ligar o Peer ID de rede à chave MLS do device** — `DeviceCertificate` v2 cobre a chave MLS (`signing_key`) e a de rede (`network_key`); `crypto` expõe `Expected { identity, peer }` em `invite`/`join` (erro `UnexpectedPeer`) e `Conversation::peer_ids`; `Chat` exige o Peer ID da conexão no KeyPackage e no Welcome e confere o rótulo salvo ao recarregar. `CryptoDevice::open` falha com `DeviceMismatch` se a chave de rede mudou. Ressalvas: troca da chave de rede invalida as conversas salvas (sem rotação ainda); commits com `Add` em grupos ainda não exigem Peer ID (P9/Fase 5); certificado v1 não é lido (apagar bancos antigos) | ✅ Sessão 3 (P15) |
| P14 | **Persistência do estado MLS** — `openmls_sqlite_storage` 0.3 + provider próprio (RustCrypto + SQLite, codec JSON), chave de device e conversas conhecidas no mesmo banco; `CryptoDevice::open`, `Conversation::load`, `Chat::open`. Ressalvas: sem cifra em repouso (segue em P13; arquivo 0600) e `CryptoDevice`/`Chat` ficam `!Send` (`rusqlite::Connection` não é `Sync`) | ✅ Sessão 3 (Fase 1.6b) |
| P8 | **Estudar o prior art antes de codar** — achados em `STUDY.md`; 4 perguntas abertas listadas lá para a fase de decisões | ✅ Sessão 2 |
| P3 | **D3 — Stack de rede**: `rust-libp2p` (decisão do usuário; ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P2 | **D2 — E2EE**: MLS via `openmls` (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P4 | **D4 — Posicionamento**: híbrido pessoal (WhatsApp) + workspaces (Slack/Discord) com threads em todo o app (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P1 | **D1 — Mailbox**: outbox local → relays Nostr (Fase 3) → peers como mailbox (Fase 4) (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
| P5 | **D5 — Anti-spam / Sybil**: contato por consentimento no modelo de amizade, com níveis de descoberta configuráveis (ver `ARCHITECTURE.md`) | ✅ Sessão 2 |
