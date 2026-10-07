//! CLI de desenvolvimento: sobe um nó Kin e conversa 1:1 pelo terminal.
//!
//! ```text
//! kin [--data-dir DIR] [--listen ADDR] [--dial ADDR] [--relay ADDR]... [--external-addr ADDR]...
//!     [--no-mdns]
//! kin --serve-relay [--listen ADDR] [--external-addr ADDR]... [--relay-max-bytes N]
//!     [--relay-max-circuits N]
//! ```
//!
//! Atrás de NAT, `--relay ADDR` (que termina em `/p2p/<relay>`) reserva um lugar no relay e imprime o
//! endereço de circuito que o outro lado deve passar a `--dial`. `--serve-relay` sobe só um relay
//! (sem chat). Na LAN, duas instâncias se acham sozinhas via mDNS. Linhas digitadas vão ao peer da conversa;
//! `/r <id> texto` responde à mensagem `<id>` (thread); `/quit` sai.

use std::path::{Path, PathBuf};

use kin_chat::{Chat, ChatEvent, Message, MessageId};
use kin_identity::{DeviceKey, StandaloneIdentity};
use kin_transport::{Multiaddr, Node, NodeConfig, NodeEvent, PeerId, RelayLimits};
use tokio::io::{AsyncBufReadExt, BufReader};

struct Args {
    data_dir: PathBuf,
    listen: Option<Multiaddr>,
    dial: Option<Multiaddr>,
    relays: Vec<Multiaddr>,
    external_addrs: Vec<Multiaddr>,
    serve_relay: bool,
    relay_limits: RelayLimits,
    mdns: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        data_dir: PathBuf::from(".kin"),
        listen: None,
        dial: None,
        relays: Vec::new(),
        external_addrs: Vec::new(),
        serve_relay: false,
        relay_limits: RelayLimits::default(),
        mdns: true,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} precisa de um valor"));
        match flag.as_str() {
            "--data-dir" => args.data_dir = value()?.into(),
            "--listen" => args.listen = Some(value()?.parse().map_err(|e| format!("{e}"))?),
            "--dial" => args.dial = Some(value()?.parse().map_err(|e| format!("{e}"))?),
            "--relay" => args
                .relays
                .push(value()?.parse().map_err(|e| format!("{e}"))?),
            "--external-addr" => args
                .external_addrs
                .push(value()?.parse().map_err(|e| format!("{e}"))?),
            "--serve-relay" => args.serve_relay = true,
            "--relay-max-bytes" => {
                args.relay_limits.max_circuit_bytes =
                    value()?.parse().map_err(|e| format!("{e}"))?;
            }
            "--relay-max-circuits" => {
                args.relay_limits.max_circuits = value()?.parse().map_err(|e| format!("{e}"))?;
            }
            "--no-mdns" => args.mdns = false,
            other => return Err(format!("argumento desconhecido: {other}")),
        }
    }
    Ok(args)
}

/// Carrega a identidade e a chave de device do diretório, criando-as na primeira execução.
/// Sem cifra em repouso (P13): só para desenvolvimento.
fn load_keys(dir: &Path) -> Result<(StandaloneIdentity, DeviceKey), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let identity_path = dir.join("identity.key");
    let device_path = dir.join("device.key");
    let identity = if identity_path.exists() {
        StandaloneIdentity::load(&identity_path)?
    } else {
        let identity = StandaloneIdentity::generate();
        identity.save(&identity_path)?;
        identity
    };
    let device = if device_path.exists() {
        DeviceKey::from_bytes(&std::fs::read(&device_path)?)?
    } else {
        let device = DeviceKey::generate();
        write_private(&device_path, &device.to_bytes()?)?;
        device
    };
    Ok((identity, device))
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?
        .write_all(bytes)
}

