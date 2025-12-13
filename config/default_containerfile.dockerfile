# syntax=docker/dockerfile:1
FROM docker.io/debian:bookworm-slim

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=1000

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache

# System package installation with Cache Mount
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    build-essential git curl wget sudo \
    openssh-client \
    libssl-dev zlib1g-dev libreadline-dev libyaml-dev libncurses5-dev libffi-dev libgdbm-dev \
    libpq-dev redis-tools imagemagick libvips ripgrep fd-find \
    vim unzip zip tar xclip \
    python3 python3-pip python3-venv \
    cmake iputils-ping iproute2 dnsutils ca-certificates

# install nvim
RUN curl -LO https://github.com/neovim/neovim/releases/download/v0.11.5/nvim-linux-x86_64.tar.gz && \
    tar -C /opt -xzf nvim-linux-x86_64.tar.gz && \
    rm nvim-linux-x86_64.tar.gz && \
    ln -s /opt/nvim-linux-x86_64/bin/nvim /usr/local/bin/nvim

# install lazygit
ARG LAZYGIT_VERSION=0.56.0
RUN curl -Lo lazygit.tar.gz "https://github.com/jesseduffield/lazygit/releases/download/v${LAZYGIT_VERSION}/lazygit_${LAZYGIT_VERSION}_linux_x86_64.tar.gz" && \
    tar xf lazygit.tar.gz lazygit && \
    install lazygit -D -t /usr/local/bin/ && \
    rm lazygit.tar.gz lazygit

# install zellij
RUN curl -Lo zellij.tar.gz "https://github.com/zellij-org/zellij/releases/latest/download/zellij-x86_64-unknown-linux-musl.tar.gz" && \
    tar -xf zellij.tar.gz && \
    chmod +x zellij && \
    install zellij -D -t /usr/local/bin/ && \
    rm zellij.tar.gz zellij && \
    zellij --version


RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m -s /bin/bash $USERNAME \
    && echo "$USERNAME ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

USER $USERNAME
WORKDIR /home/$USERNAME

# Install Mise
RUN curl https://mise.jdx.dev/install.sh | sh
ENV PATH="/home/${USERNAME}/.local/bin:${PATH}"

# Install Starship
RUN curl -sS https://starship.rs/install.sh | sh -s -- -y -b "/home/${USERNAME}/.local/bin"

RUN mkdir -p /home/${USERNAME}/.local/state/bash
ENV HISTFILE=/home/${USERNAME}/.local/state/bash/history
ENV HISTCONTROL=ignoreboth:erasedups

RUN echo 'eval "$($HOME/.local/bin/mise activate bash)"' >> ~/.bashrc && \
    echo 'eval "$(starship init bash)"' >> ~/.bashrc && \
    echo 'alias v="nvim"' >> ~/.bashrc && \
    echo 'alias ll="ls -alF"' >> ~/.bashrc && \
    echo 'alias la="ls -A"' >> ~/.bashrc

RUN git clone --depth 1 https://github.com/JohnAnon9771/my-nvim.git /home/$USERNAME/.config/nvim || true

COPY --chown=$USER_UID:$USER_GID mise.toml /home/$USERNAME/.config/mise/config.toml
COPY --chown=$USER_UID:$USER_GID starship.toml /home/$USERNAME/.config/starship.toml
ENV STARSHIP_CONFIG=/home/$USERNAME/.config/starship.toml

# Configure Zellij
RUN mkdir -p /home/$USERNAME/.config/zellij
COPY --chown=$USER_UID:$USER_GID zellij.kdl /home/$USERNAME/.config/zellij/config.kdl

RUN --mount=type=cache,target=/home/dev/.cache/mise,uid=$USER_UID,gid=$USER_GID \
    mise install

RUN --mount=type=cache,target=/home/$USERNAME/.npm,uid=$USER_UID,gid=$USER_GID \
    mise use -g npm:@anthropic-ai/claude-code && \
    mise use -g npm:@openai/codex && \
    mise use -g npm:@google/gemini-cli

# install devobox (enables devobox-in-devobox)
RUN curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/x86_64-unknown-linux-gnu.tar.gz -o /tmp/devobox.tar.gz && \
    tar -xzf /tmp/devobox.tar.gz -C /tmp && \
    mv /tmp/devobox /usr/local/bin/devobox && \
    chmod +x /usr/local/bin/devobox && \
    rm /tmp/devobox.tar.gz

CMD ["/bin/bash"]
