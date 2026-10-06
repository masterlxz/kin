# Fases Detalhadas — Planejamento Global

> **Nota**: Este é o planejamento inicial do projeto, criado a partir da spec original (Sessão 1).
> As etapas de cada fase serão detalhadas e ajustadas conforme as decisões D1–D6 (`ARCHITECTURE.md`)
> forem fechadas. Cada fase precisa ser testável isoladamente antes da próxima.
> **Nenhuma blockchain, token ou consenso global antes da Fase 7.**

---

### Fase 0 — Pré-projeto: estudo e decisões

**Objetivo**: Fechar as decisões que bloqueiam a Fase 1 antes de escrever código.

**Etapas**:
- [x] 0.1 — Estudar o prior art (`ROADMAP.md`), respondendo para cada um: *por que ele fez a escolha
  que fez para mensagens offline e NAT?* — feito na Sessão 2, ver `STUDY.md`
- [ ] 0.2 — Decidir D3 (stack: `rust-libp2p` vs `iroh`)
- [ ] 0.3 — Decidir D2 (E2EE: MLS vs Double Ratchet)
- [ ] 0.4 — Decidir D4 (posicionamento do produto)
- [ ] 0.5 — Decidir D1 (fallback/mailbox do dia zero) — necessário só a partir da Fase 3, mas molda a
  abstração de transporte desde a Fase 1

---

### Fase 1 — Identidade local + conexão direta + chat 1:1 E2EE

**Objetivo**: Fabio e um amigo conversam na mesma rede e pela internet quando há conectividade direta.

**Etapas** (preliminares):
- [ ] 1.1 — Setup do núcleo Rust + estrutura de módulos (identity, transport, overlay, crypto, chat)
- [ ] 1.2 — Identidade local (modo local do TruthID) + chave de device + Peer ID derivado
- [ ] 1.3 — Descoberta em LAN via mDNS
- [ ] 1.4 — Conexão direta pela internet (IP alcançável)
- [ ] 1.5 — E2EE 1:1 (conforme D2)
- [ ] 1.6 — Chat 1:1 mínimo (desktop primeiro)
- [ ] 1.7 — Testes de integração com dois nós

**Critério de pronto**: Fabio e amigo conversam na mesma rede e pela internet quando há conectividade
direta.

---

### Fase 2 — Hole punching + relay + convite por QR/link

**Objetivo**: Conversar atrás de CGNAT sem nenhum dos dois subir servidor.

**Etapas** (preliminares):
- [ ] 2.1 — Hole punching (ex.: DCUtR + AutoNAT)
- [ ] 2.2 — Relay público/de peer como fallback (circuit relay v2 ou equivalente)
- [ ] 2.3 — Lista de relays candidatos + medição/ranking local
- [ ] 2.4 — Convite por QR/link (endereços + chave pública, padrão do pareamento QR do TruthID)

**Critério de pronto**: conversam atrás de CGNAT sem nenhum dos dois subir servidor.

---

### Fase 3 — Store-and-forward (mailbox) + multi-device

**Objetivo**: Mensagem chega mesmo com o destinatário offline no envio.

**Etapas** (preliminares):
- [ ] 3.1 — Abstração de transporte de mailbox (trocar o backend sem mexer no resto)
- [ ] 3.2 — Outbox local (entrega na janela simultânea)
- [ ] 3.3 — Backend de mailbox de fallback (conforme D1 — hipótese: relays Nostr)
- [ ] 3.4 — Replicação em N mailboxes com TTL
- [ ] 3.5 — Multi-device (vários devices por identidade, sincronização)

**Critério de pronto**: mensagem chega mesmo com destinatário offline no envio.

---

### Fase 4 — Overlay: gossip + DHT + app-relay com limites

**Objetivo**: A rede se sustenta com 10–50 peers encaminhando tráfego entre si.

**Etapas** (preliminares):
- [ ] 4.1 — `ANNOUNCE` assinado + gossip
- [ ] 4.2 — DHT (Kademlia) para `peer_id → onde encontrar`
- [ ] 4.3 — App como relay/mailbox com limites configuráveis (`CONTEXT.md`, "App como relay")
- [ ] 4.4 — Desktop como daemon (relay natural da rede)

**Critério de pronto**: 10–50 peers; mensagem encaminhada por peers intermediários.

---

### Fase 5 — Grupos/canais, arquivos, presença, notificações

**Objetivo**: Uso diário real por um grupo pequeno.

**Etapas** (preliminares):
- [ ] 5.1 — Grupos/canais (conforme D2/D4)
- [ ] 5.2 — Arquivos (P2P direto ou chunks por relay, com limite de tamanho em app-relay)
- [ ] 5.3 — Presença
- [ ] 5.4 — Respostas/reactions
- [ ] 5.5 — Notificações
- [ ] 5.6 — App mobile (Flutter via FFI)

**Critério de pronto**: uso diário real por um grupo pequeno.

---

### Fase 6 — Chamadas WebRTC + TURN opt-in; relay dedicado empacotado

**Objetivo**: Pequena empresa usando com a própria infra de mídia.

**Etapas** (preliminares):
- [ ] 6.1 — Chamadas 1:1 via WebRTC (STUN público)
- [ ] 6.2 — TURN opt-in configurável
- [ ] 6.3 — Chamadas em grupo pequeno (mesh)
- [ ] 6.4 — Relay dedicado empacotado para empresas/entusiastas

**Critério de pronto**: pequena empresa usando com a própria infra de mídia.

---

### Definition of Done da primeira versão "completa" (Fases 1–6)

Duas pessoas conversam sem servidor central. Peers encaminham tráfego. Conexões impossíveis
diretamente usam relay. Mensagens são E2EE e chegam quando o destinatário volta. Chamadas funcionam
via WebRTC + TURN. Identidades são chaves criptográficas. Qualquer um pode hospedar um relay.

**Ao chegar aqui, parar de adicionar infraestrutura e avaliar.**

---

### Fase 7 — *(opcional)* Roteamento multi-hop / privado

**Critério de pronto**: remetente e destinatário não se veem diretamente.

---

### Fase 8 — *(opcional)* Blockchain: registro de identidade/devices, proof-of-relay/storage, incentivos

**Critério de pronto**: só se houver problema real de incentivo a infraestrutura. Ver
`ROADMAP.md`, "Onde blockchain entraria".