fn short(id: &MessageId) -> String {
    id.to_string()[..8].to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(|e| {
        eprintln!(
            "{e}\nuso: kin [--data-dir DIR] [--listen ADDR] [--dial ADDR] [--relay ADDR]... [--no-mdns]\n     kin --serve-relay [--listen ADDR] [--relay-max-bytes N] [--relay-max-circuits N]"
        );
        e
    })?;
    let (identity, device) = load_keys(&args.data_dir)?;
    if args.serve_relay {
        return serve_relay(&args, &device).await;
    }
    let config = NodeConfig {
        mdns: args.mdns,
        relays: args.relays.clone(),
        external_addrs: args.external_addrs.clone(),
        ..Default::default()
    };
    let mut chat = Chat::open(
        &identity,
        &device,
        config,
        &args.data_dir.join("mls.sqlite"),
    )?;
    println!(
        "kin {} — peer id: {}",
        env!("CARGO_PKG_VERSION"),
        chat.peer_id()
    );

    match args.listen {
        Some(addr) => chat.listen_on(addr)?,
        None => chat.listen()?,
    }
    for relay in &args.relays {
        println!("alcançável por relay em {}", chat.circuit_address(relay));
    }
    // Com relays, o dial espera a reserva: discar um endereço de circuito enquanto a conexão com o
    // relay ainda está sendo aberta para a reserva faz o libp2p cancelar o dial.
    let mut pending_dial = args.dial.clone();
    if args.relays.is_empty()
        && let Some(addr) = pending_dial.take()
    {
        chat.dial(addr)?;
    }

    let mut seen: Vec<MessageId> = Vec::new();
    let mut current: Option<PeerId> = None;
    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    loop {
        tokio::select! {
            event = chat.next_event() => {
                if matches!(event, ChatEvent::RelayReserved { .. })
                    && let Some(addr) = pending_dial.take()
                    && let Err(e) = chat.dial(addr)
                {
                    println!("(falha ao discar: {e})");
                }
                on_event(event, &mut current, &mut seen);
            }
            line = lines.next_line() => {
                let Some(line) = line? else { break };
                let line = line.trim();
                if line == "/quit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                let Some(peer) = current else {
                    println!("(sem conversa pronta ainda)");
                    continue;
                };
                let (parent, text) = match line.strip_prefix("/r ") {
                    Some(rest) => {
                        let (prefix, text) = rest.split_once(' ').unwrap_or((rest, ""));
                        match seen.iter().find(|id| id.to_string().starts_with(prefix)) {
                            Some(id) => (Some(*id), text),
                            None => {
                                println!("(mensagem {prefix} desconhecida)");
                                continue;
                            }
                        }
                    }
                    None => (None, line),
                };
                match chat.send(peer, text, parent) {
                    Ok(id) => {
                        seen.push(id);
                        println!("[{}] você: {text}", short(&id));
                    }
                    Err(e) => println!("(falha ao enviar: {e})"),
                }
            }
        }
    }
    Ok(())
}

/// Modo relay dedicado: só encaminha circuitos para os outros, sem chat.
async fn serve_relay(args: &Args, device: &DeviceKey) -> Result<(), Box<dyn std::error::Error>> {
    let config = NodeConfig {
        mdns: false,
        relay_server: Some(args.relay_limits.clone()),
        external_addrs: args.external_addrs.clone(),
        ..Default::default()
    };
    let mut node = Node::new(device.keypair().clone(), config)?;
    println!(
        "kin relay {} — peer id: {}",
        env!("CARGO_PKG_VERSION"),
        node.peer_id()
    );
    match &args.listen {
        Some(addr) => node.listen_on(addr.clone())?,
        None => node.listen()?,
    }
    loop {
        match node.next_event().await {
            NodeEvent::Listening(addr) => println!("relay em {addr}/p2p/{}", node.peer_id()),
            NodeEvent::PeerConnected(peer) => println!("cliente conectado: {peer}"),
            NodeEvent::PeerDisconnected(peer) => println!("cliente saiu: {peer}"),
            _ => {}
        }
    }
}

fn on_event(event: ChatEvent, current: &mut Option<PeerId>, seen: &mut Vec<MessageId>) {
    match event {
        ChatEvent::Listening(addr) => println!("escutando em {addr}"),
        ChatEvent::PeerConnected(peer) => println!("conectado a {peer}"),
        ChatEvent::PeerDisconnected(peer) => {
            println!("desconectado de {peer}");
            if *current == Some(peer) {
                *current = None;
            }
        }
        ChatEvent::ConversationReady { peer, identity } => {
            println!(
                "conversa cifrada pronta com {} (identidade {})",
                peer,
                identity.to_base58()
            );
            *current = Some(peer);
        }
        ChatEvent::Received { message, .. } => {
            let Message {
                id, parent, text, ..
            } = message;
            seen.push(id);
            match parent {
                Some(parent) => println!("[{}] ↳ {}: {text}", short(&id), short(&parent)),
                None => println!("[{}] peer: {text}", short(&id)),
            }
        }
        ChatEvent::DialFailed { peer, reason } => {
            let who = peer.map_or_else(|| "?".into(), |p| p.to_string());
            println!("falha ao conectar a {who}: {reason}");
        }
        ChatEvent::Route { peer, relayed } => {
            let route = if relayed { "via relay" } else { "direta" };
            println!("rota com {peer}: {route}");
        }
        ChatEvent::Nat(status) => println!("NAT: {status:?}"),
        ChatEvent::RelayReserved { relay } => println!("reserva aceita no relay {relay}"),
        ChatEvent::HolePunch { peer, result } => match result {
            Ok(()) => println!("hole punching com {peer}: ok"),
            Err(e) => println!("hole punching com {peer} falhou: {e}"),
        },
        ChatEvent::Delivered { message, .. } => println!("(entregue {})", short(&message)),
        ChatEvent::SendFailed {
            message, reason, ..
        } => {
            let id = message.map_or_else(|| "handshake".into(), |m| short(&m));
            println!("(falha no envio {id}: {reason})");
        }
        ChatEvent::Dropped { peer, reason } => println!("(descartado de {peer}: {reason})"),
    }
}
