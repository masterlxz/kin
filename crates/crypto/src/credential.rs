use kin_identity::{DeviceCertificate, IdentityId, ed25519_bytes};
use openmls::prelude::{BasicCredential, Credential};

use crate::Error;

/// Confere a credencial MLS e devolve a identidade a quem ela pertence.
///
/// A credencial carrega um `DeviceCertificate`. Aceitamos se (1) a assinatura do certificado vale,
/// (2) o certificado cobre exatamente a chave de assinatura MLS do membro (`signature_key`) e
/// (3) a identidade é a esperada, quando o chamador sabe qual é.
pub(crate) fn verify(
    credential: &Credential,
    signature_key: &[u8],
    expected: Option<&IdentityId>,
) -> Result<IdentityId, Error> {
    let cert = parse(credential)?;
    if ed25519_bytes(&cert.device)? != signature_key {
        return Err(Error::InvalidCredential);
    }
    if let Some(expected) = expected
        && expected != &cert.identity
    {
        return Err(Error::UnexpectedIdentity);
    }
    Ok(cert.identity)
}

/// Lê a identidade de uma credencial já validada pelo MLS (sem rechecar a chave do membro).
pub(crate) fn identity_of(credential: &Credential) -> Result<IdentityId, Error> {
    Ok(parse(credential)?.identity)
}

fn parse(credential: &Credential) -> Result<DeviceCertificate, Error> {
    let basic =
        BasicCredential::try_from(credential.clone()).map_err(|_| Error::InvalidCredential)?;
    let cert =
        DeviceCertificate::from_bytes(basic.identity()).map_err(|_| Error::InvalidCredential)?;
    cert.verify().map_err(|_| Error::InvalidCredential)?;
    Ok(cert)
}

#[cfg(test)]
mod tests {
    use kin_identity::{DeviceKey, IdentityProvider, StandaloneIdentity};

    use super::*;

    fn credential_for(cert: &DeviceCertificate) -> Credential {
        BasicCredential::new(cert.to_bytes()).into()
    }

    #[test]
    fn accepts_a_valid_certificate_that_covers_the_signing_key() {
        let id = StandaloneIdentity::generate();
        let key = DeviceKey::generate().public_key();
        let cert = id.authorize_device(&key, 1).unwrap();
        let raw = ed25519_bytes(&key).unwrap();

        assert_eq!(verify(&credential_for(&cert), &raw, None).unwrap(), id.id());
    }

    #[test]
    fn rejects_a_certificate_that_covers_a_different_key() {
        let id = StandaloneIdentity::generate();
        let certified = DeviceKey::generate().public_key();
        let cert = id.authorize_device(&certified, 1).unwrap();
        let other = ed25519_bytes(&DeviceKey::generate().public_key()).unwrap();

        assert!(matches!(
            verify(&credential_for(&cert), &other, None),
            Err(Error::InvalidCredential)
        ));
    }

    #[test]
    fn rejects_a_forged_or_garbage_credential() {
        let id = StandaloneIdentity::generate();
        let key = DeviceKey::generate().public_key();
        let mut cert = id.authorize_device(&key, 1).unwrap();
        // Alguém troca a identidade por outra, mantendo a assinatura original.
        cert.identity = StandaloneIdentity::generate().id();
        let raw = ed25519_bytes(&key).unwrap();

        assert!(matches!(
            verify(&credential_for(&cert), &raw, None),
            Err(Error::InvalidCredential)
        ));
        let garbage: Credential = BasicCredential::new(b"lixo".to_vec()).into();
        assert!(matches!(
            verify(&garbage, &raw, None),
            Err(Error::InvalidCredential)
        ));
    }
}
