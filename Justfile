# meta
project_id := "utrakr"
app := "utrakr-api"

# version
git_hash := "$(git rev-parse --short HEAD)"
git_dirty := "$([[ $(git diff --stat) != '' ]] && echo '-dirty')"
app_version := git_hash + git_dirty

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

test:
    #!/bin/bash

    curl --fail --cookie-jar /tmp/c --cookie /tmp/c -v http://localhost:8080/id
