use libp2p_identity::{Keypair, PeerId, PublicKey};

use crate::{Error, IdentityId};

const CERT_DOMAIN: &[u8] = b"kin/device-cert/v1";

/// Chave de um device. O Peer ID de rede é derivado dela.
pub struct DeviceKey(Keypair);

impl DeviceKey {
    pub fn generate() -> Self {
        Self(Keypair::generate_ed25519())
    }

    pub fn public_key(&self) -> PublicKey {
        self.0.public()
    }

    pub fn peer_id(&self) -> PeerId {
        self.0.public().to_peer_id()
    }

    /// Acesso ao keypair para o transporte (libp2p) assinar o handshake.
    pub fn keypair(&self) -> &Keypair {
        &self.0
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.0.to_protobuf_encoding()?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let keypair = Keypair::from_protobuf_encoding(bytes)?;
        keypair
            .clone()
            .try_into_ed25519()
            .map_err(|_| Error::NotEd25519)?;
        Ok(Self(keypair))
    }
}

/// Prova assinada pela identidade de que um device lhe pertence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceCertificate {
    pub identity: IdentityId,
    pub device: PublicKey,
    pub created_at: u64,
    pub signature: Vec<u8>,
}

impl DeviceCertificate {
    /// Bytes que a identidade assina (com separador de domínio para evitar reuso da assinatura).
    pub(crate) fn signing_payload(device: &PublicKey, created_at: u64) -> Vec<u8> {
        let mut payload = CERT_DOMAIN.to_vec();
        payload.extend_from_slice(&device.encode_protobuf());
        payload.extend_from_slice(&created_at.to_be_bytes());
        payload
    }

    /// Confere a assinatura do certificado.
    pub fn verify(&self) -> Result<(), Error> {
        let payload = Self::signing_payload(&self.device, self.created_at);
        if self.identity.public_key().verify(&payload, &self.signature) {
            Ok(())
        } else {
            Err(Error::InvalidCertificate)
        }
    }

    /// Peer ID do device certificado.
    pub fn device_peer_id(&self) -> PeerId {
        self.device.to_peer_id()
    }
}
