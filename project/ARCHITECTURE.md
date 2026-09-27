# Decisões de Arquitetura

## Registro de Decisões

| Decisão | Opções | Status |
|---|---|---|
| Nome do projeto | TruthNet vs outros | **Kin** (provisório) ✓ — decidido na Sessão 1. "TruthNet" foi rejeitado. Pode mudar |
| Relação com o TruthID | Módulo dentro do TruthID vs projeto separado que o consome | **Projeto separado** ✓ — a ideia de módulo de comunicação dentro do TruthID já foi descartada antes |
| Blockchain no caminho das mensagens | Sim vs não | **Nunca** ✓ — blockchain é péssima como banco de mensagens em tempo real. Blockchain só na Fase 8 opcional, para registro/incentivos |
| Descoberta de peers | Pseudo-blockchain de entradas vs registros assinados + gossip + DHT | **Registros assinados + gossip + DHT** ✓ — sem consenso global; last-writer-wins pelo timestamp assinado |
| Protocolo de mídia (chamadas) | Próprio vs WebRTC | **WebRTC** ✓ — não inventar protocolo de mídia |
| Background no mobile | No MVP vs depois | **Depois** ✓ — MVP: "quando o app está ativo, participa plenamente" |
| D1 — Fallback/mailbox do dia zero | Ver D1 abaixo | **Em aberto** — hipótese: d + b, com a só para hole punching |
| D2 — E2EE | MLS vs Double Ratchet + sender keys | **Em aberto** — hipótese: MLS para tudo |
| D3 — Stack | `rust-libp2p` vs `iroh` | **Em aberto** — núcleo em Rust confirmado; Tauri (desktop) + Flutter via FFI (mobile) no padrão do TruthID |
| D4 — Posicionamento do produto | WhatsApp-like vs Slack/Discord-like vs broadcast | **Em aberto** |
| D5 — Anti-spam / Sybil | Rate limit, PoW, convites, reputação TruthID, depósito | **Em aberto** |
| D6 — Metadados | Quanto de privacidade de metadados no MVP | **Em aberto** — provavelmente pouco no MVP; resto na Fase 7 |

---

## Decisões em Aberto (detalhe)

> Antes de gerar código que dependa de uma delas, levantar as opções com trade-offs e pedir decisão.

### D1 — Transporte de fallback/mailbox no dia zero (a mais importante)

O objetivo é não depender de servidor próprio, nem com 2 usuários, e ainda assim entregar mensagem
offline.

| Opção | Prós | Contras |
|---|---|---|
| **a) Relays públicos libp2p (circuit relay v2)** | Nativo do libp2p; muitos nós IPFS públicos | Projetado para **coordenar hole punching**, não para carregar tráfego: reservas com limite de duração e de bytes. Não serve como mailbox. Útil só como degrau para conexão direta |
| **b) Relays Nostr como mailbox** | Milhares de relays públicos já existentes, muitos gratuitos; guardam eventos (store-and-forward pronto); protocolo simples; payload cifrado (dá para usar gift-wrap para esconder o remetente) | Relays veem metadados (chave de destino, timing, tamanho); políticas de retenção e anti-spam variam; alguns são pagos; dependência de ecossistema externo |
| **c) Modelo SimpleX (filas em relays sem IDs de usuário)** | Melhor privacidade de metadados do mercado; qualquer um hospeda | Os relays existentes são da rede SimpleX, então exigiria compatibilizar ou reimplementar o protocolo |
| **d) Outbox local, sem mailbox** | Zero infra, 100% puro | Entrega só com os dois online simultaneamente |
| **e) Nó próprio no homelab como um relay entre vários** | Controle total; custo zero | É exatamente o "container só pra isso" que se quer evitar. Aceitável só como mais um item da lista |

Hipótese a validar: **d + b como fallback**, com **a** só para hole punching. Transporte abstrato
para trocar o backend de mailbox sem mexer no resto.

### D2 — E2EE

- **MLS (RFC 9420)**, ex.: crate `openmls`: padrão para grupos, escala bem, multi-device nativo. Mais complexo.
- **Double Ratchet (estilo Signal)** para 1:1 + sender keys para grupos: maduro, porém menos elegante
  para grupos grandes e multi-device.
- Hipótese: MLS para tudo, para não ter dois sistemas.

### D3 — Stack

- Núcleo de rede em **Rust** (`rust-libp2p`), reaproveitável entre desktop (Tauri) e mobile (Flutter
  via FFI/`flutter_rust_bridge`), no mesmo padrão do TruthID.
- Alternativa a avaliar: `iroh` (Rust, QUIC, hole punching + relays próprios), mais simples que
  libp2p, porém com relay de modelo mais centralizado por padrão.

### D4 — Posicionamento do produto

WhatsApp-like, Slack/Discord-like ou broadcast (Twitter-like)? Muda modelo de dados, grupos e
moderação. Broadcast 1→N é tecnicamente bem mais simples (gossip/pub-sub puro, sem E2EE de grupo
privado) e pode ser um MVP mais barato.

### D5 — Anti-spam / Sybil

Criar 1 milhão de identidades é grátis. Candidatos: rate limiting por identidade, proof-of-work leve
no primeiro contato, convites/trust graph, reputação atrelada ao TruthID, depósito em casos
específicos. Relays públicos (principalmente Nostr) já têm as próprias políticas, e o app precisa
conviver com elas.

### D6 — Metadados

E2EE protege conteúdo, não quem fala com quem e quando. Definir quanto de privacidade de metadados
entra no MVP (provavelmente pouco) e quanto fica para a Fase 7.
