# Kin — PRD v0.1

## Vision

Uma **camada de comunicação descentralizada** em que os próprios dispositivos participantes formam a
infraestrutura. O mensageiro é a **primeira aplicação** construída sobre ela; arquivos, chamadas e
descoberta de serviços do ecossistema vêm depois, sobre a mesma rede.

Para o usuário, é um app de chat comum ("Fabio 🟢 online — Oi!"). DHT, relays e roteamento ficam
invisíveis.

### Propriedade central: *zero-infrastructure bootstrap*

A rede não precisa existir antes dos usuários. Ela nasce quando eles começam a participar:

```
Dia 1     A ↔ B                      (eu e um amigo: já funciona)
Dia 5     A ↔ B ↔ C
Dia 30    malha de ~10 peers
Dia 365   centenas/milhares de peers + relays voluntários
```

Quanto mais participantes, mais caminhos, relays e redundância. A rede melhora com o crescimento, mas
**funciona com 2**.

---

## Ecossistema

O Kin é parte do ecossistema open-source descentralizado do usuário:

- **TruthID** — identidade/autenticação descentralizada (o Kin consome a identidade dele)
- **Warden** — agente de IA pessoal
- **Anchor** — app de teses de investimento
- **Lume** — outro produto do usuário
- **Kin** — este projeto

No futuro, os serviços do ecossistema (Warden, Lume, nó TruthID...) podem ser descobertos pela
própria rede, sem depender de `api.dominio.com` (ver "Descoberta de serviços" abaixo).

---

## Objetivos

- Duas pessoas instalam o app e conversam **sem que nenhuma delas precise subir servidor, container ou VPS**.
- Nenhum servidor central obrigatório. Pode existir infraestrutura (bootstrap, relays), mas **nenhum nó é indispensável**.
- Todo app aberto pode **cooperar** com a rede (relay de mensagens, DHT, armazenamento temporário), com limites.
- E2EE em tudo. Relays só veem bytes cifrados.
- Identidade = chave criptográfica, não conta em servidor. Integração com TruthID.
- Empresas/entusiastas podem hospedar seus próprios relays. Opcional, nunca requisito.

## Non Goals (por enquanto)

- Blockchain no caminho das mensagens. **Nunca**: blockchain é péssima como banco de mensagens em tempo real.
- Token/economia de incentivo antes de existir uma rede que realmente precise dela.
- Chamadas de voz/vídeo roteadas por celulares aleatórios da rede.
- Anonimato forte estilo Tor no MVP. Roteamento privado é fase opcional tardia.
- App participando da rede em background no mobile no MVP.

---

## Princípios de design

1. **Descentralizado ≠ sem infraestrutura.** O objetivo é eliminar a *dependência* de uma autoridade central, não proibir que existam nós de bootstrap ou relays dedicados.
2. **Degradação graciosa.** Direto → relay por peer → relay dedicado. O app tenta na ordem e cai para o próximo sozinho.
3. **Contribuição com limites.** O app coopera, mas nunca vira um CDN no bolso do usuário.
4. **Blockchain como consequência, não pré-requisito.** A pergunta certa no futuro é "a rede *precisa* de blockchain?", não "como enfio blockchain nela?".
5. **Produto primeiro, rede depois.** Cada fase entrega algo usável.

---

## Arquitetura em camadas

```
┌──────────────────────────────────────────────┐
│  APLICAÇÕES   Chat · Arquivos · Chamadas ·    │
│               Descoberta de serviços          │
├──────────────────────────────────────────────┤
│  SEGURANÇA    E2EE · forward secrecy ·        │
│               rotação de chaves · multi-device│
├──────────────────────────────────────────────┤
│  IDENTIDADE   TruthID · chaves de device ·    │
│               Peer ID derivado                │
├──────────────────────────────────────────────┤
│  OVERLAY      Descoberta · DHT · Gossip ·     │
│               Roteamento · Relay ·            │
│               Store-and-forward               │
├──────────────────────────────────────────────┤
│  TRANSPORTE   QUIC · TCP · WebRTC             │
├──────────────────────────────────────────────┤
│  CONECTIVIDADE Internet · LAN · Wi-Fi Direct ·│
│               Bluetooth                       │
└──────────────────────────────────────────────┘
```

