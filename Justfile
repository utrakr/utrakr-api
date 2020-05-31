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

test:
    #!/bin/bash

    curl --fail --cookie-jar /tmp/c --cookie /tmp/c -v http://localhost:8080/id
