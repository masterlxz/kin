/// Erros do crate `transport`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("falha ao montar o transporte: {0}")]
    Build(String),
    #[error("endereço inválido: {0}")]
    Address(#[from] libp2p::multiaddr::Error),
    #[error("falha ao escutar: {0}")]
    Listen(#[from] libp2p::TransportError<std::io::Error>),
}
