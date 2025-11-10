# Regulatory Compliance with CheckStream

CheckStream provides built-in compliance frameworks for regulated industries, with policy packs tailored to specific jurisdictions and regulatory requirements.

---

## UK Financial Conduct Authority (FCA) Consumer Duty

### Overview

The Consumer Duty (effective July 2023) requires firms to deliver **good outcomes** for retail customers across four areas:

1. **Products & Services**: Designed for target market needs
2. **Price & Value**: Fair value assessment
3. **Consumer Understanding**: Clear, fair, not misleading communications
4. **Consumer Support**: Accessible, effective support throughout customer journey

CheckStream enforces Consumer Duty at the **point of communication** — ensuring LLM responses meet these standards in real time.

### Cross-Cutting Rules

#### PRIN 2A.2.1: Act in Good Faith

Firms must act in good faith toward retail customers.

**CheckStream Implementation**:
```yaml
policies:
  - name: good_faith_communications
    regulation: "PRIN 2A.2.1"
    rules:
      - trigger:
          classifier: misleading_language
          threshold: 0.75
        action: redact
        replacement: "[Statement removed for clarity]"

      - trigger:
          pattern: "(guaranteed|zero risk|can't lose|definite)"
          context: financial_product
        action: stop_with_message
        message: "All investments carry risk. Let me provide balanced information."
```

#### PRIN 2A.2.2: Avoid Foreseeable Harm

Prevent, monitor, and mitigate foreseeable harm.

**CheckStream Implementation**:
```yaml
policies:
  - name: foreseeable_harm_prevention
    regulation: "PRIN 2A.2.2"
    rules:
      - trigger:
          classifier: unsuitable_product_mention
          user_profile: vulnerable_customer
        action: stop
        message: "This product may not be suitable. Let me connect you with a specialist advisor."

      - trigger:
          pattern: "(borrow more|increase limit|upgrade to premium)"
          user_signals: [debt_stress, payment_difficulties]
        action: block
        audit_reason: "Prevented potential debt harm per PRIN 2A.2.2"
```

#### PRIN 2A.2.3: Enable and Support Retail Customers

Make it easy for customers to pursue their financial objectives.

**CheckStream Implementation**:
```yaml
policies:
  - name: customer_enablement
    regulation: "PRIN 2A.2.3"
    rules:
      - trigger:
          intent: product_switch_inquiry
        action: inject_guidance
        guidance: |
          You can switch by:
          1. [Clear steps]
          2. No penalties apply
          3. Takes approximately [timeframe]

      - trigger:
          intent: complaint_initiation
        action: immediate_escalation
        escalation_path: complaints_team
        acknowledgment: "I've logged your concern. You'll hear from us within 3 business days. Reference: {ticket_id}"
```

### Outcome 1: Products & Services

**Requirement**: Products designed for target market, distributed appropriately.

**CheckStream Classifiers**:

| Classifier | Purpose | Trigger Example |
|------------|---------|-----------------|
| `target_market_mismatch` | Detect when product doesn't suit customer profile | Offering complex derivatives to inexperienced investor |
| `product_feature_accuracy` | Ensure product descriptions are correct | Misrepresenting interest rates or terms |

**Example Policy**:
```yaml
policies:
  - name: target_market_alignment
    regulation: "Consumer Duty - Products & Services Outcome"
    rules:
      - trigger:
          classifier: target_market_mismatch
          confidence: 0.8
        conditions:
          - user.investment_experience in [none, low]
          - product.complexity == high
        action: stop_with_alternative
        message: "This product is complex. Here are alternatives designed for your experience level: [list]"

      - trigger:
          classifier: product_feature_accuracy
          accuracy_score: < 0.9
        action: verify_with_product_db
        fallback_action: inject_disclaimer
        disclaimer: "For exact terms, see the product documentation at [URL]"
```

### Outcome 2: Price & Value

**Requirement**: Fair value assessment; prices justified by benefits.

**CheckStream Classifiers**:

| Classifier | Purpose | Trigger Example |
|------------|---------|-----------------|
| `price_disclosure_completeness` | Ensure all costs mentioned | Hidden fees, compound interest calculations |
| `value_proposition_balance` | Benefits and costs both mentioned | Only highlighting returns, not fees |

**Example Policy**:
```yaml
policies:
  - name: price_transparency_FCA
    regulation: "Consumer Duty - Price & Value Outcome"
    rules:
      - trigger:
          context: product_discussion
          classifier: price_mention_required
        conditions:
          - response.contains_price_info == false
        action: inject_disclaimer
        disclaimer: "Annual fee: £X. See full pricing at [URL]."

      - trigger:
          pattern: "(cheap|low cost|best value)"
          context: product_comparison
        action: inject_balance
        balanced_text: "While this option has lower fees, consider all features and your needs. Compare: [link]"
```

