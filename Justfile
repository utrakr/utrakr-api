# meta
app := "utrakr-api"
project_id := "utrakr"
zone := "us-central1-f"

# version
git_hash := "$(git rev-parse --short HEAD)"
git_dirty := "$([[ $(git diff --stat) != '' ]] && echo '-dirty')"
app_version := git_hash + git_dirty

fmt:
    cargo fmt
    terraform fmt -recursive

gcloud-set-project:
    gcloud config set project utrakr
gcloud-auth: gcloud-set-project
    gcloud auth login
    gcloud auth application-default login

setup-dev:
    docker rm -f {{app}}-redis || :
    docker run --name {{app}}-redis -d\
      -v ~/docker/data/{{app}}-redis/:/data\
      -p 6379:6379\
      redis:6.0\
      redis-server --appendonly yes

docker-build:
    docker build -t us.gcr.io/{{project_id}}/{{app}}:{{app_version}} .
docker-push: docker-build
    docker push us.gcr.io/{{project_id}}/{{app}}:{{app_version}}
docker-run: docker-build
    docker run --rm -it us.gcr.io/{{project_id}}/{{app}}:{{app_version}}

ssh:
    gcloud compute ssh utrakr-api --tunnel-through-iap --zone {{zone}}
deploy: docker-push
    gcloud compute ssh utrakr-api --tunnel-through-iap --zone {{zone}} --command "VERSION={{app_version}} bash -s" < scripts/app-deploy.sh
tail-event-log:
    gcloud compute ssh utrakr-api --tunnel-through-iap --zone {{zone}} --command 'tail -f "$(find /mnt/disks/app_data/utrakr-api/event-logs/ -type f -printf "%T+ %p\\n" | sort | tail -n 1 | awk "{print \$2}")"'

data-sync:
    gsutil -m rsync -r gs://utrakr-prod-utrakr-api-data ./data

test:
    #!/bin/bash
    set -euo pipefail
    IFS=$'\n\t'
    loc="https://utrakr.app/"

    reps="$(curl -s --fail -d '{"long_url":"http://example.com"}' "${loc}")"
    echo "${reps}" | jq -c .
    curl --fail --cookie-jar /tmp/cookie --cookie /tmp/cookie -v "$(echo "${reps}" | jq -r .data.micro_url)"
