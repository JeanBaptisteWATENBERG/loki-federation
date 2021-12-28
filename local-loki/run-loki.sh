#!/bin/sh

docker stop loki-1 || :
docker stop loki-2 || :

docker run -d --rm --name loki-1 -v $(pwd):/mnt/config -p 3100:3100 -p 9096:9096 grafana/loki:2.4.1 -config.file=/mnt/config/loki-config.yaml
docker run -d --rm --name loki-2 -v $(pwd):/mnt/config -p 3101:3100 -p 9097:9096 grafana/loki:2.4.1 -config.file=/mnt/config/loki-config.yaml

epochInNanoseconds="$(date +%s -u)000000000"

values1=""
values2=""
for number in {1..1000}
do
	if [ $(( $number % 10 )) -ne 0 ]
	then
		values1+="[ \"$(($epochInNanoseconds - $number * 1000000000))\", \"$number\" ]"
		if [ $number -lt 999 ]
	        then
	                values1+=","
	        fi

	fi
	if [ $number -ne 42 ]
	then
		values2+="[ \"$(($epochInNanoseconds - $number * 1000000000))\", \"$number\" ]"
		if [ $number -lt 1000 ]
	        then
	                values2+=","
	        fi
	fi
done

bodyHead="{\"streams\": [{ \"stream\": { \"service\": \"test\" }, \"values\": [ "
bodyFoot=" ] }]}"

body1="$bodyHead$values1$bodyFoot"
body2="$bodyHead$values2$bodyFoot"

curl -v -H "Content-Type: application/json" -XPOST -s "http://localhost:3100/loki/api/v1/push" --data-raw \
  "$body1"
curl -v -H "Content-Type: application/json" -XPOST -s "http://localhost:3101/loki/api/v1/push" --data-raw \
  "$body2"

