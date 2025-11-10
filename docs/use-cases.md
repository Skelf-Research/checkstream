# CheckStream Use Cases

This document provides detailed use cases across industries, showing how CheckStream addresses real-world streaming LLM safety and compliance challenges.

---

## Financial Services

### 1. Retail Banking: Neobank Customer Support

**Scenario**: A digital bank uses an LLM-powered chatbot for customer inquiries about accounts, fees, and products.

**Challenges**:
- Must comply with FCA Consumer Duty (clear, fair, not misleading)
- Fee structures must be explained transparently
- Overdraft offers require risk warnings
- Cannot mislead about product features

**CheckStream Solution**:

```yaml
policies:
  - name: fee_transparency_FCA
    description: Ensure fee disclosures are clear and complete
    rules:
      - trigger:
          pattern: "(fee|charge|cost|price)"
          context: product_discussion
        action: inject_disclaimer
        disclaimer: "Full fee schedule: [URL]. Contact us for details."
        regulation: "FCA PRIN 2A - Consumer Understanding"

  - name: overdraft_risk_warning
    description: Overdraft offers must include risk language
    rules:
      - trigger:
          classifier: product_recommendation
          product_type: overdraft
          confidence: 0.7
        action: inject_disclaimer
        disclaimer: "Overdrafts can be expensive. Only use if necessary."
        regulation: "FCA Consumer Duty - Products & Services Outcome"

  - name: vulnerability_detection
    description: Adapt tone when customer shows vulnerability cues
    rules:
      - trigger:
          pattern: "(can't pay|struggling|bereaved|job loss|disabled|anxious)"
        action: adapt_tone
        mode: supportive
        inject_resources: true
        resources_url: "https://bank.example.com/financial-difficulty"
        regulation: "FCA FG21/1 - Guidance for firms on the fair treatment of vulnerable customers"
```

**Results**:
- 100% of fee discussions include clear disclosures
- Overdraft conversations flagged for human review if vulnerability detected
- Audit trail for regulators showing compliance per conversation

---

### 2. Investment Platform: Robo-Advisor Chat

**Scenario**: An investment app offers an AI assistant for portfolio questions and market commentary.

**Challenges**:
- Must distinguish **advice** (regulated) from **information** (not regulated)
- Personalized recommendations require suitability assessment (FCA COBS 9A)
- Cannot guarantee returns or minimize risks
- High-risk products need extra warnings

**CheckStream Solution**:

```yaml
policies:
  - name: advice_vs_information_boundary
    description: Block regulated advice without suitability
    rules:
      - trigger:
          classifier: advice_vs_info
          threshold: 0.75
        conditions:
          - user.suitability_assessed == false
        action: stop_with_message
        message: "I can provide information, but personalized recommendations require assessing your circumstances. Speak to an advisor?"
        regulation: "FCA COBS 9A.2.1R - Suitability assessment"

  - name: guaranteed_returns_ban
    description: Block misleading language about returns
    rules:
      - trigger:
          pattern: "(guaranteed|no risk|can't lose|definite profit)"
          context: investment_discussion
        action: redact
        replacement: "[STATEMENT REMOVED - investments carry risk]"
        regulation: "FCA PRIN 2A.2.1 - Act in good faith towards retail customers"

  - name: high_risk_product_warning
    description: Derivatives and volatile assets need clear warnings
    rules:
      - trigger:
          classifier: product_mention
          product_category: [derivatives, crypto, leveraged_etf]
        action: inject_disclaimer
        disclaimer: "This is a high-risk product. You could lose more than your initial investment."
        regulation: "FCA Handbook - Product Intervention and Product Governance Sourcebook (PROD)"
```

**Results**:
- Clear boundary between information and advice maintained
- Zero incidents of misleading performance claims
- Regulators can review hash-chained audit trail of every conversation

---

### 3. Lending Platform: Credit Pre-Qualification Chat

**Scenario**: A lending fintech uses LLM to help users understand loan products and eligibility.

