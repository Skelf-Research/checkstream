# Financial Compliance Example

A complete example of deploying CheckStream for UK FCA and US FINRA compliance.

---

## Scenario

A fintech company wants to deploy an AI assistant that:
- Provides general financial education
- Blocks specific investment advice (FCA COBS 9A)
- Adds required risk warnings (FINRA 2210)
- Maintains tamper-proof audit trail

---

## Architecture

```
┌─────────────┐     ┌──────────────────────┐     ┌─────────────┐
│   Client    │────▶│  CheckStream Proxy   │────▶│   OpenAI    │
│   App       │◀────│  (FCA/FINRA Policy)  │◀────│   GPT-4     │
└─────────────┘     └──────────────────────┘     └─────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │   Audit Trail    │
                    │  (Hash-Chained)  │
                    └──────────────────┘
```

---

## Configuration

### config.yaml

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  metrics_port: 9090

backend:
  url: "https://api.openai.com/v1"
  timeout_ms: 30000

pipeline:
  ingress:
    enabled: true
    classifiers:
      - prompt_injection
      - financial_advice_request
    threshold: 0.85

  midstream:
    enabled: true
    token_holdback: 16
    classifiers:
      - investment_recommendation
      - projection_claim
    chunk_threshold: 0.75

  egress:
    enabled: true
    audit: true
    classifiers:
      - financial_topic
      - investment_discussion

policy:
  path: "./policies/fca-finra.yaml"

telemetry:
  logging:
    level: info
    format: json
  audit:
    enabled: true
    path: "./audit"
    hash_chain: true
    retention_days: 2555  # 7 years for FINRA
```

### classifiers.yaml

```yaml
classifiers:
  # Tier A - Pattern matching
  projection_claim:
    tier: A
    type: pattern
    patterns:
      - name: percentage_return
        pattern: '\b\d+(\.\d+)?%\s+(return|growth|yield|gain)'
        score: 0.9
      - name: guarantee
        pattern: '\b(guarantee|guaranteed|certain)\s+(return|profit)'
        score: 1.0

  # Tier B - ML classifiers
  financial_advice_request:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/financial-advice-detector"
      quantization: int8
    device: auto

  investment_recommendation:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/investment-rec-detector"
      quantization: int8
    labels:
      0: general_info
      1: recommendation

  financial_topic:
    tier: B
    type: ml
    model:
      source: huggingface
      repo: "company/financial-topic-classifier"
```

---

## Policy

### policies/fca-finra.yaml

```yaml
version: "1.0"
name: "fca-finra-compliance"
description: "UK FCA and US FINRA compliant financial AI policy"

policies:
  # ============================================
  # INGRESS PHASE - Block at input
  # ============================================

  # FCA COBS 9A.2 - No specific investment advice without suitability
  - name: block_advice_request
    phase: ingress
    priority: 100
    trigger:
      all:
        - classifier: financial_advice_request
          threshold: 0.8
        - pattern: '(should|recommend|suggest)\s+(I|we)\s+(buy|sell|invest)'
          case_insensitive: true
    action: stop
    message: |
      I'm unable to provide specific investment advice or recommendations.

      For personalized financial guidance, please consult with a qualified
      financial advisor who can assess your individual circumstances.
    regulation: "FCA COBS 9A.2.1R"

  # Block specific stock/fund recommendations
  - name: block_specific_recommendations
    phase: ingress
    priority: 95
    trigger:
      pattern: '(buy|sell|invest in)\s+(stock|shares?|fund)\s+(of|in)\s+\w+'
      case_insensitive: true
    action: stop
    message: |
      I cannot recommend specific securities. This would constitute
      regulated investment advice.

      Consider consulting a licensed financial advisor.
    regulation: "FCA COBS 9A.2.1R / FINRA Rule 2111"

  # ============================================
  # MIDSTREAM PHASE - Redact during streaming
  # ============================================

  # FINRA 2210(d)(1)(B) - No guarantees
  - name: redact_guarantees
    phase: midstream
    priority: 90
    trigger:
      pattern: '\b(guarantee|guaranteed|certain|sure)\s+(return|profit|gain|growth)'
      case_insensitive: true
    action:
      - type: redact
        replacement: "[CLAIM REMOVED - cannot guarantee investment returns]"
      - type: audit
        regulation: "FINRA Rule 2210(d)(1)(B)"

  # Redact specific percentage projections
  - name: redact_projections
    phase: midstream
    priority: 85
    trigger:
      classifier: projection_claim
      threshold: 0.8
    action:
      - type: redact
        replacement: "[PROJECTION REMOVED]"
      - type: log
        level: warn

  # Redact buy/sell recommendations
  - name: redact_recommendations
    phase: midstream
    priority: 80
    trigger:
      classifier: investment_recommendation
      label: recommendation
      confidence: 0.75
    action:
      - type: redact
        replacement: "[RECOMMENDATION REMOVED - consult a financial advisor]"
      - type: audit
        regulation: "FCA COBS 9A.2.1R"

  # ============================================
  # EGRESS PHASE - Add disclaimers
  # ============================================

  # FCA PRIN 2A.4 - Enable informed decisions
  - name: risk_warning
    phase: egress
    priority: 70
    trigger:
      classifier: investment_discussion
      threshold: 0.4
    action:
      type: inject
      position: end
      content: |

        ---
        **Important Risk Information**

        - The value of investments can fall as well as rise
        - You may get back less than you originally invested
        - Past performance is not a reliable indicator of future results
        - This information is for educational purposes only and does not
          constitute financial advice
    regulation: "FCA PRIN 2A.4.1R"

  # FINRA 2210(d)(1)(D) - Performance disclosure
  - name: performance_disclaimer
    phase: egress
    priority: 65
    trigger:
      pattern: '\b(performance|returns?|grew|growth)\b'
    action:
      type: inject
      position: end
      content: |

        *Historical performance data is provided for illustrative purposes
        only and does not guarantee future results.*
    regulation: "FINRA Rule 2210(d)(1)(D)"

  # Always audit financial topics
  - name: audit_financial
    phase: egress
    priority: 50
    trigger:
      classifier: financial_topic
      threshold: 0.3
    action:
      type: audit
      include:
        - input_hash
        - output_hash
        - classifier_scores
        - timestamp
        - user_id
      retention_days: 2555

  # ============================================
  # SHADOW MODE - Testing new rules
  # ============================================

  - name: test_enhanced_detection
    mode: shadow
    trigger:
      classifier: experimental_finance_v2
      threshold: 0.6
    action: log
    # Logs but doesn't enforce
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

