use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use kin_crypto::{Conversation, CryptoDevice, Decrypted, Error as CryptoError, Expected};
use kin_identity::{DeviceKey, IdentityId, IdentityProvider};
use kin_transport::{Multiaddr, NatStatus, Node, NodeConfig, NodeEvent, PeerId, SendId};

use crate::wire::Envelope;
use crate::{Error, Message, MessageId};

/// O que o [`Chat`] avisa à aplicação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Listening(Multiaddr),
    /// Conexão de rede aberta; o handshake E2EE começa sozinho.
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    /// Uma tentativa de conexão falhou (endereço fora do ar, Peer ID diferente do esperado, relay
    /// sem reserva do destino, etc.).
    DialFailed {
        peer: Option<PeerId>,
        reason: String,
    },
    /// Por onde a conexão com o peer passa: direta ou via relay. Chega de novo quando o hole punching
    /// troca o relay por uma conexão direta (a conversa não precisa de novo handshake).
    Route {
        peer: PeerId,
        relayed: bool,
    },
    /// O AutoNAT mudou a avaliação de alcançabilidade deste nó.
    Nat(NatStatus),
    /// O relay aceitou a reserva: este nó é alcançável pelo endereço de circuito.
    RelayReserved {
        relay: PeerId,
    },
    /// Resultado de uma tentativa de hole punching com o peer.
    HolePunch {
        peer: PeerId,
        result: Result<(), String>,
    },
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
/// Uma conversa por peer; persistente com [`Chat::open`], em memória com [`Chat::new`]. O
/// certificado do device cobre a chave MLS e a chave de rede (P15): o KeyPackage e o Welcome só são
/// aceitos se o Peer ID da conexão for o certificado nele, então `identity` é de quem está na linha.
pub struct Chat {
    node: Node,
    device: CryptoDevice,
    conversations: HashMap<PeerId, Conversation>,
    pending: HashMap<SendId, Option<MessageId>>,
    /// Eventos já produzidos mas ainda não entregues, em ordem.
    queue: std::collections::VecDeque<ChatEvent>,
}

impl Chat {
    /// Chat só em memória: `device_key` fornece o Peer ID da rede; a chave MLS do device é gerada
    /// e certificada por `identity`. Nada sobrevive ao processo.
    pub fn new(
        identity: &dyn IdentityProvider,
        device_key: &DeviceKey,
        config: NodeConfig,
    ) -> Result<Self, Error> {
        let device = CryptoDevice::new(identity, &device_key.public_key(), now_secs())?;
        Self::with_device(device, device_key, config)
    }

    /// Chat persistente: o estado MLS e a lista de conversas ficam no banco SQLite `db`. Ao
    /// reabrir, as conversas voltam sozinhas e já aceitam mensagens, sem novo handshake.
    pub fn open(
        identity: &dyn IdentityProvider,
        device_key: &DeviceKey,
        config: NodeConfig,
        db: &Path,
    ) -> Result<Self, Error> {
        let device = CryptoDevice::open(identity, &device_key.public_key(), db, now_secs())?;
        let mut chat = Self::with_device(device, device_key, config)?;
        for (label, id) in chat.device.remembered()? {
            // Rótulo ou conversa que não carregam mais são ignorados: o handshake refaz.
            let Ok(peer) = label.parse::<PeerId>() else {
                continue;
            };
            if let Some(conversation) = Conversation::load(&chat.device, &id)? {
                // O rótulo salvo tem de bater com o Peer ID certificado do outro membro.
                if conversation.peer_ids()? == [peer] {
                    chat.conversations.insert(peer, conversation);
                }
            }
        }
        Ok(chat)
    }

    fn with_device(
        device: CryptoDevice,
        device_key: &DeviceKey,
        config: NodeConfig,
    ) -> Result<Self, Error> {
        Ok(Self {
            node: Node::new(device_key.keypair().clone(), config)?,
            device,
            conversations: HashMap::new(),
            pending: HashMap::new(),
            queue: Default::default(),
        })
    }

    /// Peers com conversa pronta (inclusive as recarregadas do banco).
    pub fn ready_peers(&self) -> Vec<PeerId> {
        self.conversations.keys().copied().collect()
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

    /// Conecta direto ou, se `addr` for um endereço de circuito (`.../p2p-circuit/p2p/<peer>`), por
    /// um relay.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), Error> {
        Ok(self.node.dial(addr)?)
    }