**Challenges**:
- Must not misrepresent affordability
- APR and total cost must be clear
- Vulnerability indicators require special care
- Cannot discriminate based on protected characteristics

**CheckStream Solution**:

```yaml
policies:
  - name: apr_disclosure_required
    description: Always show APR and total cost for credit products
    rules:
      - trigger:
          classifier: credit_product_mention
          confidence: 0.7
        action: inject_disclaimer
        disclaimer: "Representative APR: X%. Total repayable: £Y over Z months. Example only."
        regulation: "Consumer Credit Act 1974 - Truth in Lending"

  - name: affordability_check_required
    description: Cannot suggest borrowing without affordability assessment
    rules:
      - trigger:
          classifier: lending_recommendation
          threshold: 0.8
        conditions:
          - user.affordability_assessed == false
        action: stop_with_message
        message: "Let's check affordability first. May I ask a few questions?"
        regulation: "FCA CONC 5.2A - Creditworthiness assessment"

  - name: vulnerability_hardship_support
    description: Offer support resources when financial difficulty mentioned
    rules:
      - trigger:
          pattern: "(can't pay|behind on payments|bankruptcy|debt)"
        action: adapt_tone
        mode: supportive
        inject_resources: true
        resources:
          - "Free debt advice: https://stepchange.org"
          - "Our hardship team: support@lender.example.com"
        regulation: "FCA FG21/1 - Vulnerable customers"
```

**Results**:
- 100% APR disclosure on credit discussions
- Vulnerability detection triggers human escalation
- Protected characteristic mentions flagged for bias review

---

### 4. Insurtech: Policy Explanation Chatbot

**Scenario**: Insurance provider uses LLM to explain policy terms and handle claims inquiries.

**Challenges**:
- Cannot misrepresent coverage or exclusions
- Claims assistance must be fair and accessible
- Medical underwriting questions require HIPAA-like protections
- Renewal pricing must be transparent

**CheckStream Solution**:

```yaml
policies:
  - name: coverage_accuracy
    description: Block incorrect coverage statements
    rules:
      - trigger:
          classifier: policy_coverage_statement
          confidence: 0.6
        action: verify_with_policy_db
        fallback_action: inject_disclaimer
        disclaimer: "This is general information. Check your policy documents for exact coverage."
        regulation: "Insurance Conduct of Business Sourcebook (ICOBS)"

  - name: claims_support_accessibility
    description: Claims process must be clearly explained
    rules:
      - trigger:
          intent: claims_inquiry
        action: inject_guidance
        guidance: |
          To file a claim:
          1. Contact us at [number]
          2. Provide [documents]
          3. We'll respond within [timeframe]
        regulation: "FCA ICOBS - Claims handling standards"

  - name: pii_medical_protection
    description: Prevent logging of medical information
    rules:
      - trigger:
          classifier: phi_detector  # Protected Health Information
          confidence: 0.8
        action: redact_from_logs
        log_placeholder: "[MEDICAL_INFO_REDACTED]"
        regulation: "GDPR Article 9 - Special category data"
```

**Results**:
- Zero claims denied due to misinformation from chatbot
- Medical data never logged in telemetry
- Audit shows fair treatment across all customer segments

---

## Healthcare

### 5. Patient Support: Symptom Checker & Triage

**Scenario**: Healthcare system uses LLM for initial patient triage and symptom guidance.

**Challenges**:
- Cannot diagnose or prescribe (unauthorized practice)
- Must include medical disclaimers
- PHI (Protected Health Information) must be protected
- Emergency situations require immediate escalation

**CheckStream Solution**:

