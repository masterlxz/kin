# Estudo de Prior Art (Fase 0.1 / P8)

> Feito na Sessão 2 (2026-10-06). Pergunta-guia para cada projeto: **por que ele fez a escolha que
> fez para mensagens offline e NAT?**
>
> **Fontes e confiança**: itens marcados com *(pesquisa)* vêm de busca na web nesta sessão; itens
> marcados com *(conhecimento prévio)* vêm do conhecimento geral do modelo e **devem ser verificados
> na documentação oficial antes de virarem premissa de decisão**. Nenhuma decisão D1–D6 foi tomada aqui.

---

## Resumo: as três estratégias para "destinatário offline"

| Estratégia | Quem usa | Custo |
|---|---|---|
| **Só entrega com os dois online** (sync oportunista) | Briar (core), Berty (sem replication server) | UX assíncrona/ruim; privacidade máxima |
| **DHT/peers guardam a mensagem** | Jami (OpenDHT) | Retenção curta e não garantida; depende de peers estáveis |
| **Relays/nós dedicados guardam (mailbox)** | Session (swarms), SimpleX (SMP), Nostr (relays) | Alguém precisa hospedar; metadados ficam expostos a ele |

E duas estratégias para NAT:

| Estratégia | Quem usa |
|---|---|
| **Fugir do problema**: Tor onion services (sem IP exposto) ou relays sempre-públicos | Briar, Session, SimpleX, Nostr |
| **Atravessar**: ICE/STUN/TURN ou DCUtR + relay | Jami, Berty/libp2p |

**Padrão que emerge**: quem **garante entrega offline sem servidor próprio** sempre acaba com algum
tipo de nó sempre-ligado (service node, SMP, relay Nostr, DHT proxy). Ninguém resolveu isso só com
peers de celular. Isso valida a hipótese de D1 (`d + b`).

---

## Briar

- **Modelo**: P2P puro, sem servidor, sem número de telefone. Sincroniza por **Tor** (internet),
  **Bluetooth/Wi-Fi** (sem internet) e até **cartão de memória** *(pesquisa)*.
- **Offline**: entrega só quando os dois dispositivos se conectam. Para horários diferentes existe o
  **Briar Mailbox**, um dispositivo próprio (ex.: Raspberry Pi) do próprio usuário que fica sempre
  ligado e guarda mensagens *(pesquisa)*.
- **NAT**: resolvido por Tor onion services: cada contato publica um serviço onion e ninguém precisa
  de IP alcançável nem de hole punching *(conhecimento prévio)*.
- **Por que essas escolhas**: o threat model é jornalistas/ativistas sob censura e corte de internet.
  Privacidade de metadados e resiliência (funcionar sem internet) valem mais que UX.
- **Lição para o Kin**: o "mailbox próprio do usuário" é um padrão legítimo, mas é a opção **e** de
  D1 (nó no homelab), que o projeto quer evitar como obrigatória. Tor resolve NAT, porém com latência
  alta e dependência da rede Tor; ruim para chamadas e para background mobile.

## Berty / Wesh

- **Modelo**: protocolo Wesh sobre **libp2p + IPFS + OrbitDB (CRDT)**. Cada grupo é um log
  replicado com cabeça (head) que referencia as mensagens anteriores *(pesquisa)*.
- **Transportes diretos**: Android Nearby, Multipeer Connectivity (iOS) e BLE entre SOs; dá para usar
  sem internet *(pesquisa)*.
- **NAT**: libp2p para os peers se acharem além de NAT/firewall *(pesquisa)*.
- **Offline**: replicação via CRDT quando os peers se encontram; para caso assíncrono existe um
  **replication server** opcional que guarda dados cifrados que ele não consegue ler *(conhecimento
  prévio)*.
