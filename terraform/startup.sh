#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

cat <<"EOF" > /etc/systemd/system/mnt-disks-${device_folder}.mount
[Unit]
Description=mount ${device_name} to ${device_folder}

[Mount]
What=/dev/disk/by-id/google-${device_name}
Where=/mnt/disks/${device_folder}
Type=ext4
Options=defaults

[Install]
WantedBy=multi-user.target
EOF

cat <<"EOF" > /etc/systemd/system/traefik.service
[Unit]
Requires=docker.service mnt-disks-${device_folder}.mount
After=docker.service mnt-disks-${device_folder}.mount

StartLimitIntervalSec=500
StartLimitBurst=5
[Service]
Restart=on-failure
RestartSec=5s

ExecStart=/usr/bin/docker run --rm --name traefik\
  --memory 100m --memory-swap 100m\
  --network local\
  -p 80:80\
  -p 443:443\
  -p 8080:8080\
  -v /var/run/docker.sock:/var/run/docker.sock:ro\
  -v /mnt/disks/${device_folder}/le/:/letsencrypt/\
  traefik:v2.2\
  --api.dashboard=true --api.insecure=true\
  --providers.docker=true --providers.docker.exposedbydefault=false\
  --entrypoints.web.address=:80\
  --entrypoints.websecure.address=:443\
  --certificatesresolvers.le.acme.tlschallenge=true\
  --certificatesresolvers.le.acme.email=tls@utrakr.app\
  --certificatesresolvers.le.acme.storage=/letsencrypt/acme.json

[Install]
WantedBy=multi-user.target
EOF

# setup docker
docker network inspect local || docker network create local

# reload
systemctl daemon-reload

# setup and enable our services
mkdir -p /mnt/disks/${device_folder}
systemctl enable mnt-disks-${device_folder}.mount
systemctl enable traefik.service
systemctl start traefik.service