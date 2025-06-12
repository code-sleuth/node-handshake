#!/usr/bin/env bash

set -e

docker compose down

sleep 1

docker compose build

sleep 1

docker compose up -d

docker logs node-handshake-node-handshake-1 -f