- **Por que**: foco em offline-first e mesh local (BLE), com infra opcional em vez de obrigatória.
- **Lição para o Kin**: é o prior art mais próximo da stack (libp2p). Mostra que CRDT por grupo é
  elegante, mas pesa em mobile e é complexo. O replication server é, na prática, uma mailbox opt-in,
  o mesmo desenho que `d + b` propõe. Vale checar quão ativo o projeto está antes de depender dele.

## Jami

- **Modelo**: cada conta roda um nó **OpenDHT (Kademlia)**. A DHT serve para estabelecer conexão
  (troca de candidatos ICE, cifrados) e para distribuir mensagens *(pesquisa)*.
- **NAT**: **ICE** para achar o caminho mais direto, **TURN** como fallback quando não há conexão
  direta *(pesquisa)*. Chamadas são P2P (SIP/SRTP) sobre isso.
- **Offline**: a mensagem é colocada em nós da DHT para o contato buscar depois; **DHT proxy** é
  usado em mobile para economizar bateria e receber push *(pesquisa)*.
- **Por que**: queria zero servidor central e reaproveitar padrões de telefonia (ICE/TURN/SIP) que
  já resolvem NAT e mídia.
- **Lição para o Kin**: usar DHT como armazenamento de mensagens funciona, mas a retenção é curta e
  não garantida (nós entram e saem), e o mobile acaba precisando de um proxy, ou seja, infra. ICE+TURN
  é a resposta madura para chamadas (Fase 6, WebRTC faz o mesmo).

## Session

- **Modelo**: fork do Signal que troca servidor central por **Service Nodes** (rede de nós
  hospedados pela comunidade, com stake em cripto) *(pesquisa)*.
- **Offline**: o destinatário tem um **swarm** (5–7 service nodes) determinado de forma determinística
  pela chave pública. O remetente envia para 3 nós do swarm, que replicam entre si e guardam a
  mensagem até o **TTL (padrão 14 dias)** *(pesquisa)*.
- **NAT**: não existe problema, porque os service nodes são públicos e o cliente só faz conexões de
  saída. Para esconder o IP, as requisições passam por **onion requests** de 3 saltos *(pesquisa)*.
- **Por que**: queria o modelo "servidor burro" do Signal sem entidade única, e metadados mínimos
  (sem telefone, IP escondido).
- **Lição para o Kin**: é o melhor exemplo de **mailbox descentralizada com localização
  determinística** (chave pública → swarm). Funciona porque há **incentivo econômico** para rodar
  nós, justamente o problema da Fase 8. Sem token, o Kin não consegue reproduzir isso no dia zero.
  Histórico: o Session removeu o PFS (Perfect Forward Secrecy) do protocolo; fica o alerta de que
  simplicidade de mailbox pode custar propriedades de segurança *(conhecimento prévio, verificar)*.

## SimpleX

- **Modelo**: sem identificadores de usuário, nem números aleatórios. Cada conexão tem **duas filas
  unidirecionais** (uma por direção), endereçadas por IDs por fila e não por usuário *(pesquisa)*.
- **Offline**: **relays SMP** guardam as mensagens da fila. O servidor autoriza o envio na fila mas
  **não autentica usuário**, não tem registro de usuários e os relays não se falam entre si. A
  implementação atual guarda mensagens **em memória**, persistindo só os registros das filas
  *(pesquisa)*. Remetente e destinatário podem usar servidores diferentes.
- **NAT**: não há P2P; todos os clientes só fazem conexões de saída para relays públicos.
- **Por que**: privacidade de metadados em primeiro lugar; quem hospeda o relay não aprende o grafo
  social.
- **Lição para o Kin**: mostra a mailbox com **menos vazamento de metadados** (opção **c** de D1).
  O custo é abrir mão de P2P de verdade e depender do protocolo/ecossistema deles, o que exigiria
  compatibilizar ou reimplementar. Para o Kin, vale o **conceito** (filas por conexão, sem ID global)
  mesmo que o backend seja outro.

## Nostr

