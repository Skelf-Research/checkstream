# Docker Deployment

Deploy CheckStream using Docker and Docker Compose.

---

## Quick Start

```bash
docker run -d \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/app/config.yaml \
  -v $(pwd)/policies:/app/policies \
  -e OPENAI_API_KEY=$OPENAI_API_KEY \
  checkstream/checkstream:latest
```

---

## Docker Compose

### Basic Setup

```yaml
# docker-compose.yml
version: '3.8'

services:
  checkstream:
    image: checkstream/checkstream:latest
    ports:
      - "8080:8080"   # Proxy
      - "9090:9090"   # Metrics
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./policies:/app/policies
      - ./models:/app/models
    environment:
      - RUST_LOG=info
      - CHECKSTREAM_BACKEND_URL=https://api.openai.com/v1
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### With GPU Support

```yaml
version: '3.8'

services:
  checkstream:
    image: checkstream/checkstream:latest-cuda
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./policies:/app/policies
    environment:
      - RUST_LOG=info
      - CHECKSTREAM_DEVICE=cuda
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
```

### Production Stack

```yaml
version: '3.8'

services:
  checkstream:
    image: checkstream/checkstream:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./policies:/app/policies
      - checkstream-models:/app/models
      - checkstream-audit:/app/audit
    environment:
      - RUST_LOG=info
      - CHECKSTREAM_BACKEND_URL=https://api.openai.com/v1
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 4G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health/ready"]
      interval: 10s
      timeout: 5s
      retries: 3
    depends_on:
      - prometheus

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
    depends_on:
      - prometheus

volumes:
  checkstream-models:
  checkstream-audit:
  prometheus-data:
  grafana-data:
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `CHECKSTREAM_HOST` | Bind address | `0.0.0.0` |
| `CHECKSTREAM_PORT` | Proxy port | `8080` |
| `CHECKSTREAM_METRICS_PORT` | Metrics port | `9090` |
| `CHECKSTREAM_BACKEND_URL` | LLM backend URL | Required |
| `CHECKSTREAM_POLICY_PATH` | Policy file path | `/app/policies/default.yaml` |
| `CHECKSTREAM_DEVICE` | ML device | `auto` |
| `HF_HOME` | HuggingFace cache | `/app/models` |

### Volume Mounts

| Path | Purpose |
|------|---------|
| `/app/config.yaml` | Main configuration |
| `/app/policies/` | Policy files |
| `/app/models/` | ML model cache |
| `/app/audit/` | Audit logs |

---

## Building the Image

### Standard Build

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release --features ml-models

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/checkstream-proxy /usr/local/bin/
COPY --from=builder /app/config.yaml /app/
COPY --from=builder /app/policies /app/policies

EXPOSE 8080 9090

CMD ["checkstream-proxy", "--config", "/app/config.yaml"]
```

### With CUDA

```dockerfile
# Dockerfile.cuda
FROM nvidia/cuda:12.0-runtime-ubuntu22.04 as runtime

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/checkstream-proxy /usr/local/bin/

ENV CHECKSTREAM_DEVICE=cuda

EXPOSE 8080 9090

CMD ["checkstream-proxy", "--config", "/app/config.yaml"]
```

### Build Commands

```bash
# Standard
docker build -t checkstream:latest .

# With CUDA
docker build -f Dockerfile.cuda -t checkstream:latest-cuda .

# Multi-platform
docker buildx build --platform linux/amd64,linux/arm64 -t checkstream:latest .
```

---

## Pre-loading Models

Download models before starting:

```yaml
# docker-compose.yml
services:
  model-loader:
    image: checkstream/checkstream:latest
    command: ["checkstream-model-loader", "--config", "/app/config.yaml"]
    volumes:
      - ./config.yaml:/app/config.yaml
      - checkstream-models:/app/models

  checkstream:
    image: checkstream/checkstream:latest
    depends_on:
      model-loader:
        condition: service_completed_successfully
    volumes:
      - checkstream-models:/app/models
```

Or use an init container:

```bash
docker run --rm \
  -v checkstream-models:/app/models \
  checkstream/checkstream:latest \
  checkstream-model-loader --config /app/config.yaml
```

---

## Networking

### Behind a Reverse Proxy

```yaml
# docker-compose.yml
services:
  nginx:
    image: nginx:latest
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./certs:/etc/nginx/certs

  checkstream:
    image: checkstream/checkstream:latest
    expose:
      - "8080"
    # No external ports
```

```nginx
# nginx.conf
upstream checkstream {
    server checkstream:8080;
}

server {
    listen 443 ssl;

    ssl_certificate /etc/nginx/certs/cert.pem;
    ssl_certificate_key /etc/nginx/certs/key.pem;

    location / {
        proxy_pass http://checkstream;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_buffering off;
    }
}
```

---

## Logging

### JSON Logs to stdout

```yaml
services:
  checkstream:
    environment:
      - RUST_LOG=info
      - CHECKSTREAM_LOG_FORMAT=json
```

### Log Aggregation

```yaml
services:
  checkstream:
    logging:
      driver: "fluentd"
      options:
        fluentd-address: "localhost:24224"
        tag: "checkstream"
```

---

## Health Checks

### Docker Health Check

```yaml
services:
  checkstream:
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health/ready"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s
```

### Load Balancer Health

```bash
# Liveness (is process running?)
curl http://localhost:8080/health/live

# Readiness (is it ready to serve?)
curl http://localhost:8080/health/ready
```

---

## Scaling

### Docker Compose

```bash
docker compose up -d --scale checkstream=3
```

### With Load Balancer

```yaml
services:
  haproxy:
    image: haproxy:latest
    ports:
      - "8080:8080"
    volumes:
      - ./haproxy.cfg:/usr/local/etc/haproxy/haproxy.cfg
    depends_on:
      - checkstream

  checkstream:
    image: checkstream/checkstream:latest
    deploy:
      replicas: 3
    expose:
      - "8080"
```

---

## Troubleshooting

### Check Logs

```bash
docker compose logs checkstream
docker compose logs -f checkstream  # Follow
```

### Enter Container

```bash
docker compose exec checkstream /bin/bash
```

### Check Health

```bash
docker compose exec checkstream curl http://localhost:8080/health/ready
```

### Resource Usage

```bash
docker stats checkstream
```

---

## Next Steps

- [Kubernetes Deployment](kubernetes.md) - Deploy to Kubernetes
- [Configuration Reference](../configuration/proxy.md) - All configuration options
- [Metrics Reference](../reference/metrics.md) - Monitoring setup
