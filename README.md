# Loki-Federation

Inspired by Thanos querier prometheus federation applied to loki.

In short, it gathers the queries from a set of loki backends and return an
aggregated result.

It is fully stateless and horizontally scalable.

## Getting started

To get started you need to create a simple configuration file with the following
minimal configuration.

```toml
[server]
port = 8080
bind_address = "0.0.0.0"

[datasources]
name = "static-http"
urls = ["http://localhost:3100", "http://localhost:3101"]

[debug]
log_level = "info"
```

and then you can start `Loki-federation` using the following comand:

```bash
$ ./loki-federation -c config.toml
```

This will start an http server listening on 0.0.0.0:8080 and exposing the same
endpoint as a Loki querier.

## Currently supported endpoints

- GET /ready
- GET /loki/api/v1/query
- GET /loki/api/v1/query_range
- GET /loki/api/v1/labels
- GET /loki/api/v1/label (alias for `GET /loki/api/v1/labels` available in Loki and used by grafana)
- GET /loki/api/v1/label/{label}/values
- GET /loki/api/v1/series
- POST /loki/api/v1/series

The federated response are not providing stats as part of the response.

## Roadmap

- [ ] enhance core api testing
- [ ] non regression rest testing and compatibility test to ensure Loki API
      compliance
- [ ] add support for GET /loki/api/v1/tail
- [ ] add runtime discovery of backends through kubernetes selectors
- [ ] add retry pattern and retry configuration to query backends
- [ ] add deduplication configuration (cf https://thanos.io/tip/components/query.md/#deduplication)
- [ ] add https support
- [ ] explore if GRPC can be used to retrieve logs from backends