**Não existe "servidor da overlay".** A overlay é só o conjunto das conexões lógicas entre peers que
executam o mesmo protocolo:

```
            INTERNET
               │
     ┌─────────┼─────────┐
    📱A       📱B       📱C
     │ \     / │ \     / │
     │  📱D    │  📱E    │
     └─────────┼─────────┘
       todos executam o PROTOCOLO
```

### Papéis de cada nó

Todo nó pode acumular os três papéis:
- **Usuário:** envia/recebe as próprias mensagens.
- **Roteador/relay:** encaminha tráfego cifrado de terceiros.
- **Mailbox:** guarda temporariamente mensagens para destinatários offline.

---

## Identidade

- Cada usuário é uma **identidade criptográfica**, sem conta em servidor.
- Hierarquia de chaves:

```
Identidade (TruthID)
 └── Master Identity Key
      ├── Messaging Keys
      └── Device Keys
           ├── Device A → Peer ID A
           └── Device B → Peer ID B
```

- **TruthID não precisa de blockchain para isso.** O modo de identidade local do TruthID (keypair
  local, sem registro on-chain) basta para o MVP. O registro on-chain pode resolver
  `identidade → devices/chaves` em fase posterior.
- O Peer ID de rede é derivado da chave do device. A identidade pertence ao **ecossistema**, não ao
  app de mensagens.
- Este projeto é **separado** do TruthID e o consome. Não é um módulo de comunicação dentro do
  TruthID, ideia que já foi descartada antes.

---

## Conectividade: a escada de fallback

O app tenta, em ordem:

| Nível | Caminho | Quando |
|---|---|---|
| 0 | **Local** (mDNS/LAN, depois BT/Wi-Fi Direct) | Mesma rede ou proximidade física |
| 1 | **Direto** pela internet | IP alcançável ou hole punching bem-sucedido |
| 2 | **Relay por peer do app** | Outro participante com app aberto encaminha |
| 3 | **Relay dedicado/público** | Nó sempre ligado (empresa, entusiasta, público) |

### O problema real: NAT/CGNAT

Celulares quase sempre estão atrás de CGNAT. Conseguem sair para a internet, mas não aceitam conexão
de entrada. Soluções: hole punching (ex.: DCUtR do libp2p) e relay quando ele falha. Hole punching
precisa de um terceiro alcançável para coordenar. Isso é inevitável, mas esse terceiro pode ser
qualquer peer ou relay público.

### Seleção de relay

O app mantém uma **lista de candidatos** (pré-cadastrados + descobertos pela rede) e mede
continuamente latência, disponibilidade, taxa de falha e capacidade. Usa o melhor e mantém fallbacks.
O ranking é interno e local, sem "relay oficial".

---

## Descoberta de peers

Separar duas perguntas:
- **Existência:** "existe um usuário com essa identidade?"
- **Alcançabilidade:** "por onde mando um pacote para ele?"

### Mecanismo: registros assinados + gossip + DHT (sem blockchain)

1. Ao entrar, o nó emite um `ANNOUNCE` assinado:
   ```
   ANNOUNCE
     peer_id, endereços, capacidades (messaging|relay|mailbox),
     timestamp, assinatura
   ```
   A assinatura prova a origem. Ninguém forja o anúncio de outro.
2. Peers conectados propagam o que conhecem via **gossip**, uma "fofoca criptograficamente verificável".
3. Com a rede maior, uma **DHT (Kademlia)** responde `peer_id → onde encontrar`, sem que ninguém
   tenha a tabela inteira. Cada nó conhece dezenas de peers, não 100k.

