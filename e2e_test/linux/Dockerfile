FROM ubuntu:24.04

# Install C++ / Python
RUN apt-get update && apt-get install -y --no-install-recommends wget curl build-essential libreadline-dev \ 
    libncursesw5-dev libssl-dev libsqlite3-dev libgdbm-dev libbz2-dev liblzma-dev zlib1g-dev uuid-dev libffi-dev libdb-dev ca-certificates

RUN wget --no-check-certificate https://www.python.org/ftp/python/3.10.15/Python-3.10.15.tgz \
    && tar -xf Python-3.10.15.tgz \
    && cd Python-3.10.15 \
    && ./configure --enable-optimizations\
    && make \
    && make install

RUN rm Python-3.10.15.tgz \
    && rm -r Python-3.10.15 \
    && echo 'alias python=python3' >> ~/.bashrc

# Install Rust
ENV RUST_HOME=/usr/local/lib/rust
ENV RUSTUP_HOME=${RUST_HOME}/rustup
ENV CARGO_HOME=${RUST_HOME}/cargo
RUN mkdir /usr/local/lib/rust && chmod 0755 $RUST_HOME
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > ${RUST_HOME}/rustup.sh \
    && chmod +x ${RUST_HOME}/rustup.sh \
    && ${RUST_HOME}/rustup.sh -y
ENV PATH=$PATH:$CARGO_HOME/bin:$RUSTUP_HOME/bin
RUN rustup toolchain install 1.70

# Install Go
RUN curl -OsSf https://dl.google.com/go/go1.20.6.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go1.20.6.linux-amd64.tar.gz \
    && rm go1.20.6.linux-amd64.tar.gz
ENV PATH=$PATH:/usr/local/go/bin

# Install uv
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
