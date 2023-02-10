FROM registry.fedoraproject.org/fedora-toolbox:37 AS devenv

RUN dnf install -y gcc gcc-c++ openssl openssl-devel npm xz
RUN npm install --global yarn

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
ENV PATH "$PATH:/root/.cargo/bin"
LABEL org.opencontainers.image.source="https://github.com/seedwing-io/seedwing-policy"

RUN rustup target add wasm32-unknown-unknown

RUN true \
    && curl -sSL https://raw.githubusercontent.com/drogue-iot/drogue-cloud-build-tools/main/binaries/$(uname -p)/wasm-bindgen/0.2.82/wasm-bindgen.xz -o wasm-bindgen.xz \
    && unxz wasm-bindgen.xz \
    && install -m 0555 wasm-bindgen /usr/local/bin/ \
    && rm wasm-bindgen

RUN true \
    && curl -sSL https://github.com/WebAssembly/binaryen/releases/download/version_109/binaryen-version_109-x86_64-linux.tar.gz -o binaryen.tar.gz \
    && tar --strip-components=2 -xzvf binaryen.tar.gz '*/wasm-opt' \
    && rm binaryen.tar.gz \
    && cp wasm-opt /usr/local/bin/ && rm wasm-opt

RUN npm install -g sass@1.58.3 && sass --version

# We use our own binary due to aarch64 and an issue with GLIBC_2.29
RUN true \
    && curl -sSL https://raw.githubusercontent.com/drogue-iot/drogue-cloud-build-tools/main/binaries/$(uname -p)/trunk/0.16.0/trunk.xz -o trunk.xz \
    && unxz trunk.xz \
    && install -m 0555 trunk /usr/local/bin/ \
    && rm trunk

FROM devenv AS builder

RUN cargo install cargo-auditable
RUN mkdir /usr/src/project
COPY . /usr/src/project
WORKDIR /usr/src/project

RUN cd seedwing-policy-frontend && yarn install
RUN cargo auditable build --release --features frontend

RUN mkdir /result && cp -pv target/release/swio /result/


FROM registry.access.redhat.com/ubi9/ubi-minimal:latest

LABEL org.opencontainers.image.source="https://github.com/seedwing-io/seedwing-policy"

COPY --from=builder /result/swio /

EXPOSE 8080

ENTRYPOINT ["/swio", "serve"]
