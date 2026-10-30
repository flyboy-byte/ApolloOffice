FROM rust:1-trixie
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev clang cmake git curl ca-certificates procps xz-utils unzip libxdo-dev mesa-vulkan-drivers \
    python3 python3-pip python3-venv python3-dev nodejs npm golang-go \
    libasound2-dev libfontconfig1-dev libfreetype-dev libgl1-mesa-dev libgtk-3-dev libwayland-dev \
    libx11-dev libxcursor-dev libxi-dev libxkbcommon-dev libxrandr-dev libudev-dev xvfb \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy && rustup target add wasm32-unknown-unknown && npm install -g pnpm yarn && pip3 install --break-system-packages pytest
ENV CARGO_TERM_COLOR=never CARGO_INCREMENTAL=0 PIP_BREAK_SYSTEM_PACKAGES=1
WORKDIR /work
