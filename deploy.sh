#!/bin/bash
set -e

# Cross-compiles the server + seed binaries for linux/arm64 (Ubuntu arm64
# prod) using `cross` (Docker-based), then uploads them along with static
# assets, env.sh, and the pm2 config to the "sky" host (~/.ssh/config) and
# (re)starts the app with pm2.
#
# Templates and migrations are embedded into the binaries at compile time, so
# only the binaries + public/ + env.sh + ecosystem.config.js need to ship.
# The prod machine needs pm2 + Postgres reachable, but no Rust toolchain.
#
# There is no .env on the server — env.sh falls back to AWS SSM
# (/rustle/prod/*) for config, so the server needs AWS credentials with
# access to that SSM path.
#
# On an x86_64 host (e.g. Windows/WSL), the arm64 build runs under QEMU and
# is much slower. If it fails with an "exec format error", run once:
#   docker run --privileged --rm tonistiigi/binfmt --install all

REMOTE_DIR="~/rustle"
TARGET="aarch64-unknown-linux-musl"
RELEASE_DIR="target/$TARGET/release"

if ! command -v cross &>/dev/null; then
  echo "Installing cross..."
  cargo install cross --git https://github.com/cross-rs/cross
fi

echo "Building server + seed for $TARGET..."
cross build --release --target "$TARGET"

echo "Ensuring remote directories exist..."
ssh sky "mkdir -p $REMOTE_DIR/public"

echo "Uploading binaries..."
scp -q "$RELEASE_DIR/server" "sky:$REMOTE_DIR/server"
scp -q "$RELEASE_DIR/seed" "sky:$REMOTE_DIR/seed"

echo "Syncing public assets..."
rsync -az --delete public/ "sky:$REMOTE_DIR/public/"

echo "Uploading env.sh, do.sh, and pm2 config..."
scp -q env.sh do.sh ecosystem.config.js "sky:$REMOTE_DIR/"
ssh sky "chmod +x $REMOTE_DIR/do.sh"

echo "Restarting app..."
ssh sky "cd $REMOTE_DIR && pm2 startOrReload ecosystem.config.js --update-env"

echo "Deploy complete."
