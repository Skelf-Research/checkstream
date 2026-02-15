# Security & Privacy

CheckStream is designed with security and privacy as foundational principles. This document outlines our security model, data handling practices, and compliance certifications.

---

## Security Model

### Threat Model

CheckStream defends against:

1. **Adversarial Users**
   - Prompt injection and jailbreak attempts
   - Multi-turn boundary erosion attacks
   - Obfuscated harmful inputs

2. **Data Exfiltration**
   - PII/PHI leakage in LLM responses
   - Credential spillage (API keys, passwords)
   - Proprietary information disclosure

3. **Supply Chain Attacks**
   - Compromised classifier models
   - Tampered policy bundles
   - Malicious dependencies

4. **Insider Threats**
   - Unauthorized policy modifications
   - Telemetry data access
   - Audit log tampering

5. **Side-Channel Attacks**
   - Timing attacks to infer sensitive info
   - Resource exhaustion (DoS)

---

## Architecture Security

### Data Plane Isolation

**Principle**: LLM traffic never touches the control plane.

```
┌─────────────────────────────────────────┐
│  Customer VPC                           │
│  ┌───────────────────────────────────┐  │
│  │  CheckStream Proxy/Sidecar        │  │
│  │  (enforcement node)               │  │
│  │  ┌─────────────┐                  │  │
│  │  │ LLM Traffic │ ← STAYS HERE     │  │
│  │  │ (tokens)    │   (never leaves) │  │
│  │  └─────────────┘                  │  │
│  └───────────────────────────────────┘  │
│           │                              │
│           │ (metadata only, over mTLS)   │
└───────────┼──────────────────────────────┘
            │
            ▼
  ┌─────────────────────┐
  │  Control Plane      │
  │  (policy, metrics)  │
  └─────────────────────┘
```

**What crosses the boundary**:
- Policy bundles (control plane → nodes)
- Metrics/telemetry (nodes → control plane, optional)
- Health checks and attestations

**What never leaves customer VPC**:
- User prompts
- LLM responses (tokens)
- Retrieved context
- Tool call arguments

### Encryption

#### Data in Transit
- **mTLS everywhere**: All control plane ↔ node communication uses mutual TLS
- **Certificate rotation**: Automatic 7-day cert rotation via SPIFFE/SPIRE
- **TLS 1.3**: Minimum version, modern ciphers only
- **Perfect forward secrecy**: Ephemeral key exchange (ECDHE)

#### Data at Rest
- **Policy bundles**: Signed and optionally encrypted at rest
- **Telemetry storage**: AES-256 encryption, per-tenant keys
- **Audit logs**: Append-only, encrypted, immutable

### Proxy Security Features

The CheckStream proxy implements multiple security hardening measures:

#### SSRF Protection

Backend URLs are validated to prevent Server-Side Request Forgery attacks:

- **HTTPS-only**: Only HTTPS URLs are permitted in production (HTTP allowed in dev mode)
- **Blocked hosts**: Localhost, loopback addresses (127.0.0.1, ::1), and cloud metadata endpoints (169.254.169.254) are blocked
- **Private IP blocking**: RFC 1918 private ranges (10.x.x.x, 172.16.x.x, 192.168.x.x) are blocked
- **Domain allowlisting**: Optional configuration to restrict backend URLs to specific domains

```yaml
# Enable development mode to allow localhost backends
# Set environment variable: CHECKSTREAM_DEV_MODE=1
```

#### Timing Attack Protection

Admin API key validation uses constant-time comparison to prevent timing attacks that could extract credentials character-by-character.

#### Request Size Limits

Request bodies are limited to 10MB to prevent memory exhaustion attacks.

#### Security Headers

All responses include security headers:

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `X-XSS-Protection` | `1; mode=block` | XSS filter |
| `Cache-Control` | `no-store` | Prevent caching sensitive responses |
| `Content-Security-Policy` | `default-src 'none'` | Strict CSP |

#### Secure ID Generation

Request IDs and audit event IDs use cryptographically secure UUID v4 generation, preventing ID prediction attacks.

#### Configuration Security

- **YAML size limits**: Configuration files are limited to 1MB to prevent YAML bomb (billion laughs) attacks
- **Tenant isolation**: Unknown tenant IDs are not logged to prevent enumeration attacks