### Outcome 3: Consumer Understanding

**Requirement**: Communications must be clear, fair, and not misleading.

**CheckStream Classifiers**:

| Classifier | Purpose | Trigger Example |
|------------|---------|-----------------|
| `promotional_balance` | Promotions must not emphasize benefits over risks | "High returns!" without risk warning |
| `clarity_score` | Detect jargon, complex sentences | Use of technical terms without explanation |
| `misleading_language` | Identify false or unbalanced claims | "Guaranteed growth" for investments |

**Example Policy**:
```yaml
policies:
  - name: consumer_understanding_FCA
    regulation: "Consumer Duty - Consumer Understanding Outcome"
    rules:
      - trigger:
          classifier: promotional_balance
          threshold: 0.7
        action: inject_disclaimer
        disclaimer: "Capital at risk. Past performance is not indicative of future results."

      - trigger:
          classifier: clarity_score
          score: < 0.6  # Too complex
        action: rewrite
        mode: simplify
        target_reading_level: 8th_grade

      - trigger:
          classifier: misleading_language
          confidence: 0.8
        action: redact
        audit_severity: high
```

### Outcome 4: Consumer Support

**Requirement**: Effective, accessible, timely support.

**CheckStream Classifiers**:

| Classifier | Purpose | Trigger Example |
|------------|---------|-----------------|
| `vulnerability_detector` | Identify customers needing extra support | Financial difficulty, bereavement, health issues |
| `support_accessibility` | Ensure instructions are clear | Complex process described poorly |

**Example Policy**:
```yaml
policies:
  - name: vulnerability_support_FCA
    regulation: "FCA FG21/1 - Vulnerable Customers"
    rules:
      - trigger:
          classifier: vulnerability_detector
          cues: [debt_stress, bereavement, health_issue, age_related]
        action: adapt_tone
        mode: supportive_empathetic
        inject_resources: true
        resources:
          - "Free debt advice: https://moneyhelper.org.uk"
          - "Our support team: {phone_number}"
        escalate_to: specialist_support_team

      - trigger:
          intent: complaint
        action: log_and_acknowledge
        acknowledgment: "I've logged this. Reference: {ref}. We'll respond within 3 business days per our complaints procedure."
        regulation: "DISP 1 - Treating complainants fairly"
```

### FCA-Specific Financial Promotions (FG23/1)

**Requirements**:
- Risk warnings for high-risk products
- Clear prominence of risks vs benefits
- Accurate, balanced, not misleading

**CheckStream Policy Pack**:
```yaml
policies:
  - name: financial_promotion_FCA
    regulation: "FG23/1 - Guidance for firms on the fair treatment of customers in financial promotions"
    rules:
      - trigger:
          classifier: investment_promotion
          product_type: [crypto, derivatives, high_risk]
        action: inject_prominent_warning
        warning: "⚠️ HIGH RISK: You could lose all your money. Only invest what you can afford to lose."
        position: top

      - trigger:
          classifier: promotional_balance
          benefits_mentioned: true
          risks_mentioned: false
        action: inject_risk_disclosure
        disclosure: |
          Risks:
          - Capital loss
          - Market volatility
          - Not covered by FSCS

      - trigger:
          pattern: "(get rich|easy money|financial freedom|passive income)"
        action: block
        reason: "Language violates FCA financial promotion standards"
```

---

## UK Fintech Vertical-Specific Compliance

### Neobanks & Retail Banking

| Regulation | CheckStream Enforcement |
|------------|-------------------------|
| **CCA 1974** - Consumer Credit Act | APR disclosure, affordability checks |
| **PSR 2017** - Payment Services | Fee transparency, error correction guidance |
| **FCA BCOBS** - Banking Conduct | Account switching support, overdraft warnings |

**Policy Pack**: `fca-retail-banking`

### Lending & Credit Platforms

| Regulation | CheckStream Enforcement |
|------------|-------------------------|
| **FCA CONC** - Consumer Credit | Suitability assessment, affordability verification |
| **PRIN 2A** - Consumer Duty | Vulnerability detection, balanced lending information |

**Policy Pack**: `fca-lending`

### Investment & Trading Apps

| Regulation | CheckStream Enforcement |
|------------|-------------------------|
| **FCA COBS 9A** - Suitability | Advice vs information boundary, risk profiling |
| **FCA PROD** - Product Governance | Complex product warnings, target market validation |

**Policy Pack**: `fca-investment`

### Insurtech & Embedded Finance

| Regulation | CheckStream Enforcement |
|------------|-------------------------|
| **ICOBS** - Insurance Conduct | Coverage accuracy, claims process clarity |
| **PRIN 2A** - Consumer Duty | Embedded product suitability, vulnerability support |

**Policy Pack**: `fca-insurance`

---

