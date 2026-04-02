# Minimal Vera CLI image
# Build: docker build -t vera:local .
# Run:   docker run --rm -v "$(pwd)":/workspace vera:local --version

FROM ubuntu:24.04 AS downloader

ARG VERA_VERSION=v0.11.5
ARG TARGETARCH

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl tar && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /tmp
RUN case "${TARGETARCH}" in \
      amd64) TARGET="x86_64-unknown-linux-gnu" ;; \
      arm64) TARGET="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac && \
    ARCHIVE="vera-${TARGET}.tar.gz" && \
    URL="https://github.com/ineersa/Vera/releases/download/${VERA_VERSION}/${ARCHIVE}" && \
    curl -fsSL "${URL}" -o "/tmp/${ARCHIVE}" && \
    tar -xzf "/tmp/${ARCHIVE}" -C /tmp && \
    mv "/tmp/vera-${TARGET}/vera" /usr/local/bin/vera && \
    chmod +x /usr/local/bin/vera

FROM ubuntu:24.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=downloader /usr/local/bin/vera /usr/local/bin/vera

ENV VERA_NO_UPDATE_CHECK=1

WORKDIR /workspace

ENTRYPOINT ["vera"]
CMD ["help"]
