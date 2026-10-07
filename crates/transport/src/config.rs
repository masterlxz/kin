use std::time::Duration;

use libp2p::{Multiaddr, relay};

/// Opções do nó.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Descobrir e conectar a peers da LAN via mDNS.
    pub mdns: bool,
    /// Relays candidatos para quando este nó está atrás de NAT. O nó mede cada um (sucesso e
    /// latência), pede reserva nos melhores (`relay_count`) e troca por outro quando um falha. Com
    /// reserva, ganha um endereço de circuito (`<relay>/p2p-circuit/p2p/<este peer>`) que outros
    /// podem discar. Cada endereço precisa terminar em `/p2p/<peer id do relay>`.
    pub relays: Vec<Multiaddr>,
    /// Em quantos relays (dos melhores, ver [`NodeConfig::relays`]) manter reserva ao mesmo tempo.
    /// Se um falha, o próximo candidato assume.
    pub relay_count: usize,
    /// Se `Some`, este nó também **serve** de relay para os outros, com estes limites. O relay
    /// anuncia seus endereços de escuta como externos (o cliente recusa uma reserva sem endereços);
    /// atrás de port-forward, informe o endereço público em [`NodeConfig::external_addrs`].
    pub relay_server: Option<RelayLimits>,
    /// Endereços pelos quais este nó sabe que é alcançável de fora (ex.: IP público + porta
    /// encaminhada). Anunciados aos peers e usados pelo hole punching.
    pub external_addrs: Vec<Multiaddr>,
    /// O AutoNAT só considera peers/endereços de IP global (padrão). Desligar em testes e
    /// laboratórios, onde tudo é loopback ou rede privada.
    pub autonat_global_only: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            mdns: true,
            relays: Vec::new(),
            relay_count: 2,
            relay_server: None,
            external_addrs: Vec::new(),
            autonat_global_only: true,
        }
    }
}

/// Limites de um nó que serve de relay (circuit relay v2).
///
/// O padrão do libp2p (128 KiB e 2 min por circuito) só serve para coordenar hole punching. Um relay
/// que **carrega conversas** (D3) precisa de limites bem maiores, por isso o padrão aqui é generoso e
/// configurável.
#[derive(Debug, Clone)]
pub struct RelayLimits {
    pub max_reservations: usize,
    pub max_circuits: usize,
    /// Quanto tempo um circuito pode durar (sempre é aplicado).
    pub max_circuit_duration: Duration,
    /// Bytes por circuito; `0` = sem limite.
    pub max_circuit_bytes: u64,
    pub reservation_duration: Duration,
}

impl Default for RelayLimits {
    fn default() -> Self {
        Self {
            max_reservations: 1024,
            max_circuits: 256,
            max_circuit_duration: Duration::from_secs(60 * 60),
            max_circuit_bytes: 0,
            reservation_duration: Duration::from_secs(60 * 60),
        }
    }
}

impl From<&RelayLimits> for relay::Config {
    fn from(limits: &RelayLimits) -> Self {
        relay::Config {
            max_reservations: limits.max_reservations,
            max_circuits: limits.max_circuits,
            max_circuit_duration: limits.max_circuit_duration,
            max_circuit_bytes: limits.max_circuit_bytes,
            reservation_duration: limits.reservation_duration,
            ..relay::Config::default()
        }
    }
}