### Authentication & Authorization

#### API Authentication
```bash
# API key (for machines)
curl https://control.checkstream.ai/v1/policies \
  -H "Authorization: Bearer cs_key_abc123..."

# SSO (for users)
# Supports: SAML 2.0, OIDC, OAuth 2.0
# Providers: Okta, Auth0, Azure AD, Google Workspace
```

#### Role-Based Access Control (RBAC)

| Role | Permissions |
|------|-------------|
| **Org Admin** | All operations, user management, billing |
| **Risk Officer** | Policy approval, audit access, compliance reports |
| **Security Analyst** | Dashboard read, incident investigation, telemetry |
| **Engineer** | Node deployment, policy editing (pending approval), logs |
| **Auditor** | Read-only access to all logs and dashboards |
| **Support** | Limited troubleshooting, no data access |

#### Multi-Party Approval

For critical policy changes:
```yaml
policy_approval:
  required_approvers: 2
  roles:
    - risk_officer
    - chief_risk_officer
  timeout: 48h
```

---

## Privacy

### Privacy Modes

CheckStream offers three telemetry modes:

#### 1. None (Maximum Privacy)

```yaml
telemetry:
  mode: none
```

- **No data** leaves customer VPC
- All enforcement local
- Audits stored on-premise
- Control plane used only for policy distribution

#### 2. Aggregate (Recommended)

```yaml
telemetry:
  mode: aggregate
```

**What's sent**:
- Metrics: request counts, token counts, latency percentiles
- Rule trigger counts (no context)
- Decision distributions (allow/redact/stop)
- Node health metrics

**What's NOT sent**:
- User prompts
- LLM responses
- Context snippets
- User identifiers

**Example payload**:
```json
{
  "node_id": "proxy-eu-001",
  "interval": "2024-01-15T10:00:00Z/PT1M",
  "requests": 1523,
  "tokens": 45690,
  "decisions": {
    "allow": 1489,
    "redact": 28,
    "stop": 6
  },
  "rules_triggered": {
    "promotional_balance": 12
  }
}
```

#### 3. Full Evidence (Opt-In)

```yaml
telemetry:
  mode: full_evidence
  privacy:
    hash_spans: true
    max_span_length: 50
    redact_pii: true
```

**What's sent**:
- Per-decision records with:
  - Rule ID, action, confidence
  - **Hashed** context spans (not plaintext)
  - Regulation citations
  - Latency metrics
- Hash-chain for audit integrity

**Example payload**:
```json
{
  "stream_id": "req_abc123",
  "timestamp": "2024-01-15T10:05:23Z",
  "decision": {
    "rule_id": "advice_boundary",
    "action": "inject_disclaimer",
    "confidence": 0.87
  },
  "context_hash": "sha256:a1b2c3...",  // Hash, not plaintext
  "policy_bundle": "v2.3.1",
  "hash_chain": "prev:def456,curr:abc789"
}
```

### PII Minimization

**Automatic redaction in telemetry**:
- Email addresses → `[EMAIL]`
- Phone numbers → `[PHONE]`
- SSNs → `[SSN]`
- Names → `[NAME]` (if detected)

**Hashing**:
- SHA-256 with per-tenant salt
- Irreversible; control plane cannot recover plaintext

### Data Residency

**Control Plane Regions**:
- US: us-east-1 (Virginia), us-west-2 (Oregon)
- EU: eu-west-2 (London), eu-central-1 (Frankfurt)
- APAC: ap-southeast-1 (Singapore)

**Selection**:
```bash
checkstream org configure \
  --control-plane-region eu-west-2
```

**Enforcement nodes** run wherever you deploy them (your VPC, on-prem).

### Data Retention

| Data Type | Retention | Configurable? |
|-----------|-----------|---------------|
| **Metrics (aggregate)** | 13 months | Yes (6-24 months) |
| **Evidence records** | 7 years | Yes (for compliance) |
| **Audit logs** | 7 years | No (immutable) |
| **Policy versions** | Forever | No (Git history) |

**Deletion**:
```bash
# Delete all telemetry for a specific period
checkstream telemetry delete --period 2024-01

# Export before deletion
checkstream telemetry export --period 2024-01 --format json
```

