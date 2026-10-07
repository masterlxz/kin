//! Relays candidatos e ranking local (Fase 2.3). Sem "relay oficial": o nó mede cada candidato e
//! reserva nos melhores, trocando por outro quando um falha.

use std::time::Duration;

use libp2p::core::transport::ListenerId;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use tokio::time::Instant;

use crate::Error;

/// Quanto esperar a reserva ser aceita antes de dar o relay como falho.
const PENDING_TIMEOUT: Duration = Duration::from_secs(15);
/// Quanto um relay que falhou fica de molho antes de ser tentado de novo.
const BACKOFF: Duration = Duration::from_secs(30);
/// Latência presumida de um relay ainda não medido.
const DEFAULT_RTT: Duration = Duration::from_millis(200);

/// Em que pé está um relay candidato.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayState {
    /// Disponível para ser usado.
    Idle,
    /// Conectando e pedindo a reserva.
    Pending,
    /// Reserva aceita: este nó é alcançável por ele.
    Reserved,
    /// Falhou há pouco; só volta a ser tentado depois de um tempo.
    Backoff,
}

/// O que o nó sabe de um relay candidato (somente leitura, para a UI e para o app persistir).
#[derive(Debug, Clone)]
pub struct RelayStat {
    pub addr: Multiaddr,
    pub peer: PeerId,
    pub state: RelayState,
    pub ok: u32,
    pub fail: u32,
    /// Tempo médio (EWMA) entre pedir e ter a reserva aceita.
    pub rtt: Option<Duration>,
}

impl RelayStat {
    /// Quanto maior, melhor: taxa de sucesso suavizada, penalizada pela latência.
    pub fn score(&self) -> f64 {
        let rate = f64::from(self.ok + 1) / f64::from(self.ok + self.fail + 2);
        let rtt_ms = self.rtt.unwrap_or(DEFAULT_RTT).as_secs_f64() * 1000.0;
        rate / (1.0 + rtt_ms / 200.0)
    }
}

struct Candidate {
    stat: RelayStat,
    listener: Option<ListenerId>,
    since: Instant,
    retry_at: Option<Instant>,
}

pub(crate) struct RelayBook {
    candidates: Vec<Candidate>,
    wanted: usize,
}

/// Peer ID do relay: o último `/p2p/<id>` do endereço (obrigatório).
fn relay_peer(addr: &Multiaddr) -> Result<PeerId, Error> {
    addr.iter()
        .filter_map(|p| match p {
            Protocol::P2p(id) => Some(id),
            _ => None,
        })
        .last()
        .ok_or_else(|| Error::Build(format!("relay sem /p2p/<peer id>: {addr}")))
}

impl RelayBook {
    pub(crate) fn new(addrs: &[Multiaddr], wanted: usize) -> Result<Self, Error> {
        let mut book = Self {
            candidates: Vec::new(),
            wanted: wanted.max(1),
        };
        for addr in addrs {
            book.add(addr.clone())?;
        }
        Ok(book)
    }

    /// Acrescenta um candidato (ignora um que já esteja na lista).
    pub(crate) fn add(&mut self, addr: Multiaddr) -> Result<(), Error> {
        let peer = relay_peer(&addr)?;
        if self.candidates.iter().all(|c| c.stat.peer != peer) {
            self.candidates.push(Candidate {
                stat: RelayStat {
                    addr,
                    peer,
                    state: RelayState::Idle,
                    ok: 0,
                    fail: 0,
                    rtt: None,
                },
                listener: None,
                since: Instant::now(),
                retry_at: None,
            });
        }
        Ok(())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Quantos relays estão em uso (conectando ou reservados).
    pub(crate) fn active(&self) -> usize {
        self.candidates
            .iter()
            .filter(|c| matches!(c.stat.state, RelayState::Pending | RelayState::Reserved))
            .count()
    }

    pub(crate) fn wants_more(&self) -> bool {
        self.active() < self.wanted
    }

    /// O melhor candidato disponível (empate: o primeiro da lista).
    pub(crate) fn next_eligible(&self) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (i, c) in self.candidates.iter().enumerate() {
            if c.stat.state != RelayState::Idle {
                continue;
            }
            if best.is_none_or(|b| c.stat.score() > self.candidates[b].stat.score()) {
                best = Some(i);
            }
        }
        best
    }

    pub(crate) fn addr(&self, i: usize) -> &Multiaddr {
        &self.candidates[i].stat.addr
    }

    pub(crate) fn mark_pending(&mut self, i: usize, listener: ListenerId, now: Instant) {
        let c = &mut self.candidates[i];
        c.stat.state = RelayState::Pending;
        c.listener = Some(listener);
        c.since = now;
    }

    /// A reserva foi aceita. Idempotente (renovações chegam como novas aceitações).
    pub(crate) fn mark_reserved(&mut self, peer: PeerId, now: Instant) {
        let Some(c) = self.candidates.iter_mut().find(|c| c.stat.peer == peer) else {
            return;
        };
        if c.stat.state == RelayState::Pending {
            let sample = now.saturating_duration_since(c.since);
            c.stat.rtt = Some(match c.stat.rtt {
                Some(old) => old.mul_f64(0.7) + sample.mul_f64(0.3),
                None => sample,
            });
            c.stat.ok += 1;
        }
        c.stat.state = RelayState::Reserved;
    }

