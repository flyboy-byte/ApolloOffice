#!/bin/sh
# Bare box -> office build environment, in one line (fork; docs/cloud-hosts.md "Box setup"):
#   curl -fsSL https://raw.githubusercontent.com/flyboy-byte/ApolloOffice/main/cloud/up.sh | REPO=owner/name sh
# Needs Docker with the compose plugin. Downloads happen on this box's connection, not the office's.
set -eu
RAW=${RAW:-https://raw.githubusercontent.com/flyboy-byte/ApolloOffice/main/cloud}
GHCR=${GHCR:-ghcr.io/flyboy-byte/apollo-office-build}
dir=${AO_DIR:-$HOME/.apollo-office-cloud}
mkdir -p "$dir" && cd "$dir"
curl -fsSL "$RAW/compose.yaml" -o compose.yaml
curl -fsSL "$RAW/Dockerfile" -o Dockerfile
TAG=${TAG:-$(sha256sum Dockerfile | cut -c1-10)}
IMG=apollo-office-build:$TAG
if ! docker image inspect "$IMG" >/dev/null 2>&1; then
  if docker pull "$GHCR:$TAG"; then docker tag "$GHCR:$TAG" "$IMG"
  else echo "no prebuilt image for $TAG, building it here"; docker build -q -t "$IMG" - < Dockerfile; fi
fi
export IMG
docker compose -f compose.yaml up -d build
docker compose -f compose.yaml run --rm prep
echo "ready: container apollo-office on $IMG"
