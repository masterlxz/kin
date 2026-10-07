#!/bin/sh
# Entrada dos containers `kin` (relay, a, b). Se houver GATEWAY, sai pela rota do roteador de NAT.
# O stdin do kin é uma fifo (/tmp/in): o run.sh "digita" mensagens com `echo ... > /tmp/in`.
set -eu

[ -n "${GATEWAY:-}" ] && ip route replace default via "$GATEWAY"
rm -f /tmp/in
mkfifo /tmp/in
# shellcheck disable=SC2086
exec kin $KIN_ARGS <> /tmp/in