    /// Dá o relay como falho (não conectou, recusou, caiu ou demorou demais).
    pub(crate) fn mark_failed(&mut self, i: usize, now: Instant) -> PeerId {
        let c = &mut self.candidates[i];
        c.stat.fail += 1;
        c.stat.state = RelayState::Backoff;
        c.listener = None;
        c.retry_at = Some(now + BACKOFF);
        c.stat.peer
    }

    /// O ouvinte de circuito `id` fechou: devolve o índice do relay a que pertencia, se algum.
    pub(crate) fn find_listener(&self, id: ListenerId) -> Option<usize> {
        self.candidates.iter().position(|c| c.listener == Some(id))
    }

    #[cfg(test)]
    pub(crate) fn peer(&self, i: usize) -> PeerId {
        self.candidates[i].stat.peer
    }

    /// Tentativas pendentes que passaram do prazo, com o ouvinte para encerrar.
    pub(crate) fn expired(&self, now: Instant) -> Vec<(usize, ListenerId)> {
        self.candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.stat.state == RelayState::Pending && now >= c.since + PENDING_TIMEOUT
            })
            .filter_map(|(i, c)| c.listener.map(|l| (i, l)))
            .collect()
    }

    /// Relays de molho que já podem ser tentados de novo voltam a `Idle`.
    pub(crate) fn tick(&mut self, now: Instant) {
        for c in &mut self.candidates {
            if c.stat.state == RelayState::Backoff && c.retry_at.is_some_and(|t| now >= t) {
                c.stat.state = RelayState::Idle;
                c.retry_at = None;
            }
        }
    }

    /// Próximo instante em que algo muda sozinho (fim do prazo de uma tentativa ou do backoff).
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.candidates
            .iter()
            .filter_map(|c| match c.stat.state {
                RelayState::Pending => Some(c.since + PENDING_TIMEOUT),
                RelayState::Backoff => c.retry_at,
                _ => None,
            })
            .min()
    }

    /// Todos os candidatos, do melhor para o pior.
    pub(crate) fn stats(&self) -> Vec<RelayStat> {
        let mut all: Vec<RelayStat> = self.candidates.iter().map(|c| c.stat.clone()).collect();
        all.sort_by(|a, b| b.score().total_cmp(&a.score()));
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relay(port: u16) -> Multiaddr {
        format!("/ip4/203.0.113.7/tcp/{port}/p2p/{}", PeerId::random())
            .parse()
            .unwrap()
    }

    fn book(n: usize, wanted: usize) -> RelayBook {
        let addrs: Vec<_> = (0..n).map(|i| relay(4000 + i as u16)).collect();
        RelayBook::new(&addrs, wanted).unwrap()
    }

    #[test]
    fn address_without_a_peer_id_is_rejected_and_duplicates_are_ignored() {
        let bare: Multiaddr = "/ip4/203.0.113.7/tcp/4001".parse().unwrap();
        assert!(RelayBook::new(&[bare], 1).is_err());

        let one = relay(4001);
        let mut book = RelayBook::new(std::slice::from_ref(&one), 1).unwrap();
        book.add(one).unwrap();
        assert_eq!(book.stats().len(), 1);
    }

    #[test]
    fn ties_keep_the_configured_order() {
        let book = book(3, 1);
        assert_eq!(book.next_eligible(), Some(0));
    }

    #[test]
    fn failing_relay_drops_in_rank_and_the_next_one_is_picked() {
        let mut book = book(2, 1);
        let now = Instant::now();
        let first = book.next_eligible().unwrap();
        book.mark_pending(first, ListenerId::next(), now);
        assert_eq!(book.active(), 1);
        assert!(!book.wants_more());

        book.mark_failed(first, now);
        assert!(book.wants_more());
        assert_eq!(book.next_eligible(), Some(1), "o que falhou fica de molho");

        let stats = book.stats();
        assert_eq!(
            stats[0].addr,
            *book.addr(1),
            "o que falhou desce no ranking"
        );
        assert_eq!(stats[1].fail, 1);
    }

    #[test]
    fn faster_relay_ranks_above_a_slower_one_and_a_failed_one_returns_after_backoff() {
        let mut book = book(2, 2);
        let t0 = Instant::now();
        for (i, ms) in [(0usize, 400u64), (1, 40)] {
            book.mark_pending(i, ListenerId::next(), t0);
            book.mark_reserved(book.peer(i), t0 + Duration::from_millis(ms));
        }
        let stats = book.stats();
        assert_eq!(stats[0].addr, *book.addr(1), "o mais rápido vem primeiro");
        assert!(stats[0].rtt.unwrap() < stats[1].rtt.unwrap());

        // Falhou: sai do jogo por um tempo; passado o backoff, volta a ser candidato.
        book.mark_failed(0, t0);
        assert_eq!(book.next_eligible(), None);
        book.tick(t0 + BACKOFF + Duration::from_secs(1));
        assert_eq!(book.next_eligible(), Some(0));
    }

    #[test]
    fn pending_attempt_expires_after_the_timeout() {
        let mut book = book(1, 1);
        let t0 = Instant::now();
        book.mark_pending(0, ListenerId::next(), t0);
        assert!(book.expired(t0 + Duration::from_secs(1)).is_empty());
        assert_eq!(book.expired(t0 + PENDING_TIMEOUT).len(), 1);
        assert_eq!(book.next_deadline(), Some(t0 + PENDING_TIMEOUT));
    }
}
