#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

PROJECT_ID=utrakr
APP=utrakr-api
: "${VERSION:?Variable not set or empty}"

set -x
docker-credential-gcr configure-docker

function install_app_events_cron() {
  # app event log services
  cat <<"EOF" > /etc/systemd/system/app-event-logs.service
[Unit]
Requires=docker.service
After=docker.service
Wants=app-event-logs.timer

[Service]
ExecStart=/usr/bin/docker run --rm --name app-event-logs\
  --memory 100m --memory-swap 100m\
  -v /mnt/disks/app_data/utrakr-api:/data\
  gcr.io/google.com/cloudsdktool/cloud-sdk:alpine\
  gsutil rsync -r /data gs://utrakr-prod-utrakr-api-data

[Install]
WantedBy=multi-user.target
EOF

  cat <<"EOF" > /etc/systemd/system/app-event-logs.timer
[Unit]
Requires=app-event-logs.service

[Timer]
Unit=app-event-logs.service
OnUnitInactiveSec=15m
RandomizedDelaySec=1m
AccuracySec=1s

[Install]
WantedBy=timers.target
EOF

  systemctl daemon-reload
  systemctl enable app-event-logs.timer
  systemctl start app-event-logs
}
sudo bash -c "$(declare -f install_app_events_cron); set -euo pipefail; install_app_events_cron"

# redis
IMAGE=redis:6.0
docker pull "${IMAGE}"
docker rm -f "${APP}-redis" || :
docker run --name "${APP}-redis" -d\
 --memory 100m --memory-swap 100m\
 --network local\
 -v /mnt/disks/app_data/redis/:/data\
 "${IMAGE}"\
 redis-server --appendonly yes

# app
IMAGE="us.gcr.io/${PROJECT_ID}/${APP}:${VERSION}"
docker pull "${IMAGE}"
docker rm -f "${APP}" || :
docker run --name "${APP}" -d\
 --memory 100m --memory-swap 100m\
 --network local\
 -e REDIRECT_HOMEPAGE=https://www.utrakr.app\
 -e DEFAULT_BASE_HOST=utrakr.app\
 -e COOKIE_SECURE=true\
 -e REDIS_URLS_CLIENT_CONN=redis://${APP}-redis\
 -e EVENT_LOG_FOLDER=/data/event-logs\
 -v /mnt/disks/app_data/${APP}/:/data\
 -l traefik.enable=true\
 -l traefik.http.middlewares.${APP}_redirect.redirectscheme.scheme=https\
 -l traefik.http.routers.${APP}_http.entrypoints=web\
 -l traefik.http.routers.${APP}_http.rule='Host(`utrakr.app`)'\
 -l traefik.http.routers.${APP}_http.middlewares=${APP}_redirect\
 -l traefik.http.routers.${APP}.entrypoints=websecure\
 -l traefik.http.routers.${APP}.rule='Host(`utrakr.app`) || Host(`api.utrakr.app`)'\
 -l traefik.http.routers.${APP}.tls.certresolver=le\
 -l traefik.http.services.${APP}.loadbalancer.server.port=8080\
 "${IMAGE}"
