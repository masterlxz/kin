use std::time::Duration;

use futures::StreamExt;
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport};
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{
    Multiaddr, PeerId, Swarm, SwarmBuilder, identify, identity::Keypair, mdns, noise, tcp, yamux,
};

use crate::Error;
use crate::codec::{MessageCodec, PROTOCOL};

const PROTOCOL_VERSION: &str = concat!("/kin/", env!("CARGO_PKG_VERSION"));

#[derive(NetworkBehaviour)]
struct Behaviour {
    mdns: Toggle<mdns::tokio::Behaviour>,
    identify: identify::Behaviour,
    messages: request_response::Behaviour<MessageCodec>,
}

/// Identifica um envio, para casar com [`NodeEvent::MessageDelivered`] / [`NodeEvent::SendFailed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SendId(OutboundRequestId);

/// Opções do nó.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Descobrir e conectar a peers da LAN via mDNS.
    pub mdns: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self { mdns: true }
    }
}

/// Eventos de rede expostos ao resto do Kin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeEvent {
    /// O nó passou a escutar neste endereço.
    Listening(Multiaddr),
    /// Peer encontrado na LAN via mDNS.
    PeerDiscovered(PeerId, Multiaddr),
    /// Conexão com o peer estabelecida.
    PeerConnected(PeerId),
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
            .with_behaviour(|key| {
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
                Ok(Behaviour {
                    mdns: mdns.into(),
                    identify,
                    messages,
                })
            })
            .map_err(|e| Error::Build(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();
        Ok(Self { swarm })
    }

    pub fn peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    /// Escuta em todas as interfaces, porta escolhida pelo SO.
    pub fn listen(&mut self) -> Result<(), Error> {
        self.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)
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
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
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
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    return NodeEvent::PeerConnected(peer_id);
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
