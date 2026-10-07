use kin_transport::PeerId;

/// Erros do crate `chat`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("criptografia: {0}")]
    Crypto(#[from] kin_crypto::Error),
    #[error("transporte: {0}")]
    Transport(#[from] kin_transport::Error),
    /// Ainda não há conversa E2EE com o peer (o handshake não terminou ou ele não está conectado).
    #[error("sem conversa com o peer {0}")]
    NoConversation(PeerId),
    /// Bytes que não seguem o formato do Kin.
    #[error("mensagem malformada")]
    Malformed,
}
