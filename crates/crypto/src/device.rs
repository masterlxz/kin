use std::sync::Arc;

use kin_identity::{IdentityId, IdentityProvider, public_key_from_ed25519};
use openmls::prelude::tls_codec::Serialize as _;
use openmls::prelude::{
    BasicCredential, Ciphersuite, CredentialWithKey, KeyPackage, SignatureScheme,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider as _;

use crate::Error;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) struct Inner {
    pub(crate) provider: OpenMlsRustCrypto,
    pub(crate) signer: SignatureKeyPair,
    pub(crate) credential: CredentialWithKey,
    pub(crate) identity: IdentityId,
}

/// Material criptográfico de um device: chave de assinatura MLS própria, certificada pela
/// identidade, e o estado (em memória) de todas as suas conversas. Clonar é barato.
#[derive(Clone)]
pub struct CryptoDevice(pub(crate) Arc<Inner>);

impl CryptoDevice {
    /// Gera a chave MLS do device e a certifica com `identity` (`created_at` em segundos Unix).
    pub fn new(identity: &dyn IdentityProvider, created_at: u64) -> Result<Self, Error> {
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(SignatureScheme::ED25519).map_err(Error::mls)?;
        signer.store(provider.storage()).map_err(Error::mls)?;

        let device_key = public_key_from_ed25519(signer.public())?;
        let certificate = identity.authorize_device(&device_key, created_at)?;
        let credential = CredentialWithKey {
            credential: BasicCredential::new(certificate.to_bytes()).into(),
            signature_key: signer.public().into(),
        };
        Ok(Self(Arc::new(Inner {
            provider,
            signer,
            credential,
            identity: identity.id(),
        })))
    }

    /// Identidade a quem este device pertence.
    pub fn identity(&self) -> &IdentityId {
        &self.0.identity
    }

    /// Gera um KeyPackage de uso único, para quem quiser convidar este device.
    pub fn key_package(&self) -> Result<Vec<u8>, Error> {
        let bundle = KeyPackage::builder()
            .build(
                CIPHERSUITE,
                &self.0.provider,
                &self.0.signer,
                self.0.credential.clone(),
            )
            .map_err(Error::mls)?;
        bundle
            .key_package()
            .tls_serialize_detached()
            .map_err(Error::mls)
    }
}
