# Kubernetes Deployment

Deploy CheckStream to Kubernetes with high availability and auto-scaling.

---

## Quick Start

```bash
# Apply manifests
kubectl apply -f checkstream-namespace.yaml
kubectl apply -f checkstream-configmap.yaml
kubectl apply -f checkstream-deployment.yaml
kubectl apply -f checkstream-service.yaml
```

---

## Namespace

```yaml
# checkstream-namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: checkstream
  labels:
    app: checkstream
```

---

## ConfigMap

```yaml
# checkstream-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: checkstream-config
  namespace: checkstream
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 8080
      metrics_port: 9090

    backend:
      url: "https://api.openai.com/v1"

    pipeline:
      ingress:
        enabled: true
        classifiers:
          - prompt_injection
      midstream:
        enabled: true
        classifiers:
          - toxicity

    telemetry:
      logging:
        level: info
        format: json

  default-policy.yaml: |
    version: "1.0"
    name: "default"
    policies:
      - name: block_injection
        trigger:
          classifier: prompt_injection
          threshold: 0.85
        action: stop
```

---

## Secret

```yaml
# checkstream-secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: checkstream-secrets
  namespace: checkstream
type: Opaque
stringData:
  OPENAI_API_KEY: "sk-your-key-here"
```

---

## Deployment

```yaml
# checkstream-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: checkstream
  namespace: checkstream
  labels:
    app: checkstream
spec:
  replicas: 3
  selector:
    matchLabels:
      app: checkstream
  template:
    metadata:
      labels:
        app: checkstream
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      containers:
        - name: checkstream
          image: checkstream/checkstream:latest
          ports:
            - containerPort: 8080
              name: http
            - containerPort: 9090
              name: metrics
          envFrom:
            - secretRef:
                name: checkstream-secrets
          env:
            - name: RUST_LOG
              value: "info"
          volumeMounts:
            - name: config
              mountPath: /app/config.yaml
              subPath: config.yaml
            - name: config
              mountPath: /app/policies/default.yaml
              subPath: default-policy.yaml
            - name: models
              mountPath: /app/models
          resources:
            requests:
              cpu: "500m"
              memory: "1Gi"
            limits:
              cpu: "2"
              memory: "4Gi"
          livenessProbe:
            httpGet:
              path: /health/live
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 8080
            initialDelaySeconds: 20
            periodSeconds: 5
          securityContext:
            readOnlyRootFilesystem: true
            runAsNonRoot: true
            runAsUser: 1000
      volumes:
        - name: config
          configMap:
            name: checkstream-config
        - name: models
          persistentVolumeClaim:
            claimName: checkstream-models
```

---

## Service

```yaml
# checkstream-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: checkstream
  namespace: checkstream
  labels:
    app: checkstream
spec:
  type: ClusterIP
  ports:
    - port: 8080
      targetPort: 8080
      name: http
    - port: 9090
      targetPort: 9090
      name: metrics
  selector:
    app: checkstream
```

---

## Ingress

```yaml
# checkstream-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: checkstream
  namespace: checkstream
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "300"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - checkstream.example.com
      secretName: checkstream-tls
  rules:
    - host: checkstream.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: checkstream
                port:
                  number: 8080
```

---

## PersistentVolumeClaim

```yaml
# checkstream-pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: checkstream-models
  namespace: checkstream
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
  storageClassName: standard
```

---

## Horizontal Pod Autoscaler

```yaml
# checkstream-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: checkstream
  namespace: checkstream
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: checkstream
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

---

## Pod Disruption Budget

```yaml
# checkstream-pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: checkstream
  namespace: checkstream
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: checkstream
```

---

## GPU Support

```yaml
# checkstream-deployment-gpu.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: checkstream-gpu
  namespace: checkstream
spec:
  replicas: 2
  selector:
    matchLabels:
      app: checkstream-gpu
  template:
    spec:
      containers:
        - name: checkstream
          image: checkstream/checkstream:latest-cuda
          env:
            - name: CHECKSTREAM_DEVICE
              value: "cuda"
          resources:
            limits:
              nvidia.com/gpu: 1
```

---

## Init Container for Models

```yaml
# Pre-download models before main container starts
spec:
  initContainers:
    - name: model-loader
      image: checkstream/checkstream:latest
      command: ["checkstream-model-loader", "--config", "/app/config.yaml"]
      volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
        - name: models
          mountPath: /app/models
  containers:
    - name: checkstream
      # ... main container
```

---

## ServiceMonitor (Prometheus Operator)

```yaml
# checkstream-servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: checkstream
  namespace: checkstream
  labels:
    app: checkstream
spec:
  selector:
    matchLabels:
      app: checkstream
  endpoints:
    - port: metrics
      interval: 15s
      path: /metrics
```

---

## Network Policy

```yaml
# checkstream-networkpolicy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: checkstream
  namespace: checkstream
spec:
  podSelector:
    matchLabels:
      app: checkstream
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - port: 8080
    - from:
        - namespaceSelector:
            matchLabels:
              name: monitoring
      ports:
        - port: 9090
  egress:
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
      ports:
        - port: 443
```

---

## Helm Chart (Optional)

```bash
# Install via Helm
helm repo add checkstream https://checkstream.github.io/charts
helm install checkstream checkstream/checkstream \
  --namespace checkstream \
  --create-namespace \
  --set backend.url=https://api.openai.com/v1 \
  --set replicas=3
```

### values.yaml

```yaml
# values.yaml
replicaCount: 3

image:
  repository: checkstream/checkstream
  tag: latest
  pullPolicy: IfNotPresent

backend:
  url: https://api.openai.com/v1

resources:
  requests:
    cpu: 500m
    memory: 1Gi
  limits:
    cpu: 2
    memory: 4Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: checkstream.example.com
      paths:
        - path: /
          pathType: Prefix

persistence:
  enabled: true
  size: 10Gi

metrics:
  enabled: true
  serviceMonitor:
    enabled: true
```

---

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n checkstream
kubectl describe pod checkstream-xxx -n checkstream
```

### View Logs

```bash
kubectl logs -f deployment/checkstream -n checkstream
```

### Check Health

```bash
kubectl exec -it deployment/checkstream -n checkstream -- curl http://localhost:8080/health/ready
```

### Port Forward for Testing

```bash
kubectl port-forward svc/checkstream 8080:8080 -n checkstream
```

---

## Next Steps

- [Docker Deployment](docker.md) - Local development with Docker
- [Configuration Reference](../configuration/proxy.md) - All configuration options
- [Metrics Reference](../reference/metrics.md) - Monitoring and alerting
