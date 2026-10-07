use std::collections::VecDeque;
use std::time::Duration;

use futures::StreamExt;
use libp2p::multiaddr::Protocol;
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport};
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, autonat, dcutr, identify, identity::Keypair, mdns,
    noise, relay, tcp, yamux,
};

use crate::codec::{MessageCodec, PROTOCOL};
use crate::{Error, NodeConfig};

const PROTOCOL_VERSION: &str = concat!("/kin/", env!("CARGO_PKG_VERSION"));

#[derive(NetworkBehaviour)]
struct Behaviour {
    mdns: Toggle<mdns::tokio::Behaviour>,
    identify: identify::Behaviour,
    messages: request_response::Behaviour<MessageCodec>,
    relay_client: relay::client::Behaviour,
    relay_server: Toggle<relay::Behaviour>,
    dcutr: dcutr::Behaviour,
    autonat: autonat::Behaviour,
}

/// Identifica um envio, para casar com [`NodeEvent::MessageDelivered`] / [`NodeEvent::SendFailed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendId(OutboundRequestId);

/// O que o AutoNAT concluiu sobre a alcançabilidade deste nó pela internet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatStatus {
    /// Peers de fora conseguem discar este nó direto.
    Public,
    /// Atrás de NAT/firewall: só alcançável por relay ou hole punching.
    Private,
    Unknown,
}

/// Eventos de rede expostos ao resto do Kin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeEvent {
    /// O nó passou a escutar neste endereço.
    Listening(Multiaddr),
    /// Peer encontrado na LAN via mDNS.
    PeerDiscovered(PeerId, Multiaddr),
    /// Primeira conexão com o peer estabelecida (por qual caminho: ver [`NodeEvent::PeerRoute`]).
    PeerConnected(PeerId),
    /// Uma conexão com o peer foi estabelecida; `relayed` diz se passa por um relay. Chega junto de
    /// `PeerConnected` e de novo quando o hole punching troca o relay por uma conexão direta.
    PeerRoute { peer: PeerId, relayed: bool },
    /// O AutoNAT mudou a avaliação de alcançabilidade deste nó.
    NatStatus(NatStatus),
    /// O relay aceitou a reserva: este nó agora é alcançável pelo endereço de circuito.
    RelayReserved { relay: PeerId },
    /// Resultado de uma tentativa de hole punching com o peer (DCUtR).
    HolePunch {
        peer: PeerId,
        result: Result<(), String>,
    },
    /// O peer informou seus endereços de escuta e o endereço pelo qual nos viu.
    PeerIdentified {
        peer: PeerId,
        listen_addrs: Vec<Multiaddr>,
        observed_addr: Multiaddr,
    },
    /// Última conexão com o peer encerrada.
    PeerDisconnected(PeerId),
    /// Tentativa de conexão falhou (inclui Peer ID diferente do esperado).
    DialFailed {
        peer: Option<PeerId>,
        reason: String,
    },
    /// Chegou uma mensagem (bytes opacos) do peer; o ack já foi devolvido.
    MessageReceived { peer: PeerId, data: Vec<u8> },
    /// O peer confirmou o recebimento da mensagem enviada com [`Node::send`].
    MessageDelivered { peer: PeerId, id: SendId },
    /// O envio falhou (sem conexão, timeout, conexão encerrada ou peer sem o protocolo).
    SendFailed {
        peer: PeerId,
        id: SendId,
        reason: String,
    },
}

/// Nó de rede: escuta em TCP, conecta por endereço ou via mDNS e troca identificação com os peers.
pub struct Node {
    swarm: Swarm<Behaviour>,
    /// Eventos já produzidos mas ainda não entregues, em ordem.
    queue: VecDeque<NodeEvent>,
}

impl Node {
    /// Cria o nó usando a chave do device (o Peer ID sai dela).
    pub fn new(keypair: Keypair, config: NodeConfig) -> Result<Self, Error> {
        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| Error::Build(e.to_string()))?
            .with_quic()
            .with_dns()
            .map_err(|e| Error::Build(e.to_string()))?
            .with_relay_client(noise::Config::new, yamux::Config::default)
            .map_err(|e| Error::Build(e.to_string()))?
            .with_behaviour(|key, relay_client| {
                let mdns = if config.mdns {
                    Some(mdns::tokio::Behaviour::new(
                        mdns::Config::default(),
                        key.public().to_peer_id(),
                    )?)
                } else {
                    None
                };
                let identify = identify::Behaviour::new(identify::Config::new(
                    PROTOCOL_VERSION.into(),
                    key.public(),
                ));
                let messages = request_response::Behaviour::with_codec(
                    MessageCodec,
                    [(PROTOCOL, ProtocolSupport::Full)],
                    request_response::Config::default(),
                );
                let local = key.public().to_peer_id();
                let relay_server = config
                    .relay_server
                    .as_ref()
                    .map(|limits| relay::Behaviour::new(local, limits.into()));
                let autonat = autonat::Behaviour::new(
                    local,
                    autonat::Config {
                        only_global_ips: config.autonat_global_only,
                        ..Default::default()
                    },
                );
                Ok(Behaviour {
                    mdns: mdns.into(),
                    identify,
                    messages,
                    relay_client,
                    relay_server: relay_server.into(),
                    dcutr: dcutr::Behaviour::new(local),
                    autonat,
                })
            })
            .map_err(|e| Error::Build(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();
        let mut node = Self {
            swarm,
            queue: VecDeque::new(),
        };
        for addr in &config.external_addrs {
            node.swarm.add_external_address(addr.clone());
        }
        for relay in &config.relays {
            node.use_relay(relay.clone())?;
        }
        Ok(node)
    }

    pub fn peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    /// Escuta em todas as interfaces (TCP e QUIC), portas escolhidas pelo SO. Escutar também em
    /// UDP/QUIC é o que dá a melhor chance ao hole punching.
    pub fn listen(&mut self) -> Result<(), Error> {
        self.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        self.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)
    }

