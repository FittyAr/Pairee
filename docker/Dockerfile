FROM debian:bookworm-slim

# Prevent interactive prompts during installation
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && apt-get install -y \
    openssh-server \
    xauth \
    libx11-6 \
    libx11-dev \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    sudo \
    procps \
    x11-apps \
    nano \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Set up SSH server directory and users
RUN mkdir /var/run/sshd && \
    echo 'root:root' | chpasswd && \
    useradd -rm -d /home/pairee -s /bin/bash -g root -G sudo -u 1000 pairee && \
    echo 'pairee:pairee' | chpasswd && \
    echo 'pairee ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

# Configure SSH for X11 Forwarding and Authentication
RUN sed -i 's/#X11Forwarding yes/X11Forwarding yes/' /etc/ssh/sshd_config && \
    sed -i 's/#X11UseLocalhost yes/X11UseLocalhost no/' /etc/ssh/sshd_config && \
    echo "PermitRootLogin yes" >> /etc/ssh/sshd_config && \
    echo "PasswordAuthentication yes" >> /etc/ssh/sshd_config && \
    echo "AddressFamily any" >> /etc/ssh/sshd_config

# Fix PAM loginuid issue inside Docker container
RUN sed -i 's/session    required     pam_loginuid.so/session    optional     pam_loginuid.so/g' /etc/pam.d/sshd

# Install Rust globally for all users
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable

# Give user pairee ownership of Rust directories to allow cache writes and crate downloads
RUN chown -R pairee:root /usr/local/cargo /usr/local/rustup

# Make Rust toolchain accessible inside SSH login shells
RUN echo 'export RUSTUP_HOME=/usr/local/rustup\nexport CARGO_HOME=/usr/local/cargo\nexport PATH=/usr/local/cargo/bin:$PATH' > /etc/profile.d/rust.sh

# Set up global motd greeting explaining testing instructions
RUN echo '\n\
 =========================================================================\n\
   Welcome to the Pairee SSH Development & Testing Container!             \n\
 =========================================================================\n\
 \n\
  To build and run Pairee:\n\
    1. Go to the project root:   cd /workspace\n\
    2. Build the project:        cargo build --release\n\
    3. Run Pairee:               cargo run\n\
 \n\
  To test the bottom bar modifier keys (Ctrl/Alt):\n\
    - Ensure you connected to this container using SSH with X11 forwarding.\n\
      On Windows, you need an X server running (VcXsrv, Xming, or MobaXterm).\n\
    - Check if X11 forwarding works: test it by running `xeyes` or `xclock`.\n\
    - If X11 works, running `cargo run` will dynamically load libX11.so\n\
      and show modifier key updates in real-time when pressing Ctrl/Alt!\n\
    - Fallback: Press Ctrl+p in the app to cycle modifier states manually.\n\
 =========================================================================\n' > /etc/motd

# Expose SSH port
EXPOSE 22

# Copy entrypoint script and set permission
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

WORKDIR /workspace

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