## US Securities & Exchange (SEC) & FINRA

### FINRA Rule 2210: Communications with the Public

**Requirements**:
- Fair and balanced content
- No exaggerated or misleading statements
- Risks prominently disclosed
- Approval for retail communications

**CheckStream Implementation**:
```yaml
policies:
  - name: finra_2210_communications
    regulation: "FINRA Rule 2210"
    rules:
      - trigger:
          classifier: performance_claims
          type: [hypothetical, backtested]
        action: inject_disclaimer
        disclaimer: "Hypothetical performance is not indicative of actual results. No guarantee of similar future performance."

      - trigger:
          pattern: "(guaranteed|no risk|safe|secure investment)"
        action: block
        reason: "Violates FINRA 2210 - exaggerated/misleading statements"

      - trigger:
          context: retail_communication
          approval_status: pending
        action: flag_for_review
        review_queue: compliance_team
```

### FINRA Rule 2111: Suitability

**Requirements**:
- Reasonable basis for recommendation
- Customer-specific suitability assessment
- Quantitative suitability (excessive trading)

**CheckStream Implementation**:
```yaml
policies:
  - name: finra_suitability_2111
    regulation: "FINRA Rule 2111"
    rules:
      - trigger:
          classifier: investment_recommendation
          confidence: 0.75
        conditions:
          - user.suitability_profile_complete == false
        action: stop_with_message
        message: "I need to understand your financial situation first. May I ask a few questions?"

      - trigger:
          context: recommendation
          product_risk_level: > user.risk_tolerance
        action: block_recommendation
        message: "Based on your profile, this product may not be suitable. Here are alternatives: [list]"
```

### SEC Regulation Best Interest (Reg BI)

**CheckStream Policy Pack**: `sec-reg-bi`

```yaml
policies:
  - name: reg_bi_disclosure
    regulation: "SEC Regulation Best Interest"
    rules:
      - trigger:
          context: broker_recommendation
        action: inject_disclosure
        disclosure: |
          Disclosure:
          - Capacity: Broker-Dealer
          - Material conflicts: [list]
          - See Form CRS: [URL]
```

---

## EU MiFID II

### Article 24: Information to Clients

**Requirements**:
- Appropriateness and suitability assessments
- Clear, fair, not misleading information
- Warnings for complex instruments

**CheckStream Implementation**:
```yaml
policies:
  - name: mifid_ii_article_24
    regulation: "MiFID II Article 24"
    rules:
      - trigger:
          product_type: [derivatives, structured_products]
          user_classification: retail
        action: inject_prominent_warning
        warning: "This is a complex instrument. Ensure you understand how it works and the risks involved."

      - trigger:
          service_type: investment_advice
        conditions:
          - user.suitability_assessed == false
        action: block
        message: "We must assess suitability before providing advice (MiFID II requirement)."
```

**Policy Pack**: `mifid-ii-eu`

---

## Healthcare: HIPAA (US)

### Privacy Rule (45 CFR 164.502)

**Requirements**:
- Minimum necessary PHI disclosure
- Patient consent for uses beyond treatment
- Audit of PHI access

**CheckStream Implementation**:
```yaml
policies:
  - name: hipaa_privacy_rule
    regulation: "HIPAA Privacy Rule 45 CFR 164.502"
    rules:
      - trigger:
          classifier: phi_detector
          phi_types: [name, dob, mrn, ssn, diagnosis, treatment]
        action: redact_from_response
        replacement: "[PROTECTED HEALTH INFORMATION]"

      - trigger:
          classifier: phi_detector
          context: logging
        action: redact_from_logs
        log_placeholder: "[PHI_REDACTED]"
        audit_event: phi_access_logged

      - trigger:
          context: patient_information_request
        conditions:
          - requester.authorized == false
        action: block
        message: "I cannot share patient information without authorization."
```

**Policy Pack**: `hipaa-us`

---

## Data Privacy: GDPR (EU) & CCPA (California)

### GDPR Article 9: Special Category Data

**Requirements**:
- Explicit consent for processing special category data
- Higher protection standards
- Data minimization

**CheckStream Implementation**:
```yaml
policies:
  - name: gdpr_special_category_data
    regulation: "GDPR Article 9"
    rules:
      - trigger:
          classifier: special_category_detector
          types: [health, biometric, genetic, racial, political, religious]
        action: redact_and_log
        replacement: "[SENSITIVE INFORMATION REMOVED]"
        audit_event: special_category_data_detected

      - trigger:
          intent: collect_special_category_data
        conditions:
          - user.explicit_consent_given == false
        action: block
        message: "We need your explicit consent to process this information."
```

### GDPR Article 22: Automated Decision-Making

**Requirements**:
- Right not to be subject to automated decisions with legal/significant effects
- Transparency about automated processing