---

## Compliance Certifications

### Current

- **SOC 2 Type II** (Security, Availability, Confidentiality)
- **ISO 27001** (Information Security Management)
- **GDPR-compliant** (EU data protection)
- **CCPA-compliant** (California consumer privacy)

### In Progress

- **ISO 27701** (Privacy Information Management)
- **FedRAMP Moderate** (US government cloud security)
- **HIPAA BAA** (Healthcare data)

### Framework Compliance

**NIST Cybersecurity Framework**:
- Identify: Asset inventory, risk assessment
- Protect: Access control, encryption, training
- Detect: Anomaly detection, logging
- Respond: Incident response plan, backups
- Recover: Disaster recovery, business continuity

**CIS Controls**:
- Implemented: 16 of 18 controls
- Partial: 2 controls (physical security, supply chain)

---

## Security Operations

### Vulnerability Management

**Disclosure**:
- Responsible disclosure: security@checkstream.ai
- PGP key available: https://checkstream.ai/.well-known/security.txt
- Response SLA: 48 hours for initial triage

**Bug Bounty**:
- HackerOne program: https://hackerone.com/checkstream
- Rewards: $100 - $10,000 depending on severity

**Patching**:
- Critical vulnerabilities: Hotfix within 24h
- High: Patch within 7 days
- Medium: Patch within 30 days
- Automatic updates for nodes (configurable)

### Incident Response

**Process**:
1. Detection (automated alerts + manual reporting)
2. Triage (severity assessment, impact analysis)
3. Containment (isolate affected systems)
4. Eradication (remove threat, patch vulnerability)
5. Recovery (restore service, validate)
6. Post-Incident Review (root cause, remediation)

**Customer Notification**:
- Critical incidents: Within 4 hours
- High incidents: Within 24 hours
- Incident report provided within 7 days

### Penetration Testing

- **Frequency**: Quarterly external pen tests
- **Scope**: Control plane, API, web UI
- **Vendor**: Certified third-party (CREST, OSCP)
- **Reports**: Available to enterprise customers under NDA

---

## Secure Development

### Supply Chain Security

**Dependencies**:
- Automated vulnerability scanning (Snyk, Dependabot)
- Pin all dependency versions
- Review all PRs from external contributors
- SBOM (Software Bill of Materials) available

**Classifiers & Models**:
- Models signed with PGP key
- Hash verification before loading
- Model cards with training data provenance
- No models trained on customer data without explicit consent

**Build Process**:
- Reproducible builds
- Signed container images (Sigstore)
- SLSA Level 3 compliance (in progress)

### Code Security

- **Static analysis**: SonarQube, Semgrep
- **Dependency scanning**: GitHub Advanced Security
- **Secret detection**: GitGuardian, Talisman
- **Container scanning**: Trivy, Grype

### CI/CD Security

- **Signed commits**: Required for all merges
- **Branch protection**: Main branch requires 2 approvals
- **Automated tests**: 95%+ code coverage
- **Security gate**: Fails build on high/critical vulns

---

## Audit & Logging

### Audit Trail

**What's logged**:
- All API calls (who, what, when, from where)
- Policy changes (create, update, delete, deploy)
- User actions (login, role changes, approvals)
- Node events (connect, disconnect, version changes)
- Incidents (detection, response, resolution)

**Log format** (JSON):
```json
{
  "timestamp": "2024-01-15T14:30:00Z",
  "event_type": "policy_deployed",
  "actor": {
    "user_id": "user_abc123",
    "email": "risk@acme-bank.com",
    "ip": "203.0.113.45",
    "user_agent": "checkstream-cli/1.2.0"
  },
  "resource": {
    "type": "policy",
    "id": "consumer-duty-v2.3.1"
  },
  "action": "deploy",
  "target": {
    "fleet": "production-eu-west-2",
    "nodes": 12
  },
  "outcome": "success",
  "metadata": {
    "approval_ticket": "RISK-1234"
  },
  "signature": "sha256:..."
}
```

### Immutability

Audit logs are **append-only**:
- Write-once storage (AWS S3 Object Lock, GCS Retention Policy)
- Hash-chained: Each entry includes hash of previous entry
- Tampering detectable via integrity verification