```yaml
policies:
  - name: no_diagnosis_or_prescription
    description: Block medical advice that constitutes practice of medicine
    rules:
      - trigger:
          classifier: medical_diagnosis_language
          threshold: 0.7
          patterns: ["you have", "I diagnose", "take this medication"]
        action: stop_with_message
        message: "I can't diagnose conditions. Please consult a licensed healthcare provider."
        regulation: "State Medical Practice Acts - Unauthorized practice"

  - name: emergency_escalation
    description: Immediate escalation for life-threatening symptoms
    rules:
      - trigger:
          keywords: [chest pain, difficulty breathing, stroke, severe bleeding]
        action: immediate_escalation
        message: "This may be an emergency. Please call 911 or go to the nearest ER immediately."
        priority: critical

  - name: medical_disclaimer_injection
    description: All health information includes disclaimer
    rules:
      - trigger:
          context: health_information
        action: inject_disclaimer
        position: end
        disclaimer: "This information is not medical advice. Consult a healthcare provider for diagnosis and treatment."
        regulation: "21 CFR Part 801 - Medical Device Labeling"

  - name: phi_protection
    description: Prevent PHI leakage in responses
    rules:
      - trigger:
          classifier: phi_detector
          types: [name, dob, mrn, ssn]
        action: redact
        replacement: "[PROTECTED INFORMATION]"
        regulation: "HIPAA Privacy Rule 45 CFR 164.502"
```

**Results**:
- Zero unauthorized medical advice incidents
- 100% emergency symptom escalation rate
- PHI never appears in LLM responses or logs

---

## Legal Services

### 6. Legal Tech: Contract Review Assistant

**Scenario**: SaaS platform helps small businesses review contracts with LLM assistance.

**Challenges**:
- Cannot provide legal advice (unauthorized practice of law)
- Must include disclaimers about attorney consultation
- Cannot miss critical risk clauses
- Different rules per jurisdiction

**CheckStream Solution**:

```yaml
policies:
  - name: upl_prevention
    description: Prevent unauthorized practice of law
    rules:
      - trigger:
          classifier: legal_advice_detector
          threshold: 0.75
          patterns: ["you should sign", "I recommend", "this contract is"]
        action: rewrite
        replacement: "Consider consulting an attorney about [topic]. I can only provide general information."
        regulation: "State Bar Rules - Unauthorized Practice of Law"

  - name: jurisdiction_awareness
    description: Flag jurisdiction-specific issues
    rules:
      - trigger:
          classifier: jurisdiction_specific_clause
          confidence: 0.7
        action: inject_disclaimer
        disclaimer: "Laws vary by state/country. Consult a local attorney for advice specific to [jurisdiction]."

  - name: critical_clause_flagging
    description: Ensure user sees high-risk provisions
    rules:
      - trigger:
          clause_type: [indemnification, non-compete, arbitration, ip_assignment]
        action: inject_notice
        notice: "⚠️ This is a significant legal provision. Consider professional review."
        regulation: "ABA Model Rules - Competence and Diligence"
```

**Results**:
- Clear boundary between information and legal advice
- Critical clauses highlighted for human attorney review
- Jurisdiction disclaimers prevent cross-border UPL risks

---

## Security Use Cases

### 7. Enterprise Copilot: Code Generation with Secret Protection

**Scenario**: Software company deploys GitHub Copilot-like assistant for developers.

**Challenges**:
- Cannot leak API keys, credentials, or internal IPs
- Prevent injection attacks via code comments
- Block generation of vulnerable code patterns
- Maintain code quality and security standards

**CheckStream Solution**:

```yaml
policies:
  - name: secret_detection
    description: Prevent API keys and credentials in generated code
    rules:
      - trigger:
          pattern: |
            (api[_-]?key|secret|password|token)\\s*=\\s*['\"][^'\"]{8,}['\"]
        action: redact
        replacement: "# [CREDENTIAL PLACEHOLDER - Use environment variables]"

  - name: injection_in_comments
    description: Detect prompt injection via code comments
    rules:
      - trigger:
          classifier: prompt_injection
          context: code_comment
        action: block
        message: "Potential injection detected. Please rephrase your request."

  - name: vulnerable_patterns
    description: Block common security anti-patterns
    rules:
      - trigger:
          patterns:
            - "eval\\("
            - "exec\\("
            - "os\\.system\\("
            - "innerHTML\\s*="
        action: inject_warning
        warning: |
          ⚠️ Security notice: This pattern may introduce vulnerabilities.
          Consider safer alternatives:
          - Use ast.literal_eval() instead of eval()
          - Use subprocess.run() instead of os.system()
          - Use textContent instead of innerHTML
```

