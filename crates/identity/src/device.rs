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

    /// Serializa o certificado: `len16|identidade | len16|device | created_at u64 BE | len16|assinatura`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put(&mut out, &self.identity.public_key().encode_protobuf());
        put(&mut out, &self.device.encode_protobuf());
        out.extend_from_slice(&self.created_at.to_be_bytes());
        put(&mut out, &self.signature);
        out
    }

    /// Lê um certificado serializado por [`Self::to_bytes`]. Não confere a assinatura: use `verify`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut cur = bytes;
        let identity = PublicKey::try_decode_protobuf(take(&mut cur)?)?;
        let device = PublicKey::try_decode_protobuf(take(&mut cur)?)?;
        let created_at = u64::from_be_bytes(
            cur.get(..8)
                .ok_or(Error::InvalidCertificate)?
                .try_into()
                .map_err(|_| Error::InvalidCertificate)?,
        );
        cur = &cur[8..];
        let signature = take(&mut cur)?.to_vec();
        if !cur.is_empty() {
            return Err(Error::InvalidCertificate);
        }
        Ok(Self {
            identity: IdentityId::new(identity),
            device,
            created_at,
            signature,
        })
    }

    /// Peer ID do device certificado.
    pub fn device_peer_id(&self) -> PeerId {
        self.device.to_peer_id()
    }
}

fn put(out: &mut Vec<u8>, field: &[u8]) {
    let len = u16::try_from(field.len()).expect("campo do certificado maior que 64 KiB");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(field);
}

fn take<'a>(cur: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    let len = u16::from_be_bytes(
        cur.get(..2)
            .ok_or(Error::InvalidCertificate)?
            .try_into()
            .map_err(|_| Error::InvalidCertificate)?,
    ) as usize;
    let field = cur.get(2..2 + len).ok_or(Error::InvalidCertificate)?;
    *cur = &cur[2 + len..];
    Ok(field)
}

/// Monta uma [`PublicKey`] a partir dos 32 bytes de uma chave pública Ed25519.
pub fn public_key_from_ed25519(bytes: &[u8]) -> Result<PublicKey, Error> {
    let key = libp2p_identity::ed25519::PublicKey::try_from_bytes(bytes)
        .map_err(|_| Error::NotEd25519)?;
    Ok(libp2p_identity::PublicKey::from(key))
}

/// Extrai os 32 bytes de uma chave pública Ed25519.
pub fn ed25519_bytes(key: &PublicKey) -> Result<[u8; 32], Error> {
    Ok(key
        .clone()
        .try_into_ed25519()
        .map_err(|_| Error::NotEd25519)?
        .to_bytes())
}
