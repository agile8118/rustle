#!/bin/bash
set -e

# Same as deploy.sh, but with this, you can also deploy even if you don't have Rust installed locally for as long as you have Docker.


REMOTE_DIR="~/rustle"
TARGET="aarch64-unknown-linux-musl"
RELEASE_DIR="target/$TARGET/release"
BUILD_IMAGE="rust:alpine"

echo "Building server + seed for $TARGET via Docker ($BUILD_IMAGE)..."
docker run --rm \
  -v "$PWD":/work -w /work \
  -v rustle-cargo-registry:/usr/local/cargo/registry \
  "$BUILD_IMAGE" \
  sh -c "apk add --no-cache musl-dev gcc >/dev/null && cargo build --release --target $TARGET && chown -R $(id -u):$(id -g) target"

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