**Results**:
- Zero credential leaks in generated code
- Vulnerable patterns flagged with remediation guidance
- Injection attempts blocked at ingress

---

### 8. Customer Service: Public Chatbot with PII Protection

**Scenario**: E-commerce site uses LLM chatbot for order support, returns, and FAQs.

**Challenges**:
- Customers may inadvertently share credit card numbers, SSNs
- Chatbot must not echo back or log PII
- Prompt injection via fake "customer support" requests
- Must handle angry/abusive customers gracefully

**CheckStream Solution**:

```yaml
policies:
  - name: pii_input_protection
    description: Prevent customers from sharing PII in chat
    rules:
      - trigger:
          classifier: pii_detector
          types: [credit_card, ssn, passport]
        action: redact_and_warn
        replacement: "[INFORMATION REMOVED FOR YOUR SECURITY]"
        warning: "Please don't share sensitive information like credit card numbers in chat. Our team never asks for this."
        regulation: "PCI DSS - Cardholder data protection"

  - name: prompt_injection_defense
    description: Block attempts to manipulate bot via fake instructions
    rules:
      - trigger:
          classifier: prompt_injection
          confidence: 0.8
        action: block
        message: "I can only help with order and product questions. How can I assist you today?"

  - name: abuse_handling
    description: Gracefully handle toxic customer messages
    rules:
      - trigger:
          classifier: toxicity
          threshold: 0.9
        action: stop_with_message
        message: "I'm here to help, but I need to end this conversation due to inappropriate language. You can reach our support team at support@example.com."
```

**Results**:
- PII never logged or echoed in responses
- Prompt injection attempts blocked
- Abusive conversations terminated gracefully, preserving customer relationship

---

## Government & Defense

### 9. Classified Systems: Document Q&A with Classification Control

**Scenario**: Defense contractor builds LLM system for classified document retrieval.

**Challenges**:
- Cannot leak higher-classification content to lower-cleared users
- Must maintain classification markings in responses
- Prevent cross-domain information flow
- Audit every access for security review

**CheckStream Solution**:

```yaml
policies:
  - name: classification_boundary_enforcement
    description: Block higher-classification content from appearing in lower-classification responses
    rules:
      - trigger:
          classifier: classification_detector
          detected_level: [TS, S]  # Top Secret, Secret
        conditions:
          - user.clearance_level in [C, U]  # Confidential, Unclassified
        action: stop_with_message
        message: "This query returned results above your clearance level. Contact your security officer."
        audit_priority: critical

  - name: classification_marking_preservation
    description: Maintain proper classification markings
    rules:
      - trigger:
          context: classified_response
        action: inject_header_footer
        header: "CLASSIFICATION: {{response.classification}}"
        footer: "Derived from: {{source.classification_guide}}"
        regulation: "EO 13526 - Classified National Security Information"

  - name: cross_domain_prevention
    description: Never mix sources from different classification domains
    rules:
      - trigger:
          condition: sources_from_multiple_domains
        action: block
        message: "Cross-domain query detected. Please refine to single domain."
```

**Results**:
- Zero classification breaches
- Every response includes proper markings
- Audit trail for security reviews and insider threat detection

---

## Education

### 10. EdTech: Homework Help with Academic Integrity

**Scenario**: Learning platform offers AI tutor for student questions.

**Challenges**:
- Cannot directly solve homework (violates academic integrity)
- Must encourage learning, not provide answers
- Detect and block cheating attempts (exam questions pasted in)
- Age-appropriate content filtering

**CheckStream Solution**:

