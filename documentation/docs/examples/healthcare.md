# Healthcare Compliance Example

Deploy CheckStream for HIPAA-compliant healthcare AI applications.

---

## Scenario

A healthcare organization deploys an AI assistant for:
- General health education
- Appointment scheduling assistance
- Symptom information (not diagnosis)

Requirements:
- Block PHI (Protected Health Information) disclosure
- Add medical disclaimers
- Maintain HIPAA-compliant audit trail
- Prevent medical diagnosis

---

## Configuration

### config.yaml

```yaml
server:
  host: "0.0.0.0"
  port: 8080

backend:
  url: "https://api.openai.com/v1"

pipeline:
  ingress:
    enabled: true
    classifiers:
      - phi_request
      - diagnosis_request
      - pii_detector
    threshold: 0.85

  midstream:
    enabled: true
    token_holdback: 20
    classifiers:
      - phi_detector
      - medical_advice
      - pii_detector
    chunk_threshold: 0.80

  egress:
    enabled: true
    audit: true
    classifiers:
      - medical_topic

policy:
  path: "./policies/hipaa.yaml"

telemetry:
  audit:
    enabled: true
    path: "./audit/hipaa"
    hash_chain: true
    retention_days: 2190  # 6 years (HIPAA requirement)
    encryption:
      enabled: true
      key_env: "AUDIT_ENCRYPTION_KEY"
```

### classifiers.yaml

```yaml
classifiers:
  # Tier A - Pattern matching for PHI
  phi_detector:
    tier: A
    type: pattern
    patterns:
      - name: mrn
        pattern: '\b(MRN|Medical Record|Patient ID)[\s:#]+\S+'
        score: 1.0
      - name: ssn
        pattern: '\b\d{3}-\d{2}-\d{4}\b'
        score: 1.0
      - name: dob_explicit
        pattern: '\b(DOB|Date of Birth|Born)[\s:]+\d{1,2}[/-]\d{1,2}[/-]\d{2,4}'
        score: 0.95
      - name: diagnosis_code
        pattern: '\b[A-Z]\d{2}(\.\d{1,2})?\b'  # ICD-10 codes
        score: 0.9

  pii_detector:
    tier: A
    type: pattern
    patterns:
      - name: email
        pattern: '\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b'
        score: 0.85
      - name: phone
        pattern: '\b\d{3}[-.]?\d{3}[-.]?\d{4}\b'
        score: 0.85

  # Tier B - ML classifiers
  phi_request:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/phi-request-detector"
      quantization: int8

  diagnosis_request:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/diagnosis-request-detector"
      quantization: int8

  medical_advice:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/medical-advice-classifier"
      quantization: int8
    labels:
      0: general_info
      1: medical_advice
      2: diagnosis

  medical_topic:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/medical-topic-classifier"
```

---

## Policy

### policies/hipaa.yaml

```yaml
version: "1.0"
name: "hipaa-compliance"
description: "HIPAA Privacy and Security Rule compliant policy"

policies:
  # ============================================
  # INGRESS - Block PHI requests
  # ============================================

  # 45 CFR 164.502 - No PHI disclosure without authorization
  - name: block_phi_request
    phase: ingress
    priority: 100
    trigger:
      any:
        - classifier: phi_request
          threshold: 0.85
        - pattern: '(patient|medical|health)\s+record'
          case_insensitive: true
        - pattern: '(diagnosis|treatment|prescription)\s+(for|of)\s+'
          case_insensitive: true
    action: stop
    message: |
      I cannot access, retrieve, or discuss specific patient records or
      protected health information (PHI).

      For information about your medical records, please contact your
      healthcare provider directly or use the patient portal.
    regulation: "HIPAA 45 CFR 164.502"

  # Block diagnosis requests
  - name: block_diagnosis_request
    phase: ingress
    priority: 95
    trigger:
      classifier: diagnosis_request
      threshold: 0.8
    action: stop
    message: |
      I'm not able to provide medical diagnoses. Only licensed healthcare
      professionals can diagnose medical conditions.

      If you're experiencing symptoms, please consult with a healthcare
      provider or call emergency services if urgent.
    regulation: "Medical Practice Guidelines"

  # ============================================
  # MIDSTREAM - Redact any PHI in responses
  # ============================================

  # 45 CFR 164.502(b) - Minimum Necessary
  - name: redact_phi
    phase: midstream
    priority: 100
    trigger:
      classifier: phi_detector
      threshold: 0.85
    action:
      - type: redact
        replacement: "[PHI REDACTED]"
      - type: audit
        regulation: "HIPAA 45 CFR 164.502(b)"
        severity: high

  # Redact PII
  - name: redact_pii
    phase: midstream
    priority: 95
    trigger:
      classifier: pii_detector
      threshold: 0.9
    action:
      - type: redact
        replacement: "[PII REDACTED]"
      - type: log
        level: warn

  # Redact specific diagnosis
  - name: redact_diagnosis
    phase: midstream
    priority: 90
    trigger:
      classifier: medical_advice
      label: diagnosis
      confidence: 0.75
    action:
      - type: redact
        replacement: "[MEDICAL ASSESSMENT REMOVED - please consult a healthcare provider]"
      - type: audit
        regulation: "Medical Practice Guidelines"

  # ============================================
  # EGRESS - Add disclaimers
  # ============================================

  # Medical information disclaimer
  - name: medical_disclaimer
    phase: egress
    priority: 70
    trigger:
      classifier: medical_topic
      threshold: 0.4
    action:
      type: inject
      position: end
      content: |

        ---
        **Medical Disclaimer**

        This information is provided for educational purposes only and is
        not intended to be a substitute for professional medical advice,
        diagnosis, or treatment.

        Always seek the advice of your physician or other qualified health
        provider with any questions you may have regarding a medical condition.

        If you think you may have a medical emergency, call your doctor or
        emergency services immediately.
    regulation: "Medical Disclaimer Best Practice"

  # Symptom information disclaimer
  - name: symptom_disclaimer
    phase: egress
    priority: 65
    trigger:
      pattern: '(symptom|symptoms|sign|signs)\s+of'
    action:
      type: inject
      position: end
      content: |

        *Symptom information is general in nature. Individual symptoms may
        indicate different conditions. Please consult a healthcare provider
        for proper evaluation.*

  # Audit all medical interactions
  - name: audit_medical
    phase: egress
    priority: 50
    trigger:
      classifier: medical_topic
      threshold: 0.2
    action:
      type: audit
      regulation: "HIPAA 45 CFR 164.312(b)"
      include:
        - input_hash
        - output_hash
        - classifier_scores
        - timestamp
        - session_id
      retention_days: 2190
```

