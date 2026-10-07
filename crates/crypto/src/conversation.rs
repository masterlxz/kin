use kin_identity::IdentityId;
use openmls::prelude::tls_codec::Deserialize as _;
use openmls::prelude::{
    KeyPackageIn, MlsGroup, MlsGroupCreateConfig, MlsMessageBodyIn, MlsMessageIn,
    ProcessedMessageContent, ProtocolVersion, SenderRatchetConfiguration, StagedWelcome,
};

use openmls_traits::OpenMlsProvider as _;

use crate::device::CIPHERSUITE;
use crate::{CryptoDevice, Error, credential};

/// Quantas gerações atrás (e à frente) o MLS tolera numa mesma época: cobre a entrega fora de ordem.
const OUT_OF_ORDER_TOLERANCE: u32 = 10;
const MAX_FORWARD_DISTANCE: u32 = 2000;
/// Épocas passadas mantidas para decifrar mensagens que chegam depois de um commit.
const MAX_PAST_EPOCHS: usize = 3;

fn config() -> MlsGroupCreateConfig {
    MlsGroupCreateConfig::builder()
        .ciphersuite(CIPHERSUITE)
        .use_ratchet_tree_extension(true)
        .sender_ratchet_configuration(SenderRatchetConfiguration::new(
            OUT_OF_ORDER_TOLERANCE,
            MAX_FORWARD_DISTANCE,
        ))
        .max_past_epochs(MAX_PAST_EPOCHS)
        .build()
}

/// Resultado de [`Conversation::invite`].
#[derive(Debug, Clone)]
pub struct Invite {
    /// Commit que leva os membros atuais à nova época (em 1:1, o convidante está sozinho: ninguém
    /// precisa dele, mas em grupos maiores os outros membros precisam).
    pub commit: Vec<u8>,
    /// Welcome para o convidado entrar na conversa.
    pub welcome: Vec<u8>,
}

/// O que uma mensagem recebida produziu.
// `EpochChanged` é raro e a diferença de tamanho é só a `IdentityId`; encaixotar não compensa.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decrypted {
    /// Mensagem de aplicação decifrada.
    Message {
        sender: IdentityId,
        plaintext: Vec<u8>,
    },
    /// Era um commit; a conversa avançou de época.
    EpochChanged,
}

/// Conversa E2EE (grupo MLS). Só um dos lados deve emitir commits por vez (P9).
pub struct Conversation {
    device: CryptoDevice,
    group: MlsGroup,
}

impl Conversation {
    /// Cria uma conversa nova, só com este device.
    pub fn create(device: &CryptoDevice) -> Result<Self, Error> {
        let inner = &device.0;
        let group = MlsGroup::new(
            &inner.provider,
            &inner.signer,
            &config(),
            inner.credential.clone(),
        )
        .map_err(Error::mls)?;
        Ok(Self {
            device: device.clone(),
            group,
        })
    }

    /// Entra numa conversa a partir de um Welcome. Confere a credencial de todos os membros;
    /// `expected_inviter` exige que alguém do grupo tenha essa identidade.
    pub fn join(
        device: &CryptoDevice,
        welcome: &[u8],
        expected_inviter: Option<&IdentityId>,
    ) -> Result<Self, Error> {
        let inner = &device.0;
        let MlsMessageBodyIn::Welcome(welcome) = MlsMessageIn::tls_deserialize_exact(welcome)
            .map_err(|_| Error::Malformed)?
            .extract()
        else {
            return Err(Error::Malformed);
        };
        let group =
            StagedWelcome::new_from_welcome(&inner.provider, config().join_config(), welcome, None)
                .map_err(|_| Error::NotForThisDevice)?
                .into_group(&inner.provider)
                .map_err(Error::mls)?;

        let mut found_inviter = expected_inviter.is_none();
        for member in group.members() {
            let id = credential::verify(&member.credential, &member.signature_key, None)?;
            found_inviter |= Some(&id) == expected_inviter;
        }
        if !found_inviter {
            return Err(Error::UnexpectedIdentity);
        }
        Ok(Self {
            device: device.clone(),
            group,
        })
    }

    /// Identificador da conversa (igual nos dois lados).
    pub fn id(&self) -> Vec<u8> {
        self.group.group_id().as_slice().to_vec()
    }

    /// Época atual do grupo; sobe a cada commit.
    pub fn epoch(&self) -> u64 {
        self.group.epoch().as_u64()
    }

