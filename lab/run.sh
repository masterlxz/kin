#!/usr/bin/env bash
# Laboratório de NAT (Fase 2): sobe relay + dois roteadores com NAT + dois peers e confere o que a
# escada de fallback faz. Uso: `cargo build -p kin-cli && lab/run.sh [cone|symmetric|relay-down]...`
# (sem argumentos roda os três cenários). Limpa containers e redes ao final.
set -uo pipefail
cd "$(dirname "$0")"

BIN=../target/debug/kin
[ -x "$BIN" ] || { echo "compile antes: cargo build -p kin-cli" >&2; exit 2; }

# A imagem é construída uma vez aqui (vários serviços construindo a mesma tag em paralelo brigam).
docker build -q -t kin-lab . >/dev/null || { echo "falha ao construir a imagem kin-lab" >&2; exit 2; }
compose() { docker compose "$@"; }
logs() { compose logs --no-log-prefix "$1" 2>&1; }
# Espera `padrão` aparecer no log do container, até `segundos`.
wait_log() {
  local c=$1 pat=$2 t=${3:-30}
  for _ in $(seq $((t * 2))); do
    logs "$c" | grep -qE "$pat" && return 0
    sleep 0.5
  done
  return 1
}
first() { logs "$1" | grep -oE "$2" | head -1; }
# Peer ID impresso na primeira linha do kin do container.
peer_id() { first "$1" 'peer id: [A-Za-z0-9]+' | awk '{print $3}'; }
# O container `$1` tem conexão DIRETA com o peer `$2`? (a rota para o relay também é "direta",
# por isso o padrão exige o Peer ID do outro lado.)
direct_with() { logs "$1" | grep -qE "rota com $2: direta"; }
wait_direct() { wait_log "$1" "rota com $2: direta" "${3:-45}"; }
# "Digita" uma linha no stdin do kin do container.
say() { compose exec -T "$1" sh -c "echo '$2' > /tmp/in"; }

FAILED=0
pass() { echo "  PASS  $1"; }
fail() { echo "  FAIL  $1"; FAILED=1; }
check() { # descrição, comando...
  local what=$1; shift
  if "$@"; then pass "$what"; else fail "$what"; return 1; fi
}

cleanup() { compose down -v --remove-orphans >/dev/null 2>&1; }
[ -n "${KEEP:-}" ] || trap cleanup EXIT   # KEEP=1 deixa os containers de pé para inspeção

# Sobe a topologia com o NAT pedido e deixa A e B com a conversa pronta (pelo relay).
# Deixa em variáveis: RELAY_ADDR, A_CIRCUIT.
setup() {
  local nat=$1
  cleanup
  export NAT_MODE=$nat A_ARGS="" B_ARGS=""
  compose up -d relay routera routerb >/dev/null || return 1
  wait_log relay "relay em /ip4/172.30.0.10/tcp/4001/p2p/" 30 || { echo "relay não subiu"; return 1; }
  RELAY_ADDR=$(first relay '/ip4/172.30.0.10/tcp/4001/p2p/[A-Za-z0-9]+')

  export A_ARGS="--data-dir /data --no-mdns --relay $RELAY_ADDR"
  compose up -d a >/dev/null || return 1
  wait_log a "reserva aceita no relay" 30 || { echo "a não conseguiu reserva no relay"; logs a; return 1; }
  A_CIRCUIT=$(first a '/ip4/172.30.0.10/tcp/4001/p2p/[A-Za-z0-9]+/p2p-circuit/p2p/[A-Za-z0-9]+')

  export B_ARGS="--data-dir /data --no-mdns --relay $RELAY_ADDR --dial $A_CIRCUIT"
  compose up -d b >/dev/null || return 1
  wait_log a "conversa cifrada pronta" 40 && wait_log b "conversa cifrada pronta" 40 \
    || { echo "a conversa não ficou pronta pelo relay"; logs a; logs b; return 1; }
  A_ID=$(peer_id a); B_ID=$(peer_id b)
}

delivered() { # remetente, destinatário, texto
  say "$1" "$3"
  wait_log "$2" "peer: $3" 20
}

scenario_cone() {
  echo "== NAT cone: o hole punching deve trocar o relay por conexão direta"
  setup cone || { fail "montar o laboratório"; return; }
  check "conversa E2EE pronta pelo relay" true
  check "mensagem chega pelo relay" delivered a b "oi pelo relay"
  if wait_direct a "$B_ID" 45 || wait_direct b "$A_ID" 5; then
    pass "hole punching: conexão direta entre a e b"
  else
    fail "hole punching: nenhuma conexão direta em 45s"; logs a | tail -25; logs b | tail -25
  fi
  check "mensagem chega depois do furo" delivered b a "oi direto"
}

scenario_symmetric() {
  echo "== NAT simétrico: o furo falha e a conversa continua pelo relay"
  setup symmetric || { fail "montar o laboratório"; return; }
  sleep 20
  if direct_with a "$B_ID" || direct_with b "$A_ID"; then
    fail "inesperado: conexão direta atrás de NAT simétrico"
  else
    pass "sem conexão direta (esperado)"
  fi
  check "mensagem chega pelo relay" delivered a b "oi simétrico"
  check "resposta chega pelo relay" delivered b a "volta simétrica"
}

scenario_relay_down() {
  echo "== Relay cai depois do furo: a conexão direta segue viva"
  setup cone || { fail "montar o laboratório"; return; }
  if ! (wait_direct a "$B_ID" 45 || wait_direct b "$A_ID" 5); then
    fail "sem conexão direta para testar"; return
  fi
  compose stop relay >/dev/null
  sleep 2
  check "mensagem chega sem o relay" delivered a b "sem relay"
}

SCENARIOS=("$@")
[ ${#SCENARIOS[@]} -eq 0 ] && SCENARIOS=(cone symmetric relay-down)
for s in "${SCENARIOS[@]}"; do
  case "$s" in
    cone) scenario_cone ;;
    symmetric) scenario_symmetric ;;
    relay-down) scenario_relay_down ;;
    *) echo "cenário desconhecido: $s" >&2; exit 2 ;;
  esac
done

[ $FAILED -eq 0 ] && echo "TUDO OK" || echo "HÁ FALHAS"
exit $FAILED