    /// Endereço pelo qual outros discam este nó através do relay `relay_addr`.
    pub fn circuit_address(&self, relay_addr: &Multiaddr) -> Multiaddr {
        self.node.circuit_address(relay_addr)
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
                self.announce_if_ready(peer);
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
            NodeEvent::PeerRoute { peer, relayed } => {
                self.queue.push_back(ChatEvent::Route { peer, relayed });
            }
            NodeEvent::NatStatus(status) => self.queue.push_back(ChatEvent::Nat(status)),
            NodeEvent::RelayReserved { relay } => {
                self.queue.push_back(ChatEvent::RelayReserved { relay });
            }
            NodeEvent::HolePunch { peer, result } => {
                self.queue.push_back(ChatEvent::HolePunch { peer, result });
            }
            NodeEvent::DialFailed { peer, reason } => {
                self.queue.push_back(ChatEvent::DialFailed { peer, reason });
            }
            NodeEvent::PeerDiscovered(..) | NodeEvent::PeerIdentified { .. } => {}
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
        let invite = conversation.invite(
            key_package,
            &Expected {
                identity: None,
                peer: Some(&peer),
            },
        )?;
        self.transmit(peer, &Envelope::Welcome(invite.welcome));
        self.ready(peer, conversation)
    }

    fn on_welcome(&mut self, peer: PeerId, welcome: &[u8]) -> Result<(), Error> {
        if self.conversations.contains_key(&peer) || self.peer_id() < peer {
            return Ok(());
        }
        let conversation = Conversation::join(
            &self.device,
            welcome,
            &Expected {
                identity: None,
                peer: Some(&peer),
            },
        )?;
        self.ready(peer, conversation)
    }

    fn ready(&mut self, peer: PeerId, conversation: Conversation) -> Result<(), Error> {
        self.device
            .remember(&peer.to_string(), &conversation.id())?;
        self.conversations.insert(peer, conversation);
        self.announce_if_ready(peer);
        Ok(())
    }

    /// Avisa a aplicação que a conversa com `peer` está pronta (nova ou recarregada do banco).
    fn announce_if_ready(&mut self, peer: PeerId) {
        let identity = self
            .conversations
            .get(&peer)
            .and_then(|c| c.peers().ok())
            .and_then(|peers| peers.into_iter().next());
        if let Some(identity) = identity {
            self.queue
                .push_back(ChatEvent::ConversationReady { peer, identity });
        }
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use kin_identity::StandaloneIdentity;

    use super::*;

    fn chat(identity: &StandaloneIdentity, device: &DeviceKey) -> Chat {
        Chat::new(
            identity,
            device,
            NodeConfig {
                mdns: false,
                ..Default::default()
            },
        )
        .unwrap()
    }

    /// P15: o KeyPackage legítimo de Bia, entregue por outro Peer ID, não vira conversa.
    #[tokio::test]
    async fn key_package_from_another_peer_is_refused() {
        let (id_a, id_b) = (
            StandaloneIdentity::generate(),
            StandaloneIdentity::generate(),
        );
        // `on_key_package` só age no lado de menor Peer ID: sorteia até Ana ser a menor.
        let (dev_a, dev_b, dev_m) = loop {
            let keys = (
                DeviceKey::generate(),
                DeviceKey::generate(),
                DeviceKey::generate(),
            );
            if keys.0.peer_id() < keys.1.peer_id() && keys.0.peer_id() < keys.2.peer_id() {
                break keys;
            }
        };
        let mut ana = chat(&id_a, &dev_a);
        let bia_kp = CryptoDevice::new(&id_b, &dev_b.public_key(), 1)
            .unwrap()
            .key_package()
            .unwrap();

        // M (um peer qualquer, conectado a Ana) repassa o pacote da Bia como se fosse dele.
        let err = ana.on_key_package(dev_m.peer_id(), &bia_kp).unwrap_err();
        assert!(
            matches!(err, Error::Crypto(CryptoError::UnexpectedPeer)),
            "{err:?}"
        );
        assert!(!ana.is_ready(&dev_m.peer_id()) && !ana.is_ready(&dev_b.peer_id()));

        // Vindo da própria Bia, o mesmo pacote é aceito.
        ana.on_key_package(dev_b.peer_id(), &bia_kp).unwrap();
        assert!(ana.is_ready(&dev_b.peer_id()));
    }
}
