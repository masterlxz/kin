use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use kin_crypto::{Conversation, CryptoDevice, Decrypted, Error as CryptoError};
use kin_identity::{DeviceKey, IdentityId, IdentityProvider};
use kin_transport::{Multiaddr, Node, NodeConfig, NodeEvent, PeerId, SendId};

use crate::wire::Envelope;
use crate::{Error, Message, MessageId};

/// O que o [`Chat`] avisa à aplicação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Listening(Multiaddr),
    /// Conexão de rede aberta; o handshake E2EE começa sozinho.
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    /// Conversa E2EE pronta: já dá para enviar mensagens a este peer.
    ConversationReady {
        peer: PeerId,
        identity: IdentityId,
    },
    /// Mensagem recebida e decifrada.
    Received {
        peer: PeerId,
        sender: IdentityId,
        message: Message,
    },
    /// O peer confirmou o recebimento.
    Delivered {
        peer: PeerId,
        message: MessageId,
    },
    /// O envio falhou; a aplicação decide se reenvia.
    SendFailed {
        peer: PeerId,
        message: Option<MessageId>,
        reason: String,
    },
    /// Algo do peer foi descartado (malformado, de outra conversa, etc.).
    Dropped {
        peer: PeerId,
        reason: String,
    },
}

/// Chat 1:1: liga o transporte (`kin-transport`) à conversa cifrada (`kin-crypto`).
///
/// Uma conversa por peer, em memória (P14). A ligação entre o Peer ID e a identidade MLS do
/// peer ainda não é provada (P15): `identity` vem do certificado MLS, não da conexão.
pub struct Chat {
    node: Node,
    device: CryptoDevice,
    conversations: HashMap<PeerId, Conversation>,
    pending: HashMap<SendId, Option<MessageId>>,
    /// Eventos já produzidos mas ainda não entregues, em ordem.
    queue: std::collections::VecDeque<ChatEvent>,
}

