//! Chat 1:1 (Fase 1.6): formato de mensagem com `parent_message_id` (D4) e o fluxo que liga
//! `crypto` e `transport`.
//!
//! Estado MLS só em memória (P14): reiniciar o app perde as conversas.

mod chat;
mod error;
mod message;
mod wire;

pub use chat::{Chat, ChatEvent};
pub use error::Error;
pub use message::{Message, MessageId};
