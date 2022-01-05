#!/bin/bash

docker stop loki-1 || :
docker stop loki-2 || :

docker run -d --rm --name loki-1 -v $(pwd):/mnt/config -p 3100:3100 -p 9096:9096 grafana/loki:2.4.1 -config.file=/mnt/config/loki-config.yaml
docker run -d --rm --name loki-2 -v $(pwd):/mnt/config -p 3101:3100 -p 9097:9096 grafana/loki:2.4.1 -config.file=/mnt/config/loki-config.yaml