**CheckStream Implementation**:
```yaml
policies:
  - name: gdpr_automated_decisions
    regulation: "GDPR Article 22"
    rules:
      - trigger:
          decision_type: [loan_approval, employment, insurance_pricing]
          automated: true
        action: inject_disclosure
        disclosure: |
          This decision involves automated processing.
          You have the right to:
          - Request human review
          - Object to the decision
          - Receive an explanation
          Contact: dpo@company.example.com
```

**Policy Pack**: `gdpr-eu`

---

## Audit & Evidence Generation

### Compliance Report Generation

CheckStream automatically generates audit-ready reports:

**Monthly Consumer Duty Report**:
```json
{
  "period": "2024-01",
  "organization": "acme-bank",
  "policy_pack": "fca-consumer-duty",
  "metrics": {
    "total_interactions": 45230,
    "outcomes": {
      "products_services": {
        "target_market_mismatches_prevented": 23,
        "unsuitable_products_blocked": 12
      },
      "price_value": {
        "price_disclosures_injected": 1234,
        "fee_transparency_enforced": 100
      },
      "consumer_understanding": {
        "misleading_statements_redacted": 45,
        "risk_warnings_added": 567,
        "complexity_simplifications": 89
      },
      "consumer_support": {
        "vulnerability_detections": 78,
        "support_escalations": 34,
        "complaint_acknowledgments": 12
      }
    },
    "regulatory_breaches_prevented": {
      "PRIN_2A_2_1": 23,  // Good faith
      "PRIN_2A_2_2": 12,  // Foreseeable harm
      "COBS_9A": 8        // Suitability
    }
  },
  "evidence_samples": [
    {
      "incident_id": "INC-2024-01-0042",
      "rule_id": "unsuitable_product_FCA",
      "regulation": "Consumer Duty - Products & Services",
      "timestamp": "2024-01-15T14:23:45Z",
      "action": "blocked_recommendation",
      "rationale": "Complex derivative offered to inexperienced investor",
      "user_profile": "investment_experience: none",
      "hash_chain": "sha256:abc123...def456"
    }
  ],
  "export_date": "2024-02-01",
  "signed_by": "checkstream-audit-service",
  "signature": "sha256:..."
}
```

### Regulatory Evidence Export

```bash
# Generate evidence pack for regulator inquiry
checkstream audit export \
  --period 2024-Q1 \
  --regulation FCA_CONSUMER_DUTY \
  --format PDF \
  --include-samples 100 \
  --output fca_evidence_q1_2024.pdf
```

**Output**:
- Executive summary of enforcement actions
- Breakdown by regulation and outcome
- Sample incidents with full decision traces
- Hash-chain integrity verification
- Policy versions and change log

---

## Continuous Compliance

### Policy Version Control

Every policy change is tracked:
```yaml
policy_metadata:
  version: "2.3.1"
  effective_date: "2024-01-15"
  regulation_updates:
    - "FCA PS23/6 - Consumer Duty implementation update"
  changes:
    - "Added vulnerability cue 'anxious' to FG21/1 detector"
    - "Increased promotional balance threshold from 0.7 to 0.75"
  approved_by: "risk_committee"
  approval_date: "2024-01-10"
  git_commit: "abc123def456"
```

### Regulatory Monitoring

CheckStream tracks regulatory changes:
- FCA policy statements
- FINRA notices
- SEC rulemaking
- GDPR guidance updates

Alerts sent when policy updates recommended:
```
Alert: FCA FG24/2 published (Guidance on AI in financial services)
Recommended action: Review promotional_balance classifier thresholds
Affected policies: consumer_understanding_FCA, promotional_balance
```

---

## Compliance Policy Packs

Pre-built, ready-to-deploy:

| Pack Name | Regulations Covered | Industries |
|-----------|---------------------|------------|
| `fca-consumer-duty` | FCA PRIN 2A, COBS, CONC, FG21/1 | UK Financial Services |
| `fca-retail-banking` | CCA 1974, PSR 2017, BCOBS | UK Neobanks |
| `fca-investment` | COBS 9A, PROD, FG23/1 | UK Investment Platforms |
| `finra-broker-dealer` | Rules 2210, 2111, 3110 | US Broker-Dealers |
| `sec-reg-bi` | Regulation Best Interest | US Investment Advisors |
| `mifid-ii-eu` | MiFID II Articles 24, 25 | EU Investment Firms |
| `hipaa-us` | Privacy Rule, Security Rule | US Healthcare |
| `gdpr-eu` | Articles 9, 22, 32 | EU Data Processing |

---

## Next Steps

- **Deploy a policy pack**: [Getting Started](getting-started.md)
- **Customize policies**: [Policy Engine](policy-engine.md)
- **Review architecture**: [Architecture](architecture.md)
- **Explore use cases**: [Use Cases](use-cases.md)
