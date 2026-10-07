/// Erros do crate `identity`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("falha ao assinar: {0}")]
    Signing(#[from] libp2p_identity::SigningError),
    #[error("chave inválida: {0}")]
    Decoding(#[from] libp2p_identity::DecodingError),
    #[error("a chave não é Ed25519")]
    NotEd25519,
    #[error("certificado de device inválido")]
    InvalidCertificate,
    #[error("erro de E/S: {0}")]
    Io(#[from] std::io::Error),
}