impl Chat {
    /// `device_key` fornece o Peer ID da rede; a chave MLS do device é gerada e certificada por
    /// `identity`.
    pub fn new(
        identity: &dyn IdentityProvider,
        device_key: &DeviceKey,
        config: NodeConfig,
    ) -> Result<Self, Error> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Ok(Self {
            node: Node::new(device_key.keypair().clone(), config)?,
            device: CryptoDevice::new(identity, created_at)?,
            conversations: HashMap::new(),
            pending: HashMap::new(),
            queue: Default::default(),
        })
    }

    pub fn peer_id(&self) -> PeerId {
        self.node.peer_id()
    }

    pub fn listen(&mut self) -> Result<(), Error> {
        Ok(self.node.listen()?)
    }

    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<(), Error> {
        Ok(self.node.listen_on(addr)?)
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), Error> {
        Ok(self.node.dial(addr)?)
    }

    /// Há conversa E2EE pronta com o peer?
    pub fn is_ready(&self, peer: &PeerId) -> bool {
        self.conversations.contains_key(peer)
    }

    /// Cifra e envia `text` ao peer; `parent` faz a mensagem responder a outra (thread). A entrega
    /// chega depois como [`ChatEvent::Delivered`] ou [`ChatEvent::SendFailed`].
    pub fn send(
        &mut self,
        peer: PeerId,
        text: impl Into<String>,
        parent: Option<MessageId>,
    ) -> Result<MessageId, Error> {
        let conversation = self
            .conversations
            .get_mut(&peer)
            .ok_or(Error::NoConversation(peer))?;
        let message = Message::new(text.into(), parent);
        let wire = conversation.encrypt(&message.encode())?;
        let id = self.node.send(peer, Envelope::Mls(wire).encode());
        self.pending.insert(id, Some(message.id));
        Ok(message.id)
    }

    /// Avança o chat até o próximo evento relevante para a aplicação.
    pub async fn next_event(&mut self) -> ChatEvent {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return event;
            }
            let event = self.node.next_event().await;
            self.handle(event);
        }
    }

    fn handle(&mut self, event: NodeEvent) {
        match event {
            NodeEvent::Listening(addr) => self.queue.push_back(ChatEvent::Listening(addr)),
            NodeEvent::PeerConnected(peer) => {
                self.queue.push_back(ChatEvent::PeerConnected(peer));
                self.transmit(peer, &Envelope::Hello);
            }
            NodeEvent::PeerDisconnected(peer) => {
                self.queue.push_back(ChatEvent::PeerDisconnected(peer));
            }
            NodeEvent::MessageReceived { peer, data } => match Envelope::decode(&data) {
                Ok(envelope) => self.on_envelope(peer, envelope),
                Err(e) => self.drop_message(peer, e),
            },
            NodeEvent::MessageDelivered { peer, id } => {
                if let Some(Some(message)) = self.pending.remove(&id) {
                    self.queue.push_back(ChatEvent::Delivered { peer, message });
                }
            }
            NodeEvent::SendFailed { peer, id, reason } => {
                let message = self.pending.remove(&id).flatten();
                self.queue.push_back(ChatEvent::SendFailed {
                    peer,
                    message,
                    reason,
                });
            }
            NodeEvent::PeerDiscovered(..)
            | NodeEvent::PeerIdentified { .. }
            | NodeEvent::DialFailed { .. } => {}
        }
    }

    fn on_envelope(&mut self, peer: PeerId, envelope: Envelope) {
        let result = match envelope {
            Envelope::Hello => self.on_hello(peer),
            Envelope::KeyPackage(kp) => self.on_key_package(peer, &kp),
            Envelope::Welcome(welcome) => self.on_welcome(peer, &welcome),
            Envelope::Mls(wire) => self.on_mls(peer, &wire),
        };
        if let Err(e) = result {
            self.drop_message(peer, e);
        }
    }

    /// Só o de maior Peer ID responde com KeyPackage; o de menor convida (evita convite duplo).
    fn on_hello(&mut self, peer: PeerId) -> Result<(), Error> {
        if !self.conversations.contains_key(&peer) && self.peer_id() > peer {
            let kp = self.device.key_package()?;
            self.transmit(peer, &Envelope::KeyPackage(kp));
        }
        Ok(())
    }

    fn on_key_package(&mut self, peer: PeerId, key_package: &[u8]) -> Result<(), Error> {
        if self.conversations.contains_key(&peer) || self.peer_id() > peer {
            return Ok(());
        }
        let mut conversation = Conversation::create(&self.device)?;
        let invite = conversation.invite(key_package, None)?;
        self.transmit(peer, &Envelope::Welcome(invite.welcome));
        self.ready(peer, conversation)
    }

    fn on_welcome(&mut self, peer: PeerId, welcome: &[u8]) -> Result<(), Error> {
        if self.conversations.contains_key(&peer) || self.peer_id() < peer {
            return Ok(());
        }
        let conversation = Conversation::join(&self.device, welcome, None)?;
        self.ready(peer, conversation)
    }

    fn ready(&mut self, peer: PeerId, conversation: Conversation) -> Result<(), Error> {
        let identity = conversation
            .peers()?
            .into_iter()
            .next()
            .ok_or(Error::Malformed)?;
        self.conversations.insert(peer, conversation);
        self.queue
            .push_back(ChatEvent::ConversationReady { peer, identity });
        Ok(())
    }

    fn on_mls(&mut self, peer: PeerId, wire: &[u8]) -> Result<(), Error> {
        let conversation = self
            .conversations
            .get_mut(&peer)
            .ok_or(Error::NoConversation(peer))?;
        match conversation.decrypt(wire) {
            Ok(Decrypted::Message { sender, plaintext }) => {
                let message = Message::decode(&plaintext)?;
                self.queue.push_back(ChatEvent::Received {
                    peer,
                    sender,
                    message,
                });
                Ok(())
            }
            Ok(Decrypted::EpochChanged) => Ok(()),
            // Reentrega (at-least-once) ou velha demais: seguro ignorar.
            Err(CryptoError::Duplicate | CryptoError::TooOld) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn transmit(&mut self, peer: PeerId, envelope: &Envelope) {
        let id = self.node.send(peer, envelope.encode());
        self.pending.insert(id, None);
    }

    fn drop_message(&mut self, peer: PeerId, error: Error) {
        self.queue.push_back(ChatEvent::Dropped {
            peer,
            reason: error.to_string(),
        });
    }
}