# General education - ALLOWED
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "What is compound interest?"
    }]
)
print(response.choices[0].message.content)
# Output: Explanation of compound interest with disclaimer
```

### Blocked Interactions

```python
# Specific advice request - BLOCKED
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{
        "role": "user",
        "content": "Should I buy Tesla stock?"
    }]
)
# Returns error:
# "I'm unable to provide specific investment advice..."
```

### Redacted Responses

```python
# If somehow a recommendation starts generating - REDACTED
# User sees:
# "Based on historical data, [RECOMMENDATION REMOVED - consult a financial advisor]"
```

---

## Audit Trail

### Sample Audit Record

```json
{
  "id": "audit-2024011510300001",
  "timestamp": "2024-01-15T10:30:00.123Z",
  "previous_hash": "sha256:abc123...",
  "hash": "sha256:def456...",
  "request_id": "req-789",
  "user_id": "user-123",
  "input_hash": "sha256:input...",
  "output_hash": "sha256:output...",
  "classifiers": [
    {"name": "financial_topic", "score": 0.72},
    {"name": "investment_discussion", "score": 0.45}
  ],
  "actions": [
    {
      "type": "inject",
      "rule": "risk_warning",
      "regulation": "FCA PRIN 2A.4.1R"
    }
  ],
  "regulations_cited": ["FCA PRIN 2A.4.1R"],
  "phase_latencies": {
    "ingress": 3.2,
    "midstream_avg": 1.8,
    "egress": 2.1
  }
}
```

### Query Audit Trail

```bash
# Get all FCA-related audit records
curl "http://localhost:8080/audit?regulation=FCA&start=2024-01-01"

# Verify chain integrity
curl "http://localhost:8080/audit/verify?start=2024-01-01&end=2024-01-31"
```

---

## Compliance Report

Generate periodic compliance reports:

```bash
curl "http://localhost:8080/admin/compliance-report" \
  -d '{
    "start_date": "2024-01-01",
    "end_date": "2024-01-31",
    "regulations": ["FCA", "FINRA"]
  }'
```

```json
{
  "period": "2024-01-01 to 2024-01-31",
  "total_requests": 125000,
  "summary": {
    "blocked": 1234,
    "redacted": 567,
    "disclaimers_added": 89012,
    "audited": 125000
  },
  "by_regulation": {
    "FCA COBS 9A.2.1R": {
      "blocks": 890,
      "redactions": 234
    },
    "FINRA 2210": {
      "blocks": 344,
      "redactions": 333,
      "disclaimers": 45000
    }
  },
  "audit_chain": {
    "status": "valid",
    "records": 125000,
    "integrity": "verified"
  }
}
```

---

## Testing

### Unit Tests

```python
import pytest
from checkstream_client import CheckStreamClient

client = CheckStreamClient("http://localhost:8080")

def test_blocks_investment_advice():
    response = client.test_policy(
        text="You should buy AAPL stock",
        phase="ingress"
    )
    assert response.decision == "block"
    assert "FCA COBS 9A" in response.regulation

def test_allows_education():
    response = client.test_policy(
        text="What is a mutual fund?",
        phase="ingress"
    )
    assert response.decision == "allow"

def test_adds_disclaimer():
    response = client.test_policy(
        text="Stocks historically return 7% annually",
        phase="egress"
    )
    assert response.decision == "inject"
    assert "past performance" in response.content.lower()
```

### Integration Tests

```bash
# Run compliance test suite
./scripts/run_compliance_tests.sh --regulation FCA
./scripts/run_compliance_tests.sh --regulation FINRA
```

---

## Next Steps

- [Healthcare Compliance](healthcare.md) - HIPAA example
- [Content Moderation](content-moderation.md) - Safety example
- [Policy Engine Guide](../guides/policy-engine.md) - Customize policies