**Por que não uma "pseudo-blockchain" de entradas na rede:** um log ordenado de "Peer X entrou" exige
decidir quem define o próximo bloco. Isso é consenso global, uma complexidade enorme para um problema
que registros assinados com timestamp resolvem sem ordem global. Vale o "last-writer-wins" pelo
timestamp assinado.

**Comparação com Arweave:** "falar o protocolo da rede" não elimina a necessidade de saber *com quem*
falar. O Arweave tem gateways conhecidos. Aqui, a própria rede distribui essa informação.

### Primeiro contato (o problema de verdade)

- **Mesma LAN:** mDNS, zero servidor.
- **Internet:** precisa de pelo menos um ponto de encontro conhecido: um peer já conhecido, um relay
  público da lista embutida, ou um convite (QR/link com endereços + chave pública, no mesmo padrão do
  pareamento QR do TruthID).

---

## Store-and-forward (mensagens offline)

- Destinatário offline → a mensagem cifrada fica em um ou mais nós mailbox. Na reconexão, o
  destinatário puxa.
- O mailbox só vê bytes cifrados + metadados mínimos de roteamento.
- Replicar em N mailboxes para redundância, com TTL.

### ⚠️ Limitação do "dia zero" com 2 usuários

Com só Fabio e João, **se os dois nunca estiverem online ao mesmo tempo, P2P puro não entrega nada.**
App-relay não resolve isso, porque não existe um terceiro nó ligado. Nesse cenário:
- A mensagem fica na outbox do remetente até haver janela simultânea (modelo do Briar). Funciona, mas
  é assíncrono de verdade.
- **Ou** usa-se um nó sempre ligado como mailbox. Ver as opções em D1 (`ARCHITECTURE.md`).

Essa é a principal tensão entre "funciona desde o dia zero" e "sem nenhum servidor". Ela é tratada
como decisão explícita, não como detalhe.

---

## App como relay (contribuição)

Quando aberto, o app pode manter conexões, participar da DHT/gossip, encaminhar pacotes e guardar
mensagens temporariamente, **sempre com limites configuráveis**. Defaults sugeridos:

```
max_relay_connections: 5–10
max_relay_bandwidth:   limitado (ex.: poucos MB/s)
max_mailbox_storage:   ~100 MB
only_on_wifi:          true
only_when_charging:    opcional
min_battery:           ex.: 30%
```

### Realidade por plataforma

- **Desktop/Linux:** participa plenamente; pode rodar como daemon. É o melhor relay natural.
- **Android:** possível com foreground service e notificação persistente, com custo de bateria e UX.
- **iOS:** praticamente sem execução em background para isso. Relay só com o app em primeiro plano.

**MVP:** "quando o app está ativo, participa plenamente". Background vem depois. Na prática, os relays
naturais da rede serão os **desktops**.

---

## Aplicações

### Chat (MVP)

1:1, grupos/canais, presença, offline delivery, respostas/reactions, notificações, sincronização entre
devices do mesmo usuário.

Posicionamento do produto em aberto (D4): WhatsApp-like (N↔N pessoal), Slack/Discord-like
(workspaces/canais) ou broadcast 1→N (Twitter-like).

### Arquivos

Fotos/vídeos/arquivos via P2P direto ou em chunks por relay. Arquivo grande em relay de celular é
ruim, então limitar tamanho em app-relay.

### Chamadas (fora do MVP)

- **Não inventar protocolo de mídia.** WebRTC.
- P2P direto quando possível. Fallback via **TURN**. Voz/vídeo são sensíveis à latência, e relay por
  celular aleatório não serve.
- STUN público é trivial e gratuito. TURN público gratuito existe, mas com limites e sem garantia.
  Para uso sério, TURN dedicado (empresa sobe o próprio, opt-in).
- Grupos pequenos em mesh; chamadas grandes exigem SFU (fase muito posterior).

### Descoberta de serviços do ecossistema (futuro)

Serviços do ecossistema (Warden, Lume, nó TruthID...) sendo descobertos pela rede, sem depender de
`api.dominio.com`:
```
service://<ns>/...
```