- **Modelo**: **relays públicos "burros"** que guardam e repassam eventos assinados; os clientes
  fazem o trabalho inteligente *(pesquisa)*.
- **DMs privadas (NIP-17)**: criptografadas com NIP-44 e embrulhadas em **gift wrap (NIP-59)**,
  que esconde remetente, timestamp real e metadados; o evento interno **não é assinado** (deniability).
  O usuário publica a lista de relays preferidos para receber DMs (**kind 10050**) e o remetente
  publica o gift wrap (kind 1059) nesses relays *(pesquisa)*.
- **NAT**: inexistente como problema, pois são clientes → relay.
- **Por que**: simplicidade extrema e resistência a censura por redundância (mande para N relays).
- **Lição para o Kin**: dá **store-and-forward pronto** em milhares de relays públicos, sem subir
  nada (base da hipótese `b`). Riscos: política e retenção variam por relay, anti-spam (alguns cobram),
  e o relay ainda enxerga timing/tamanho/chave de destino. NIP-17 é o precedente de como esconder
  remetente numa mailbox pública. Sem forward secrecy nativo de NIP-44 (conhecimento prévio, verificar),
  então o E2EE em camada (D2) teria de vir por cima.

## Matrix P2P (Pinecone)

- **Modelo**: levar o homeserver (Dendrite) **para dentro do cliente** e conectar clientes por um
  **overlay P2P** chamado **Pinecone**, inspirado no Yggdrasil, com roteamento por source routing e
  virtual ring routing além de greedy routing *(pesquisa)*.
- **Offline/NAT**: o overlay roteia entre peers independentemente do meio de conexão; para
  entrega assíncrona, depende de homeservers tradicionais continuando como fallback/relay
  *(conhecimento prévio)*.
- **Por que**: reaproveitar o protocolo Matrix inteiro (salas, federação) sem servidor central.
- **Lição para o Kin**: é o aviso de **escopo**. Fazer P2P *e* manter a semântica de um protocolo
  grande (estado de salas, federação) é pesado; o experimento ficou como prova de conceito e o Matrix
  continua federado em produção *(conhecimento prévio, verificar status atual)*. Overlay próprio de
  roteamento (Pinecone, Yggdrasil) é pesquisa de rede, o oposto de "preferir bibliotecas maduras".

## libp2p (docs e medições)

