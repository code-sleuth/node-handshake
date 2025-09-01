#!/usr/bin/env bash

set -e

docker compose down

sleep 1

docker compose build

sleep 1

docker compose up -d

# watch gossip server logs
docker-compose logs -f --tail=10 gossip-server