**Verification**:
```bash
checkstream audit verify --period 2024-01
# Output: ✓ All 12,453 audit entries verified. No tampering detected.
```

### SIEM Integration

Export to:
- **Splunk**: HTTP Event Collector (HEC)
- **Datadog**: Logs API
- **Chronicle**: Ingestion API
- **Azure Sentinel**: Log Analytics
- **Generic**: Syslog, S3, webhook

---

## Network Security

### Firewall Rules

**Ingress** (to proxy/sidecar nodes):
- Port 8080: LLM API traffic (authenticated)
- Port 9090: Metrics (internal only)
- Port 443: Control plane sync (mTLS)

**Egress** (from nodes):
- LLM backend (e.g., api.openai.com:443)
- Control plane (control.checkstream.ai:443)
- Model CDN (models.checkstream.ai:443)

**Recommended** (zero trust):
```yaml
# Only allow specific destinations
egress_rules:
  - destination: api.openai.com
    port: 443
    protocol: HTTPS
  - destination: control.checkstream.ai
    port: 443
    protocol: HTTPS
```

### DDoS Protection

- Rate limiting per source IP
- Connection throttling
- CloudFlare Enterprise (for control plane)

---

## Customer Responsibilities

### Shared Responsibility Model

| Component | CheckStream | Customer |
|-----------|-------------|----------|
| **Control plane infrastructure** | ✓ | |
| **Control plane security** | ✓ | |
| **Enforcement node software** | ✓ | |
| **Enforcement node infrastructure** | | ✓ |
| **API keys / credentials** | | ✓ |
| **Policy definitions** | | ✓ |
| **User access management** | | ✓ |
| **Network security (VPC)** | | ✓ |

### Best Practices for Customers

1. **Rotate API keys** every 90 days
2. **Use SSO** instead of password auth
3. **Enable MFA** for all admin users
4. **Review audit logs** monthly
5. **Run nodes in private subnets** (no public IPs)
6. **Encrypt backend API keys** (AWS Secrets Manager, etc.)
7. **Monitor node health** and set up alerts
8. **Test disaster recovery** procedures quarterly

---

## Disaster Recovery

### Backup

**Control Plane**:
- **Policies**: Git-backed, replicated across regions
- **Audit logs**: Replicated to 3 regions
- **Telemetry**: Daily backups, 30-day retention

**Nodes**:
- **Stateless**: No local state to back up
- **Config**: Stored in control plane, re-fetch on restart

### RTO/RPO

| Service | RTO (Recovery Time) | RPO (Recovery Point) |
|---------|---------------------|----------------------|
| **Control plane** | < 1 hour | < 15 minutes |
| **Policy distribution** | < 5 minutes | < 1 minute |
| **Telemetry ingestion** | < 30 minutes | < 5 minutes |
| **Enforcement nodes** | Immediate (restart) | None (stateless) |

### Failover

**Multi-region** control plane:
- Active-active in US/EU/APAC
- Automatic DNS failover
- No manual intervention required

**Node resilience**:
- Nodes cache policies locally
- Continue enforcement even if control plane unavailable
- Telemetry queued and flushed when connectivity restored

---

## Security Roadmap

### Q2 2024
- [ ] FIPS 140-2 compliant cryptography
- [ ] Hardware security module (HSM) integration
- [ ] Advanced threat detection (ML-based anomaly detection)

### Q3 2024
- [ ] FedRAMP Moderate authorization
- [ ] Private control plane (customer-hosted option)
- [ ] Customer-managed encryption keys (CMEK)

### Q4 2024
- [ ] ISO 27701 (Privacy) certification
- [ ] HIPAA BAA availability
- [ ] Zero-trust node attestation

---

## Contact

**Security Team**: security@checkstream.ai
**Compliance Team**: compliance@checkstream.ai
**Privacy Officer**: privacy@checkstream.ai

**PGP Key**: https://checkstream.ai/.well-known/security.txt
**Security Portal**: https://security.checkstream.ai

---

## Next Steps

- **Deploy securely**: [Getting Started](getting-started.md)
- **Configure RBAC**: [Control Plane](control-plane.md)
- **Review policies**: [Policy Engine](policy-engine.md)
- **Integrate APIs**: [API Reference](api-reference.md)
