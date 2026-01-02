# Regulatory Compliance Guide

Configure CheckStream for FCA, FINRA, GDPR, HIPAA, and other regulatory requirements.

---

## Overview

CheckStream includes pre-built policy packs for common regulations:

| Regulation | Region | Domain |
|------------|--------|--------|
| FCA | UK | Financial services |
| FINRA | US | Broker-dealers |
| GDPR | EU | Data protection |
| HIPAA | US | Healthcare |
| MiFID II | EU | Investment services |

---

## FCA Compliance (UK Financial)

### Consumer Duty Requirements

```yaml
version: "1.0"
name: "fca-consumer-duty"

policies:
  # PRIN 2A.2 - Act in good faith
  - name: honest_communication
    trigger:
      classifier: misleading_content
      threshold: 0.7
    action: stop
    message: "Content may be misleading. Please rephrase."
    regulation: "FCA PRIN 2A.2.1R"

  # PRIN 2A.3 - Avoid foreseeable harm
  - name: prevent_harm
    trigger:
      classifier: harmful_advice
      threshold: 0.8
    action: stop
    regulation: "FCA PRIN 2A.3.1R"

  # PRIN 2A.4 - Enable informed decisions
  - name: risk_disclosure
    phase: egress
    trigger:
      classifier: investment_discussion
      threshold: 0.4
    action: inject
    position: end
    content: |

      ---
      **Risk Warning**: The value of investments can fall as well as rise.
      You may not get back the amount originally invested.
    regulation: "FCA PRIN 2A.4.1R"
```

### COBS Requirements

```yaml
policies:
  # COBS 4 - Financial promotions
  - name: fair_promotion
    trigger:
      all:
        - classifier: financial_promotion
          threshold: 0.6
        - not:
            classifier: balanced_content
            threshold: 0.7
    action: stop
    message: "Financial promotions must be fair, clear and not misleading."
    regulation: "FCA COBS 4.2.1R"

  # COBS 9A - Suitability
  - name: block_unsuitable_advice
    trigger:
      classifier: investment_advice
      threshold: 0.75
    action: stop
    message: "Personalized investment advice requires a suitability assessment."
    regulation: "FCA COBS 9A.2.1R"

  # COBS 14 - Client assets
  - name: asset_warning
    trigger:
      pattern: '(transfer|move|send)\s+(funds|money|assets)'
    action: inject
    position: end
    content: "\n\n*Please verify all transfer details carefully.*"
    regulation: "FCA COBS 14.3"
```

---

## FINRA Compliance (US Broker-Dealers)

### Rule 2210 - Communications

```yaml
version: "1.0"
name: "finra-2210"

policies:
  # Fair and balanced
  - name: balanced_presentation
    trigger:
      all:
        - classifier: investment_benefits
          threshold: 0.6
        - not:
            classifier: risk_disclosure
            threshold: 0.5
    action: inject
    position: end
    content: |

      *Investment involves risk including loss of principal.*
    regulation: "FINRA Rule 2210(d)(1)"

  # No guarantees
  - name: no_guarantees
    trigger:
      pattern: '\b(guarantee|guaranteed|certain)\s+(return|profit|gain)'
    action: stop
    message: "Cannot guarantee investment returns."
    regulation: "FINRA Rule 2210(d)(1)(B)"

  # Performance claims
  - name: performance_disclosure
    trigger:
      pattern: '\d+%\s+(return|growth|performance)'
    action: inject
    position: inline
    content: " (past performance does not guarantee future results)"
    regulation: "FINRA Rule 2210(d)(1)(D)"
```

### Rule 2111 - Suitability

```yaml
policies:
  - name: suitability_required
    trigger:
      all:
        - classifier: recommendation
          threshold: 0.7
        - classifier: investment_product
          threshold: 0.6
    action: stop
    message: "Recommendations require a suitability determination."
    regulation: "FINRA Rule 2111"
```

---

## GDPR Compliance (EU Data Protection)

### Personal Data Protection

```yaml
version: "1.0"
name: "gdpr-compliance"

policies:
  # Article 9 - Special categories
  - name: block_sensitive_data
    trigger:
      any:
        - classifier: health_data
          threshold: 0.8
        - classifier: biometric_data
          threshold: 0.8
        - classifier: political_opinion
          threshold: 0.8
    action: stop
    message: "Cannot process special category personal data without explicit consent."
    regulation: "GDPR Article 9(1)"

  # Article 5 - Data minimization
  - name: redact_excessive_pii
    phase: midstream
    trigger:
      classifier: pii_detector
      threshold: 0.9
    action: redact
    replacement: "[DATA MINIMIZED]"
    regulation: "GDPR Article 5(1)(c)"

  # Article 13 - Right to information
  - name: processing_notice
    phase: egress
    trigger:
      classifier: personal_data_processing
      threshold: 0.5
    action: inject
    position: end
    content: |

      ---
      *Your data is processed in accordance with our Privacy Policy.*
    regulation: "GDPR Article 13"
```

