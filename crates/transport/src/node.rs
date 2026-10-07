use std::time::Duration;

use futures::StreamExt;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, Swarm, SwarmBuilder, identity::Keypair, mdns, noise, tcp, yamux};

use crate::Error;

#[derive(NetworkBehaviour)]
struct Behaviour {
    mdns: mdns::tokio::Behaviour,
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
    /// Última conexão com o peer encerrada.
    PeerDisconnected(PeerId),
}

/// Nó de rede: escuta em TCP, descobre peers na LAN e conecta neles.
pub struct Node {
    swarm: Swarm<Behaviour>,
}

impl Node {
    /// Cria o nó usando a chave do device (o Peer ID sai dela).
    pub fn new(keypair: Keypair) -> Result<Self, Error> {
        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| Error::Build(e.to_string()))?
            .with_behaviour(|key| {
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;
                Ok(Behaviour { mdns })
            })
            .map_err(|e| Error::Build(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();
        Ok(Self { swarm })
    }

    pub fn peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    /// Começa a escutar em todas as interfaces, porta escolhida pelo SO.
    pub fn listen(&mut self) -> Result<(), Error> {
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        Ok(())
    }

    /// Conecta direto a um endereço conhecido.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), Error> {
        self.swarm
            .dial(addr)
            .map_err(|e| Error::Build(e.to_string()))
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
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    return NodeEvent::PeerConnected(peer_id);
                }
                SwarmEvent::ConnectionClosed {
                    peer_id,
                    num_established: 0,
                    ..
                } => return NodeEvent::PeerDisconnected(peer_id),
                _ => {}
            }
        }
    }
}