---

## Usage Examples

### Allowed Interactions

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="your-key"
)

# General health education - ALLOWED
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "What are common symptoms of the flu?"
    }]
)
# Returns educational content with medical disclaimer
```

### Blocked Interactions

```python
# PHI request - BLOCKED
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "What's in John Smith's medical record?"
    }]
)
# Returns: "I cannot access, retrieve, or discuss specific patient records..."

# Diagnosis request - BLOCKED
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "I have a headache and fever. What do I have?"
    }]
)
# Returns: "I'm not able to provide medical diagnoses..."
```

---

## Audit Trail (HIPAA Compliant)

### Required Audit Elements (45 CFR 164.312(b))

```json
{
  "id": "audit-hipaa-2024011510300001",
  "timestamp": "2024-01-15T10:30:00.123Z",
  "hash_chain": {
    "previous": "sha256:abc123...",
    "current": "sha256:def456..."
  },
  "session_id": "session-789",
  "input_hash": "sha256:input...",
  "output_hash": "sha256:output...",
  "classifications": {
    "medical_topic": 0.85,
    "phi_detector": 0.0,
    "diagnosis_request": 0.12
  },
  "actions": [
    {
      "type": "inject",
      "rule": "medical_disclaimer"
    }
  ],
  "access_type": "read",
  "regulations": ["HIPAA 45 CFR 164.312(b)"]
}
```

### Audit Retention

```yaml
telemetry:
  audit:
    retention_days: 2190  # 6 years (HIPAA minimum)
    encryption:
      enabled: true
      algorithm: "AES-256-GCM"
    backup:
      enabled: true
      location: "s3://hipaa-audit-backup/"
```

---

## Security Considerations

### Data Encryption

```yaml
# In-transit
server:
  tls:
    enabled: true
    cert_path: "/certs/server.crt"
    key_path: "/certs/server.key"
    min_version: "1.2"

# At-rest (audit logs)
telemetry:
  audit:
    encryption:
      enabled: true
      key_env: "AUDIT_ENCRYPTION_KEY"
```

### Access Controls

```yaml
admin:
  auth:
    enabled: true
    type: jwt
    issuer: "https://auth.hospital.com"
    audience: "checkstream-api"
  roles:
    admin:
      - reload
      - audit-read
      - audit-verify
    operator:
      - health
      - metrics
```

---

## Deployment

### Kubernetes with Security Context

```yaml
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
    fsGroup: 1000
  containers:
    - name: checkstream
      securityContext:
        readOnlyRootFilesystem: true
        allowPrivilegeEscalation: false
        capabilities:
          drop:
            - ALL
```

### Network Isolation

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: checkstream-hipaa
spec:
  podSelector:
    matchLabels:
      app: checkstream
      compliance: hipaa
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - podSelector:
            matchLabels:
              access: checkstream-approved
  egress:
    - to:
        - podSelector:
            matchLabels:
              service: openai-proxy
      ports:
        - port: 443
```

---

## Compliance Checklist

| HIPAA Requirement | CheckStream Implementation |
|-------------------|---------------------------|
| 164.502 - PHI Use/Disclosure | Block PHI requests, redact PHI in responses |
| 164.502(b) - Minimum Necessary | Automatic PHI redaction |
| 164.312(b) - Audit Controls | Hash-chained audit trail |
| 164.312(c) - Integrity | Tamper-proof hash chain |
| 164.312(d) - Authentication | JWT-based admin access |
| 164.312(e) - Transmission Security | TLS 1.2+ encryption |
| 164.530(j) - Retention | 6-year audit retention |

---

## Next Steps

- [Financial Compliance](financial-compliance.md) - FCA/FINRA example
- [Content Moderation](content-moderation.md) - Safety example
- [Compliance Guide](../guides/compliance.md) - Full compliance documentation
