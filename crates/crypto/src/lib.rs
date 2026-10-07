//! E2EE via MLS (D2), atrás de uma interface própria: nenhum tipo do `openmls` aparece aqui.
//!
//! Uma conversa 1:1 é um grupo MLS de dois membros. Cada device tem uma chave de assinatura MLS
//! própria, certificada pela identidade (`DeviceCertificate`, D7) e carregada na credencial MLS.
//! Todas as mensagens saem como `Vec<u8>`, independentes do transporte (D1).
//!
//! Entrega at-least-once e sem ordem garantida (P9): duplicatas e mensagens velhas demais viram
//! erros tipados ([`Error::Duplicate`], [`Error::TooOld`]) para o chamador ignorar ou reagir.

mod conversation;
mod credential;
mod device;
mod error;

pub use conversation::{Conversation, Decrypted, Invite};
pub use device::CryptoDevice;
pub use error::Error;
