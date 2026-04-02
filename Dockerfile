# Vera Docker image (release binary, API mode)
# Build:  docker build -t vera:local .
# Run:    docker run --rm -i -v $(pwd):/workspace vera:local mcp

FROM debian:trixie-slim AS downloader

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl jq && \
    rm -rf /var/lib/apt/lists/*

ARG TARGET=x86_64-unknown-linux-musl
ARG REPO=ineersa/Vera

WORKDIR /tmp
RUN ARCHIVE="vera-${TARGET}.tar.gz" && \
    RELEASE_URL=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" \
      | jq -r ".assets[] | select(.name == \"${ARCHIVE}\") | .browser_download_url") && \
    if [ -z "$RELEASE_URL" ]; then echo "ERROR: ${ARCHIVE} not found in latest release" >&2; exit 1; fi && \
    echo "Downloading ${RELEASE_URL}..." && \
    curl -sL "$RELEASE_URL" -o "$ARCHIVE" && \
    tar xzf "$ARCHIVE" && \
    mv vera /usr/local/bin/vera && \
    chmod +x /usr/local/bin/vera && \
    rm -rf /tmp/*

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=downloader /usr/local/bin/vera /usr/local/bin/vera

WORKDIR /workspace

ENTRYPOINT ["vera"]
CMD ["mcp"]
