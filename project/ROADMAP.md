# Roadmap e Evoluções Planejadas

## Sequenciamento Sugerido

> **2026-09-27 (Sessão 1)** — projeto registrado como pré-projeto. Entra na fila do ecossistema
> **depois dos projetos em andamento** (TruthID, Warden, Anchor, Lume). Quando começar: Fase 0
> (estudo de prior art + decisões D1–D4) e depois as fases em ordem, respeitando o Definition of Done
> das Fases 1–6 (`PHASE.md`).

Resumo das fases (detalhe em `PHASE.md`):

| Fase | Entrega | Critério de pronto |
|---|---|---|
| **1** | Identidade local + conexão direta (LAN via mDNS e internet) + chat 1:1 E2EE | Fabio e amigo conversam na mesma rede e pela internet quando há conectividade direta |
| **2** | Hole punching + relay (público/de peer) + convite por QR/link | Conversam atrás de CGNAT sem nenhum dos dois subir servidor |
| **3** | Store-and-forward (mailbox) + multi-device | Mensagem chega mesmo com destinatário offline no envio |
| **4** | Overlay: gossip + DHT + app-relay com limites | 10–50 peers; mensagem encaminhada por peers intermediários |
| **5** | Grupos/canais, arquivos, presença, notificações | Uso diário real por um grupo pequeno |
| **6** | Chamadas WebRTC + TURN opt-in; relay dedicado empacotado para empresas | Pequena empresa usando com a própria infra de mídia |
| **7** | *(opcional)* Roteamento multi-hop / privado | Remetente e destinatário não se veem diretamente |
| **8** | *(opcional)* Blockchain: registro de identidade/devices, proof-of-relay/storage, incentivos | Só se houver problema real de incentivo a infraestrutura |

---

## Prior art para estudar antes de codar

| Projeto | O que olhar |
|---|---|
| **Briar** | P2P puro, Bluetooth/Wi-Fi/Tor, entrega só com ambos online |
| **Berty / Wesh** | Mensageiro sobre libp2p, offline-first, BLE |
| **Jami** | Sem servidor, DHT própria (OpenDHT), chamadas P2P |
| **Session** | Rede de nós (service nodes) + onion routing + swarms como mailbox |
| **SimpleX** | Filas em relays sem identificador de usuário; relays self-hosted |
| **Nostr** | Relays públicos burros + clientes inteligentes; DMs cifradas |
| **Matrix P2P (Pinecone)** | Tentativa de levar Matrix para overlay P2P |
| **libp2p docs** | Circuit relay v2, DCUtR, Kademlia, GossipSub, AutoNAT |

A pergunta útil para cada um: *por que ele fez a escolha que fez para mensagens offline e NAT?* É aí
que está o problema difícil deste projeto.

---

## Onde blockchain entraria (Fase 8, se entrar)

**Não é necessária para:** chat, E2EE, chamadas, P2P, relay, DHT, descoberta, arquivos, sync,
presença, offline.

**Pode agregar:**
- Registro verificável `identidade → devices/chaves` (via TruthID on-chain).
- Incentivos: proof-of-relay / proof-of-storage → recompensa. Resolve a pergunta "por que alguém
  rodaria relay de chamada para estranhos?", que hoje não tem boa resposta sem incentivo.
- Governança do protocolo (muito depois).

Ordem de introdução de incentivos: **reputação + rate limiting + proof of contribution primeiro**;
token só se houver motivo econômico real.

---

## Riscos

- **Mobile em background** é o maior limitador prático, mais do que a rede em si.
- **Dia zero com 2 usuários e offline:** ver D1. Sem mailbox, a UX é assíncrona.
- **Relays públicos de terceiros** podem mudar política, cair ou exigir pagamento. Mitigação: vários
  relays + transporte abstrato.
- **Escopo:** é fácil virar "reinventar a internet". Seguir as fases e respeitar o Definition of Done.

---

## Ideias de Expansão (Brainstorm — sem `/plan`)

### Descoberta de serviços do ecossistema

Serviços do ecossistema (Warden, Lume, nó TruthID...) descobertos pela própria rede, sem depender de
`api.dominio.com`, via endereços `service://<ns>/...`.
