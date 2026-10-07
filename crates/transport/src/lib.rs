//! Conexões entre peers e abstração de mailbox (D1, D3).
//!
//! Fase 1.3: um [`Node`] que escuta em TCP (Noise + Yamux), descobre peers na LAN via mDNS e
//! conecta neles automaticamente.

mod error;
mod node;

pub use error::Error;
pub use libp2p::{Multiaddr, PeerId};
pub use node::{Node, NodeEvent};
