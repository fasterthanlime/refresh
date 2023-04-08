FROM ubuntu:20.04 AS builder

RUN set -eux; \
		apt update; \
		apt install -y --no-install-recommends \
			curl ca-certificates gcc libc6-dev pkg-config libssl-dev \
			;		

# Install rustup
RUN set -eux; \
		curl --location --fail \
			"https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init" \
			--output rustup-init; \
		chmod +x rustup-init; \
		./rustup-init -y --no-modify-path --default-toolchain stable; \
		rm rustup-init;

# Add rustup to path, check that it works
ENV PATH=${PATH}:/root/.cargo/bin
RUN set -eux; \
		rustup --version;

# Copy sources and build them
WORKDIR /app
COPY src src
COPY Cargo.toml Cargo.lock ./

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN --mount=type=cache,target=/root/.rustup \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
		--mount=type=cache,target=/app/target \
		set -eux; \
		rustup default stable; \
		cargo build --release; \
		cp target/release/refresh-proxy /app/refresh-proxy

################################################################################

FROM ubuntu:20.04

RUN set -eux; \
		apt update; \
		apt install -y --no-install-recommends \
			curl ca-certificates bash unzip

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN set -eux; \
		curl -fsSL https://deno.land/x/install/install.sh | bash; \
		/root/.deno/bin/deno upgrade --version 1.32.1

ENV PATH=${PATH}:/root/.deno/bin

COPY --from=builder /app/refresh-proxy /app/refresh-proxy

WORKDIR /deno-app
COPY . /deno-app

CMD ["/app/refresh-proxy", "serve"]
