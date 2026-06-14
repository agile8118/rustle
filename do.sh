#!/bin/bash

CMD=$1

# If no command is given, show usage
if [ -z "$CMD" ]; then
  echo "Usage: $0 [build | run | seed | test]"
  exit 1
fi

if [ "$CMD" == "build" ]; then
  echo "Building server..."
  cargo build --release
  echo "Build complete. Run with: ./do.sh run"
fi

if [ "$CMD" == "run" ]; then
  if [ -f .env ]; then
    echo "Starting server..."
    ./env.sh cargo run
  else
    echo "Starting server with pm2..."
    pm2 startOrReload ecosystem.config.js --update-env
  fi
fi

if [ "$CMD" == "seed" ]; then
  if command -v cargo &>/dev/null; then
    echo "Seeding database..."
    ./env.sh cargo run --bin seed
  else
    echo "Seeding database (precompiled)..."
    ./env.sh ./seed
  fi
fi

if [ "$CMD" == "test" ]; then
  echo "Running tests against .env.test..."
  set -a
  . ./.env.test
  set +a
  cargo test
fi
