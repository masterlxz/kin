//! Conexões entre peers e abstração de mailbox (D1, D3).
//!
//! Fases 1.3 e 1.4: um [`Node`] que escuta em TCP (Noise + Yamux), descobre peers na LAN via mDNS
//! (opcional), conecta por endereço quando o IP é alcançável e troca `identify` com os peers.
//!
//! Fase 1.6: [`Node::send`] entrega bytes opacos (já cifrados pelo `crypto`) a um peer conectado,
//! com ack de recebimento.

mod codec;
mod error;
mod node;

pub use codec::MAX_MESSAGE_SIZE;
pub use error::Error;
pub use libp2p::{Multiaddr, PeerId};
pub use node::{Node, NodeConfig, NodeEvent, SendId};
