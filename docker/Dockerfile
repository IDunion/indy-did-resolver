ARG RUST_VER=1.58

FROM rust:${RUST_VER}-buster as builder
USER root
ENV LANG=C.UTF-8 \
    CARGO_HOME="/root/.cargo" \
    USER="root"

RUN mkdir -p ${CARGO_HOME}
RUN mkdir -p /app
RUN apt-get update && apt-get install -y libzmq3-dev cmake 

# Copy sources and build
WORKDIR /app
COPY . .
RUN cargo build --release

# Check licenses and generate report
RUN cargo install --locked cargo-about
RUN cargo about generate about.hbs > licenses.html


FROM debian:buster-slim
USER root
RUN apt-get update && apt-get install ca-certificates -y && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/indy-did-driver /app/
COPY --from=builder /app/indy-did-driver/genesis-files /app/genesis-files

COPY --from=builder /app/licenses.html /app/

#RUN groupadd runner && useradd -r -g runner runner
#USER runner

EXPOSE 8080

RUN printf "General information about third-party software components and their licenses, \
which are distributed with this image, can be found in the the licenses.html \
file distributed with this image at /app/licenses.html."

WORKDIR /app
ENTRYPOINT ["/app/indy-did-driver"]