    /// Conecta ao relay `addr` (que deve terminar em `/p2p/<peer id do relay>`) e pede uma reserva.
    /// Quando aceita, chega [`NodeEvent::RelayReserved`] e o endereço de circuito aparece em
    /// [`NodeEvent::Listening`]; outros peers discam `Node::circuit_address(addr)`.
    pub fn use_relay(&mut self, addr: Multiaddr) -> Result<(), Error> {
        self.swarm.listen_on(addr.with(Protocol::P2pCircuit))?;
        Ok(())
    }

    /// Endereço pelo qual outros alcançam este nó através do relay `relay_addr`.
    pub fn circuit_address(&self, relay_addr: &Multiaddr) -> Multiaddr {
        relay_addr
            .clone()
            .with(Protocol::P2pCircuit)
            .with(Protocol::P2p(self.peer_id()))
    }

    /// Escuta em um endereço específico.
    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<(), Error> {
        self.swarm.listen_on(addr)?;
        Ok(())
    }

    /// Conecta direto a um endereço. Se o endereço terminar em `/p2p/<peer-id>`, a conexão só vale
    /// se o peer do outro lado provar ter esse Peer ID.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), Error> {
        self.swarm
            .dial(addr)
            .map_err(|e| Error::Dial(e.to_string()))
    }

    /// Envia bytes opacos a um peer já conectado. O resultado chega como `MessageDelivered` ou
    /// `SendFailed` com o mesmo [`SendId`].
    pub fn send(&mut self, peer: PeerId, data: Vec<u8>) -> SendId {
        SendId(
            self.swarm
                .behaviour_mut()
                .messages
                .send_request(&peer, data),
        )
    }

    /// Avança o nó até o próximo evento relevante.
    pub async fn next_event(&mut self) -> NodeEvent {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return event;
            }
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    if self.swarm.behaviour().relay_server.is_enabled() {
                        self.swarm.add_external_address(address.clone());
                    }
                    return NodeEvent::Listening(address);
                }
                SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    let mut first = None;
                    for (peer, addr) in list {
                        if self.swarm.is_connected(&peer) {
                            continue;
                        }
                        // Falha ao discar aqui não é fatal: o mDNS reanuncia depois.
                        let _ = self.swarm.dial(addr.clone());
                        first.get_or_insert((peer, addr));
                    }
                    if let Some((peer, addr)) = first {
                        return NodeEvent::PeerDiscovered(peer, addr);
                    }
                }
                SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received {
                    peer_id,
                    info,
                    ..
                })) => {
                    return NodeEvent::PeerIdentified {
                        peer: peer_id,
                        listen_addrs: info.listen_addrs,
                        observed_addr: info.observed_addr,
                    };
                }
                SwarmEvent::Behaviour(BehaviourEvent::Messages(event)) => {
                    if let Some(event) = self.on_message_event(event) {
                        return event;
                    }
                }
                SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
                )) => {
                    return NodeEvent::RelayReserved {
                        relay: relay_peer_id,
                    };
                }
                SwarmEvent::Behaviour(BehaviourEvent::Dcutr(event)) => {
                    return NodeEvent::HolePunch {
                        peer: event.remote_peer_id,
                        result: event.result.map(|_| ()).map_err(|e| e.to_string()),
                    };
                }
                SwarmEvent::Behaviour(BehaviourEvent::Autonat(autonat::Event::StatusChanged {
                    new,
                    ..
                })) => {
                    return NodeEvent::NatStatus(match new {
                        autonat::NatStatus::Public(_) => NatStatus::Public,
                        autonat::NatStatus::Private => NatStatus::Private,
                        autonat::NatStatus::Unknown => NatStatus::Unknown,
                    });
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint,
                    num_established,
                    ..
                } => {
                    let route = NodeEvent::PeerRoute {
                        peer: peer_id,
                        relayed: endpoint.is_relayed(),
                    };
                    // Só a primeira conexão é "conectou"; as seguintes (ex.: o hole punching
                    // trocando o relay por uma direta) são só mudança de rota.
                    if num_established.get() == 1 {
                        self.queue.push_back(route);
                        return NodeEvent::PeerConnected(peer_id);
                    }
                    return route;
                }
                SwarmEvent::ConnectionClosed {
                    peer_id,
                    num_established: 0,
                    ..
                } => return NodeEvent::PeerDisconnected(peer_id),
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    return NodeEvent::DialFailed {
                        peer: peer_id,
                        reason: error.to_string(),
                    };
                }
                _ => {}
            }
        }
    }

    fn on_message_event(
        &mut self,
        event: request_response::Event<Vec<u8>, ()>,
    ) -> Option<NodeEvent> {
        use request_response::{Event, Message};
        match event {
            Event::Message {
                peer,
                message:
                    Message::Request {
                        request, channel, ..
                    },
                ..
            } => {
                // Se o ack não puder ser enviado, o canal já fechou; o remetente reenvia.
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .messages
                    .send_response(channel, ());
                Some(NodeEvent::MessageReceived {
                    peer,
                    data: request,
                })
            }
            Event::Message {
                peer,
                message: Message::Response { request_id, .. },
                ..
            } => Some(NodeEvent::MessageDelivered {
                peer,
                id: SendId(request_id),
            }),
            Event::OutboundFailure {
                peer,
                request_id,
                error,
                ..
            } => Some(NodeEvent::SendFailed {
                peer,
                id: SendId(request_id),
                reason: error.to_string(),
            }),
            Event::InboundFailure { .. } | Event::ResponseSent { .. } => None,
        }
    }
}
