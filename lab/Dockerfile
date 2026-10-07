# Imagem única do laboratório de NAT: só o necessário para rotear (iptables) e rodar o binário `kin`,
# que é compilado NO HOST e montado por volume (nada de recompilar Rust dentro de container).
FROM debian:stable-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends iptables iproute2 \
    && rm -rf /var/lib/apt/lists/*