    /// Identidades dos outros membros.
    pub fn peers(&self) -> Result<Vec<IdentityId>, Error> {
        let own = self.group.own_leaf_index();
        self.group
            .members()
            .filter(|m| m.index != own)
            .map(|m| credential::identity_of(&m.credential))
            .collect()
    }

    /// Convida o dono do KeyPackage. Valida o KeyPackage e a credencial dele antes de aceitar;
    /// `expected_identity` exige uma identidade específica.
    pub fn invite(
        &mut self,
        key_package: &[u8],
        expected_identity: Option<&IdentityId>,
    ) -> Result<Invite, Error> {
        let inner = &self.device.0;
        let key_package = KeyPackageIn::tls_deserialize_exact(key_package)
            .map_err(|_| Error::Malformed)?
            .validate(inner.provider.crypto(), ProtocolVersion::Mls10)
            .map_err(|_| Error::Malformed)?;
        credential::verify(
            key_package.leaf_node().credential(),
            key_package.leaf_node().signature_key().as_slice(),
            expected_identity,
        )?;

        let (commit, welcome, _) = self
            .group
            .add_members(
                &inner.provider,
                &inner.signer,
                core::slice::from_ref(&key_package),
            )
            .map_err(Error::mls)?;
        self.group
            .merge_pending_commit(&inner.provider)
            .map_err(Error::mls)?;
        Ok(Invite {
            commit: commit.to_bytes().map_err(Error::mls)?,
            welcome: welcome.to_bytes().map_err(Error::mls)?,
        })
    }

    /// Renova as chaves do grupo (post-compromise security). Devolve o commit para os outros
    /// membros, que o aplicam com [`Self::decrypt`].
    pub fn rotate_keys(&mut self) -> Result<Vec<u8>, Error> {
        let inner = &self.device.0;
        let bundle = self
            .group
            .self_update(
                &inner.provider,
                &inner.signer,
                openmls::prelude::LeafNodeParameters::default(),
            )
            .map_err(Error::mls)?;
        self.group
            .merge_pending_commit(&inner.provider)
            .map_err(Error::mls)?;
        bundle.into_commit().to_bytes().map_err(Error::mls)
    }

    /// Cifra `plaintext` para os outros membros.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
        let inner = &self.device.0;
        self.group
            .create_message(&inner.provider, &inner.signer, plaintext)
            .map_err(Error::mls)?
            .to_bytes()
            .map_err(Error::mls)
    }

    /// Processa uma mensagem recebida (aplicação ou commit). Idempotente: reentregas viram
    /// [`Error::Duplicate`], sem alterar o estado.
    pub fn decrypt(&mut self, bytes: &[u8]) -> Result<Decrypted, Error> {
        let inner = &self.device.0;
        let message = MlsMessageIn::tls_deserialize_exact(bytes)
            .map_err(|_| Error::Malformed)?
            .try_into_protocol_message()
            .map_err(|_| Error::Malformed)?;
        let processed = self
            .group
            .process_message(&inner.provider, message)
            .map_err(map_process_error)?;

        let sender = credential::identity_of(processed.credential())?;
        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(m) => Ok(Decrypted::Message {
                sender,
                plaintext: m.into_bytes(),
            }),
            ProcessedMessageContent::StagedCommitMessage(staged) => {
                for add in staged.add_proposals() {
                    let leaf = add.add_proposal().key_package().leaf_node();
                    credential::verify(leaf.credential(), leaf.signature_key().as_slice(), None)?;
                }
                self.group
                    .merge_staged_commit(&inner.provider, *staged)
                    .map_err(Error::mls)?;
                Ok(Decrypted::EpochChanged)
            }
            _ => Err(Error::Unsupported),
        }
    }
}

fn map_process_error<S: std::error::Error>(e: openmls::prelude::ProcessMessageError<S>) -> Error {
    use openmls::prelude::ProcessMessageError as P;
    use openmls::prelude::ValidationError as V;
    match &e {
        P::ValidationError(V::UnableToDecrypt(inner)) => {
            let text = inner.to_string();
            if text.contains("deleted to preserve forward secrecy") {
                Error::Duplicate
            } else if text.contains("too old") {
                Error::TooOld
            } else {
                Error::mls(e)
            }
        }
        P::ValidationError(V::NoPastEpochData) => Error::TooOld,
        P::ValidationError(V::WrongGroupId) => Error::WrongConversation,
        P::ValidationError(V::WrongEpoch) => Error::UnknownEpoch,
        _ => Error::mls(e),
    }
}
