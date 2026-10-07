use crate::Error;

/// O que trafega entre dois peers. O handshake é in-band: quem tem o menor Peer ID convida.
///
/// `Hello` (ambos, ao conectar) → o de maior Peer ID responde `KeyPackage` → o de menor cria a
/// conversa e manda `Welcome` → daí em diante só `Mls`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Envelope {
    Hello,
    KeyPackage(Vec<u8>),
    Welcome(Vec<u8>),
    Mls(Vec<u8>),
}

impl Envelope {
    pub(crate) fn encode(&self) -> Vec<u8> {
        let (tag, payload): (u8, &[u8]) = match self {
            Self::Hello => (0, &[]),
            Self::KeyPackage(b) => (1, b),
            Self::Welcome(b) => (2, b),
            Self::Mls(b) => (3, b),
        };
        let mut out = vec![tag];
        out.extend_from_slice(payload);
        out
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let (&tag, payload) = bytes.split_first().ok_or(Error::Malformed)?;
        Ok(match tag {
            0 if payload.is_empty() => Self::Hello,
            1 => Self::KeyPackage(payload.to_vec()),
            2 => Self::Welcome(payload.to_vec()),
            3 => Self::Mls(payload.to_vec()),
            _ => return Err(Error::Malformed),
        })
    }
}