- **Peças**: circuit relay v2, DCUtR, AutoNAT, Kademlia, GossipSub (ver `ARCHITECTURE.md` D1/D3).
- **Medição real (IMC '26, Trautwein et al.)** *(pesquisa)*: 4,4 milhões de tentativas, 85 mil redes,
  167 países. Taxa de sucesso do **hole punching (DCUtR) ≈ 70% ± 7,1%** (TCP e QUIC equivalentes),
  **condicionada** a conseguir antes uma reserva de relay e descobrir endereço público, etapas que
  falham em ~29% das tentativas; ~36% dos peers atrás de NAT nunca conseguem reserva de relay.
  97,6% dos sucessos acontecem na primeira tentativa.
- **Por que o circuit relay v2 é limitado**: foi pensado para **coordenar** o hole punching, com
  reservas limitadas em tempo e bytes, e não para transportar tráfego (ver D1 opção **a**).
- **Lição para o Kin**: contar com ~70% de conexão direta (e bem menos no pior caso) significa que
  **o fallback não é opcional**: ~30% dos pares precisam de relay de verdade. Isso reforça a
  necessidade de um relay que carregue tráfego (não só o circuit relay v2 padrão), e é uma razão
  forte para olhar o relay do iroh, que encaminha pacotes cifrados em vez de reservar circuitos.

---

## Síntese por decisão em aberto

| Decisão | O que o estudo mostra |
|---|---|
| **D1 (mailbox)** | Todos que garantem offline têm nó sempre-ligado. As duas famílias viáveis sem infra própria são "relays públicos baratos" (Nostr) e "filas sem ID" (SimpleX). Session só funciona com incentivo. O padrão **outbox local + mailbox em relay + transporte abstrato** é consistente com o que Berty (replication server) e Briar (Mailbox) fazem de forma opt-in |
| **D3 (stack)** | libp2p dá DHT/gossip prontos, mas ~30% dos pares dependem de relay que carregue tráfego e a taxa de hole punching medida é ~70%. O iroh foca em conectividade (hole punching + relay stateless embutidos). Berty prova que libp2p em mobile é viável, mas com custo de complexidade |
| **D2 (E2EE)** | Session e Nostr mostram que mailbox simples tende a perder forward secrecy se não houver camada própria de E2EE por cima. MLS/Double Ratchet precisam ser **independentes do transporte** |
| **D4 (posicionamento)** | Briar/Berty/Jami miram mensageiro pessoal; Nostr é broadcast público + DM. Broadcast 1→N é o caso mais barato (Nostr prova) |
| **D5 (anti-spam)** | Nostr e SimpleX delegam ao operador do relay; Session usa stake dos nós. Sem relay próprio, o Kin herda a política de cada relay |
| **D6 (metadados)** | SimpleX (sem ID) e Session (onion) são o teto; Nostr gift wrap é o meio-termo barato. Hipótese de "pouco no MVP" é coerente |

## Perguntas que o estudo deixou abertas (para a fase de decisões)

1. O Kin vale o custo de uma **dependência de ecossistema externo** (Nostr) para ter mailbox no
   dia zero, ou o MVP aceita entrega só com os dois online (como o Briar) e adia a mailbox?
2. Aceitamos **~30% de conexões via relay** como caso normal e dimensionamos o relay para isso?
3. Queremos **relays que qualquer peer desktop possa ser** (Fase 4), o que aproxima o Kin do Session
   sem token? Quem protege esse relay de abuso (D5)?
4. O mobile em background (maior risco do `ROADMAP.md`) exige push/proxy como o Jami faz?
   Isso seria uma exceção ao "sem servidor"?

## Fontes consultadas

- [Briar (site oficial)](https://briarproject.org)
- [Berty Protocol docs](https://berty.tech/protocol) e [Wesh: flight of a byte](https://dev.to/bertytechnologies/wesh-flight-of-a-byte-1bn9)
- [Jami: distributed network](https://docs.jami.net/user/jami-distributed-network.html) e [Establishing P2P connections with Jami](https://jami.net/establishing-peer-to-peer-connections-with-jami/)
- [Session: paper (arXiv 2002.04609)](https://arxiv.org/pdf/2002.04609) e [Onion requests and message routing](https://docs.getsession.org/session-network/session-protocol/onion-requests-and-message-routing)
- [SimpleX Chat: sem IDs de usuário](https://pyshine.com/SimpleX-Chat-Private-Messaging-Without-User-IDs/)
- [Nostr NIP-17](https://nips.4rs.nl/nips/17) e [Private DMs (OpenSats)](https://opensats.org/topics/private-dms)
- [Pinecones and Dendrites (FOSDEM 2021)](https://archive.fosdem.org/2021/schedule/event/matrix_pinecones/) e [Growing Pinecones for P2P Matrix (FOSDEM 2022)](https://archive.fosdem.org/2022/schedule/event/matrix_p2p_pinecone/)
- [Large-Scale Measurement of NAT Traversal: DCUtR in IPFS (arXiv 2604.12484)](https://arxiv.org/pdf/2604.12484) e [Challenging Tribal Knowledge (arXiv 2510.27500)](https://arxiv.org/pdf/2510.27500)
- [Iroh 1.0: Dial Keys, Not IPs](https://pinggy.io/blog/iroh_1_0_dial_keys_not_ips/) e [iroh road to 1.0](https://iroh.computer/blog/road-to-1-0)
