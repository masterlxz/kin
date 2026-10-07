//! Conexões entre peers e abstração de mailbox (D1, D3).
//!
//! Fases 1.3 e 1.4: um [`Node`] que escuta em TCP (Noise + Yamux), descobre peers na LAN via mDNS
//! (opcional), conecta por endereço quando o IP é alcançável e troca `identify` com os peers.

mod error;
mod node;

pub use error::Error;
pub use libp2p::{Multiaddr, PeerId};
pub use node::{Node, NodeConfig, NodeEvent};
