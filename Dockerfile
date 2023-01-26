FROM registry.access.redhat.com/ubi9/ubi:latest as builder

RUN dnf install -y gcc gcc-c++ openssl openssl-devel
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal

ENV PATH "$PATH:/root/.cargo/bin"

RUN cargo install cargo-auditable

RUN mkdir /usr/src/project
COPY . /usr/src/project
WORKDIR /usr/src/project

RUN cargo auditable build --release

RUN mkdir /result && cp -pv target/release/seedwing-policy-server /result/

FROM registry.access.redhat.com/ubi9/ubi-minimal:latest

LABEL org.opencontainers.image.source="https://github.com/seedwing-io/seedwing-policy"

COPY --from=builder /result/seedwing-policy-server /

EXPOSE 8080

ENTRYPOINT ["/seedwing-policy-server"]
