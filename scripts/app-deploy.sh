#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

PROJECT_ID=utrakr
APP=utrakr-api
: "${VERSION:?Variable not set or empty}"


set -x
docker-credential-gcr configure-docker

docker network inspect local || docker network create local

IMAGE=redis:6.0
docker pull "${IMAGE}"
docker rm -f "${APP}-redis" || :
docker run --name "${APP}-redis" -d\
 --memory 200m --memory-swap 200m\
 --network local\
 -v /mnt/disks/app_data/redis/:/data\
 "${IMAGE}"\
 redis-server --appendonly yes

IMAGE="us.gcr.io/${PROJECT_ID}/${APP}:${VERSION}"
docker pull "${IMAGE}"
docker rm -f "${APP}" || :
docker run --name "${APP}" -d\
 --memory 200m --memory-swap 200m\
 --network local\
 -e REDIRECT_HOMEPAGE=https://www.utrakr.app\
 -e DEFAULT_BASE_HOST=utrakr.app\
 -e COOKIE_SECURE=true\
 -e REDIS_URLS_CLIENT_CONN=redis://${APP}-redis\
 -l traefik.enable=true\
 -l traefik.http.middlewares.${APP}_redirect.redirectscheme.scheme=https\
 -l traefik.http.routers.${APP}_http.entrypoints=web\
 -l traefik.http.routers.${APP}_http.rule='Host(`utrakr.app`)'\
 -l traefik.http.routers.${APP}_http.middlewares=${APP}_redirect\
 -l traefik.http.routers.${APP}.entrypoints=websecure\
 -l traefik.http.routers.${APP}.rule='Host(`utrakr.app`)'\
 -l traefik.http.routers.${APP}.tls.certresolver=le\
 -l traefik.http.services.${APP}.loadbalancer.server.port=8080\
 "${IMAGE}"