```yaml
policies:
  - name: no_direct_answers
    description: Provide hints and explanations, not solutions
    rules:
      - trigger:
          classifier: homework_question_detector
          confidence: 0.7
        action: rewrite_response_mode
        mode: socratic
        guidance: "Guide with questions and hints, not full solutions"

  - name: exam_question_detection
    description: Block suspected exam questions
    rules:
      - trigger:
          patterns: ["exam question", "test question", "quiz"]
          classifier: assessment_language
        action: stop_with_message
        message: "This looks like an exam question. I can't help during assessments. Good luck!"

  - name: age_appropriate_content
    description: Filter content based on student age
    rules:
      - trigger:
          classifier: age_inappropriate
          user_age: < 13
        action: redact
        replacement: "[Content not available for your age group]"
        regulation: "COPPA - Children's Online Privacy Protection Act"
```

**Results**:
- Students engage with learning process, not just copying answers
- Exam integrity maintained
- Age-appropriate content filtering prevents COPPA violations

---

## Cross-Industry: Prompt Injection Defense

### 11. Any Streaming LLM: Real-Time Injection Blocking

**Scenario**: Any application using streaming LLM APIs.

**Attack Vectors**:
- Direct injection: "Ignore previous instructions and..."
- Indirect injection: Malicious content in retrieved documents
- Multi-turn attacks: Gradual boundary erosion
- Tool-use injection: Malicious arguments in function calls

**CheckStream Solution**:

```yaml
policies:
  - name: direct_injection_detection
    description: Block obvious injection attempts
    rules:
      - trigger:
          classifier: prompt_injection
          confidence: 0.8
          patterns:
            - "ignore (previous|all) (instructions|rules)"
            - "system prompt"
            - "you are now"
            - "DAN mode"
        action: block
        message: "Your request could not be processed. Please rephrase."

  - name: indirect_injection_from_context
    description: Scan retrieved documents for injection payloads
    rules:
      - trigger:
          classifier: prompt_injection
          context: retrieved_documents
          confidence: 0.7
        action: sanitize_context
        method: remove_section
        log_event:
          severity: high
          message: "Indirect injection detected in retrieved content"

  - name: tool_use_injection
    description: Validate function call arguments
    rules:
      - trigger:
          tool_call: true
          classifier: injection_in_args
        action: block_tool_call
        message: "Function call blocked due to suspicious arguments"
        escalate: security_team
```

**Results**:
- 99.7% injection blocking rate
- Indirect attacks caught before reaching LLM
- Tool-use exploits prevented

---

## Summary Table

| Industry | Use Case | Primary Risk | CheckStream Solution | Regulatory Framework |
|----------|----------|--------------|----------------------|----------------------|
| **Banking** | Neobank support | Misleading fees, vulnerability | Fee disclosure + tone adaptation | FCA Consumer Duty |
| **Investing** | Robo-advisor | Advice vs. info boundary | Classifier + suitability gate | FCA COBS 9A |
| **Lending** | Credit pre-qual | Affordability, APR clarity | Mandatory disclosures | Consumer Credit Act |
| **Insurance** | Policy chatbot | Coverage misrepresentation | Verify with policy DB | ICOBS |
| **Healthcare** | Symptom triage | Unauthorized practice, PHI | Block diagnosis + redact PHI | HIPAA, State Medical Acts |
| **Legal** | Contract review | UPL, jurisdiction issues | Legal advice detector + disclaimers | State Bar Rules |
| **Tech** | Code copilot | Secret leakage, vulnerabilities | Secret detector + pattern blocking | OWASP, SOC 2 |
| **E-commerce** | Customer service | PII logging, injection | PII redaction + injection defense | PCI DSS, GDPR |
| **Government** | Classified Q&A | Classification spillage | Classification enforcement | EO 13526 |
| **Education** | Homework help | Academic integrity, COPPA | No direct answers + age filtering | COPPA, Academic policies |
| **All** | Streaming LLMs | Prompt injection | Multi-layer detection | Security best practices |

---

## Next Steps

- **Explore compliance details**: [Regulatory Compliance](regulatory-compliance.md)
- **Understand deployment**: [Deployment Modes](deployment-modes.md)
- **Write policies**: [Policy Engine](policy-engine.md)
- **Review architecture**: [Architecture](architecture.md)