### Right to Be Forgotten

```yaml
policies:
  - name: erasure_compliance
    trigger:
      pattern: 'delete|erase|remove|forget'
    action: audit
    metadata:
      potential_erasure_request: true
    regulation: "GDPR Article 17"
```

---

## HIPAA Compliance (US Healthcare)

### Privacy Rule

```yaml
version: "1.0"
name: "hipaa-privacy"

policies:
  # PHI protection
  - name: block_phi_requests
    phase: ingress
    trigger:
      any:
        - pattern: '(patient|medical|health)\s+record'
        - pattern: '\b(diagnosis|treatment|prescription)\b'
        - classifier: phi_request
          threshold: 0.8
    action: stop
    message: "Cannot access or disclose protected health information."
    regulation: "HIPAA 45 CFR 164.502"

  # Minimum necessary
  - name: redact_phi
    phase: midstream
    trigger:
      any:
        - classifier: phi_detector
          threshold: 0.85
        - pattern: '\b(MRN|DOB|SSN)[\s:]+\S+'
        - pattern: '\b\d{3}-\d{2}-\d{4}\b'  # SSN
    action: redact
    replacement: "[PHI REDACTED]"
    regulation: "HIPAA 45 CFR 164.502(b)"

  # No medical advice
  - name: medical_disclaimer
    phase: egress
    trigger:
      classifier: medical_advice
      threshold: 0.4
    action: inject
    position: end
    content: |

      ---
      *This information is for educational purposes only and is not medical advice.
      Please consult a healthcare provider for medical decisions.*
    regulation: "HIPAA Disclaimer"
```

### Security Rule

```yaml
policies:
  # Audit logging
  - name: audit_phi_access
    trigger:
      classifier: health_topic
      threshold: 0.3
    action: audit
    include:
      - input_hash
      - output_hash
      - timestamp
      - user_id
    regulation: "HIPAA 45 CFR 164.312(b)"
```

---

## Loading Compliance Packs

### Single Regulation

```yaml
policy:
  path: "./policies/fca-consumer-duty.yaml"
```

### Multiple Regulations

```yaml
policy:
  paths:
    - "./policies/fca-consumer-duty.yaml"
    - "./policies/gdpr-compliance.yaml"
  merge_strategy: combine     # combine, first_match, priority
```

### Per-Tenant Compliance

```yaml
tenants:
  configs:
    uk-finance:
      policy_path: "./policies/fca-consumer-duty.yaml"

    us-broker:
      policy_path: "./policies/finra-2210.yaml"

    healthcare:
      policy_path: "./policies/hipaa-privacy.yaml"
```

---

## Audit Trail for Compliance

### Hash-Chained Audit Log

```yaml
telemetry:
  audit:
    enabled: true
    hash_chain: true          # Tamper-proof chain
    path: "./audit"
    rotation: "daily"
    retention_days: 2555      # 7 years for FINRA
    include:
      - request_id
      - timestamp
      - tenant
      - input_hash
      - output_hash
      - classifiers_triggered
      - actions_taken
      - regulations_applied
```

### Audit Record Example

```json
{
  "id": "audit-123456",
  "timestamp": "2024-01-15T10:30:00Z",
  "previous_hash": "abc123...",
  "hash": "def456...",
  "request_id": "req-789",
  "tenant": "uk-finance",
  "input_hash": "sha256:...",
  "output_hash": "sha256:...",
  "classifiers": [
    {"name": "financial_advice", "score": 0.82}
  ],
  "actions": [
    {"type": "stop", "rule": "block_investment_advice"}
  ],
  "regulations": ["FCA COBS 9A.2.1R"]
}
```

---

## Compliance Reporting

### Generate Report

```bash
curl http://localhost:8080/admin/compliance-report \
  -d '{
    "start_date": "2024-01-01",
    "end_date": "2024-01-31",
    "regulation": "FCA"
  }'
```

### Report Output

```json
{
  "period": "2024-01-01 to 2024-01-31",
  "regulation": "FCA",
  "summary": {
    "total_requests": 125000,
    "blocked_requests": 1234,
    "redacted_responses": 567,
    "disclaimers_added": 8901
  },
  "by_rule": [
    {
      "rule": "block_investment_advice",
      "regulation": "FCA COBS 9A.2.1R",
      "triggers": 890,
      "action": "stop"
    }
  ]
}
```

---

## Best Practices

1. **Layer defenses** - Multiple rules for critical regulations
2. **Use shadow mode** - Test policies before enforcement
3. **Maintain audit trails** - Required for most regulations
4. **Regular review** - Update policies as regulations change
5. **Document everything** - Include regulation references
6. **Train classifiers** - Fine-tune for your domain
7. **Test edge cases** - Ensure coverage of boundary conditions

---

## Next Steps

- [Policy Engine](policy-engine.md) - Write custom policies
- [Audit & Telemetry](../reference/metrics.md) - Configure audit logging
- [Examples](../examples/financial-compliance.md) - Real-world implementations
