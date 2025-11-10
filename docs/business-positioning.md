# CheckStream Business Positioning

## Market Opportunity

### Problem Space

The rapid adoption of streaming LLM interfaces has created a critical gap: **safety and compliance systems designed for batch processing cannot keep pace with real-time token generation**.

**Market Size**:
- Global AI software market: $126B (2024), projected $1.3T by 2032 (42% CAGR)
- Enterprise AI governance & compliance tools: $12B (2024), growing 48% annually
- Financial services AI spend: $35B (2024), with 65% on compliance and risk

**Regulatory Drivers**:
- **EU AI Act** (2024): Mandatory runtime risk controls for high-risk AI systems
- **FCA Consumer Duty** (UK, 2023): Real-time communication compliance
- **US Executive Order 14110** (2023): Federal AI safety standards
- **SEC/FINRA** (Ongoing): Heightened scrutiny of AI in financial advice

### Buyer Landscape

#### Primary Buyers (Regulated Industries)

**Financial Services**
- **Champions**: Chief Risk Officer, Chief Compliance Officer, MLRO
- **Budget**: Risk management, regulatory compliance, operational resilience
- **Pain**: FCA Consumer Duty fines, slow manual review, reputational risk
- **Annual spend potential**: $500K - $5M per institution

**Healthcare**
- **Champions**: CISO, Chief Medical Information Officer, Privacy Officer
- **Budget**: HIPAA compliance, patient safety, data governance
- **Pain**: PHI leakage, malpractice risk, unauthorized medical advice
- **Annual spend potential**: $300K - $3M per health system

**Legal Services**
- **Champions**: General Counsel, Risk & Compliance Director
- **Budget**: Professional liability, ethics compliance
- **Pain**: Unauthorized practice of law, client confidentiality breaches
- **Annual spend potential**: $200K - $1.5M per firm

#### Secondary Buyers (General Enterprise)

**Enterprise IT & Security**
- **Champions**: CISO, VP Engineering, Platform Security Lead
- **Budget**: Application security, data protection, incident response
- **Pain**: Prompt injection, data exfiltration, compliance risk
- **Annual spend potential**: $100K - $1M per org

---

## Competitive Landscape

### Direct Competitors

| Vendor | Positioning | Strengths | Weaknesses | CheckStream Advantage |
|--------|-------------|-----------|------------|----------------------|
| **AWS Bedrock Guardrails** | Cloud-native safety filters | Deep AWS integration, managed service | Vendor lock-in, batch-oriented, no regulatory taxonomy | Model-agnostic, token-level streaming, FCA/FINRA policy packs |
| **Azure AI Content Safety** | Multi-modal moderation API | Enterprise scale, GDPR compliance | Generic harm categories only, latency >50ms | <10ms latency, regulatory-specific classifiers, in-VPC deployment |
| **Lakera Guard** | AI security firewall | Strong injection detection, dashboard | Post-hoc filtering, SaaS-only, not compliance-focused | Preventive safety (vLLM mode), cryptographic audit, compliance packs |
| **NVIDIA NeMo Guardrails** | Framework for safe LLMs | Flexible, open-source, well-documented | Requires deep integration, no managed option, latency tuning manual | Managed control plane, pre-tuned latency, regulatory templates |

### Indirect Competitors

| Category | Examples | When They're Chosen | How We Differentiate |
|----------|----------|---------------------|----------------------|
| **Offline compliance checkers** | Red Marker, Droit | Marketing content pre-publish | Real-time streaming enforcement vs batch review |
| **GRC platforms** | OneTrust, Archer, ServiceNow | Enterprise-wide governance | Streaming-specific guardrails, not generic GRC |
| **API security gateways** | Kong, Apigee, AWS API Gateway | General API management | LLM-aware (token-level), regulatory intelligence |
| **Build in-house** | Custom Python scripts | Engineering-heavy orgs | 10x faster time-to-value, maintained classifiers, compliance expertise |

---

## Value Proposition

### For Chief Risk Officers

**Message**: *"Enforce Consumer Duty and regulatory compliance at the point of conversation—before unsafe communications reach customers."*

**Value Drivers**:
- **Regulatory defense**: Automated evidence packs for FCA/FINRA audits
- **Incident prevention**: Stop unsuitable advice, misleading claims, vulnerability mishandling
- **Audit efficiency**: 90% reduction in manual communication review
- **Fines avoidance**: FCA Consumer Duty fines average £2M+; prevention ROI is immediate

**KPIs**:
- % communications reviewed (target: 100% automated)
- Regulatory breach incidents (target: zero)
- Audit preparation time (reduce from weeks to hours)
- Cost of compliance per communication

**ROI Example**:
```
Annual cost of CheckStream: £200K
Value delivered:
- Avoided FCA fine (one incident): £2M+
- Manual review hours saved: 5,000 hours × £50/hr = £250K
- Faster time-to-market for AI features: £500K revenue opportunity
Total ROI: 13x first-year return
```

---

### For CISOs & Security Leaders

**Message**: *"Real-time AI firewall: stop prompt injection, data exfiltration, and toxic outputs before they escape your infrastructure."*

**Value Drivers**:
- **Runtime protection**: Block attacks during generation, not after
- **Zero-trust AI**: Enforce least-privilege for tools, contexts, outputs
- **Threat intelligence**: Structured logs of injection attempts, jailbreaks
- **Compliance**: GDPR Article 32 (security of processing), SOC 2, ISO 27001

**KPIs**:
- % prompt injections blocked (target: >99%)
- Mean time to detection (MTTD) for LLM threats (target: <1 second)
- Data exfiltration incidents (target: zero)
- False positive rate (target: <2%)

**ROI Example**:
```
Annual cost of CheckStream: £150K
Value delivered:
- Prevented data breach (avg cost £3.5M): One incident pays for 23 years
- Reduced SOC analyst time on LLM incidents: 2,000 hours × £60/hr = £120K
- Faster security review of AI features: 6 months faster to production
Total ROI: 24x (from breach prevention alone)
```

---

### For Engineering Leaders

**Message**: *"Ship AI features with confidence. Sub-10ms guardrails that don't break your latency SLAs."*

**Value Drivers**:
- **Zero infrastructure overhead**: Drop-in proxy or sidecar, no model changes
- **Latency parity**: <10ms added per chunk; users don't notice
- **Developer experience**: Local testing, streaming visualization, hot-reload policies
- **Reduce risk escalations**: 80% fewer compliance/legal review cycles

**KPIs**:
- Time-to-first-token (TTFT): maintain <300ms
- Tokens/second: maintain baseline throughput
- Feature velocity: # AI features shipped per quarter
- Developer NPS: satisfaction with guardrail tooling

**ROI Example**:
```
Annual cost of CheckStream: £100K
Value delivered:
- 3 additional AI features shipped (£1M ARR each): £3M
- Reduced engineering rework (compliance failures): 500 hours × £80/hr = £40K
- Faster production approval: 2 months saved per feature
Total ROI: 30x
```

---

## Positioning Statements

### Elevator Pitch (30 seconds)

*"CheckStream is the real-time safety and compliance layer for streaming AI. We enforce regulatory rules—like FCA Consumer Duty or FINRA suitability—on every token as it's generated, with sub-10ms latency. Financial services, healthcare, and government use us to prove compliance, prevent data leaks, and stop harmful outputs before they reach customers."*

### Problem-Solution Statement

**Problem**: When LLMs stream responses in real-time, traditional batch moderation is too slow—unsafe or non-compliant content escapes before it can be caught. Regulators like the FCA now require proof of real-time compliance, which existing tools can't provide.

**Solution**: CheckStream is a streaming guardrail platform that inspects tokens as they're generated, enforces policy-as-code rules with regulatory citations, and produces cryptographic audit trails—all within a <10ms latency budget that preserves user experience.

### Taglines by Buyer Persona

| Persona | Tagline |
|---------|---------|
| **Risk & Compliance** | "Compliance that operates at the speed of conversation." |
| **Security (CISO)** | "Your LLMs need a firewall. This is it." |
| **Engineering** | "Trust at the speed of generation." |
| **Executive** | "Make your generative AI auditable." |

---

## Market Segmentation & GTM Strategy

### Tier 1: Financial Services (Initial Focus)

**Verticals**:
- Neobanks & retail banking
- Investment platforms & robo-advisors
- Lending & BNPL fintechs
- Insurtech & embedded finance

**Entry Point**:
- Consumer Duty compliance urgency (FCA deadlines)
- Recent regulatory scrutiny or fines
- Deploying customer-facing LLM chatbots

**Sales Cycle**: 3-6 months (compliance + procurement approvals)

**ACV**: £200K - £2M

**GTM Motions**:
- **Inbound**: SEO (Consumer Duty + AI), webinars with law firms, FCA guidance commentary
- **Outbound**: CRO/CCO targeted campaigns, compliance conference sponsorships
- **Partnerships**: Compliance consultancies (Deloitte, PwC), LLM vendors (Anthropic, OpenAI partnerships)

---

### Tier 2: Healthcare (Follow-On)

**Verticals**:
- Health systems & hospital networks
- Telemedicine platforms
- HealthTech SaaS (EHR, patient engagement)

**Entry Point**:
- HIPAA compliance for AI deployments
- PHI leakage prevention
- Patient safety around symptom triage bots

**Sales Cycle**: 6-12 months (clinical + IT + legal approvals)

**ACV**: £150K - £1.5M

---

### Tier 3: Enterprise Security (Expansion)

**Verticals**:
- SaaS companies deploying copilots
- Customer service platforms
- Developer tools (code generation)

**Entry Point**:
- Prompt injection protection
- Data exfiltration prevention
- SOC 2 / ISO 27001 requirements

**Sales Cycle**: 2-4 months (security + engineering alignment)

**ACV**: £50K - £500K

---

## Competitive Differentiation

### Key Differentiators

1. **True Token-Level Streaming**
   - **Us**: Sliding holdback buffer, per-chunk safety in <10ms
   - **Them**: Batch API calls every 100-500 tokens (50-200ms latency)

2. **Regulatory Taxonomy Out-of-the-Box**
   - **Us**: FCA Consumer Duty, FINRA 2111, MiFID II policy packs with rule citations
   - **Them**: Generic "toxicity" or "harm" labels; customers must build regulatory logic

3. **Cryptographic Audit Trail**
   - **Us**: Hash-chained evidence, immutable logs, exportable for regulators
   - **Them**: Standard logging (CloudWatch/Splunk); no tamper-proof chain

4. **Deployment Flexibility**
   - **Us**: Proxy (model-agnostic), Sidecar (vLLM deep integration), Control Plane (SaaS)
   - **Them**: Managed SaaS only (vendor lock-in) or OSS framework (DIY complexity)

5. **Latency Obsession**
   - **Us**: INT8/INT4 quantized classifiers, CPU-optimized, <10ms per chunk
   - **Them**: Cloud API calls, network RTT, 50-200ms overhead

6. **Data Sovereignty**
   - **Us**: In-VPC enforcement, optional SaaS control plane (out-of-band)
   - **Them**: All traffic routes through vendor SaaS (data residency concerns)

### Objection Handling

| Objection | Response |
|-----------|----------|
| *"We'll just use AWS Bedrock Guardrails"* | "Bedrock Guardrails are great for generic harms, but they don't understand FCA Consumer Duty or provide cryptographic audit trails. We complement cloud guardrails with regulatory intelligence and token-level precision. Many customers use both—Bedrock for baseline safety, CheckStream for compliance." |
| *"Can't we build this in-house?"* | "You could, but regulatory classifiers require continuous tuning as FCA guidance evolves. We maintain models, update policies for new regulations, and provide audit-ready reports. Our customers' in-house attempts took 12-18 months and still needed external compliance review. We deliver in 30 days." |
| *"What about latency impact?"* | "Our quantized classifiers add <10ms per chunk—imperceptible to users. We've maintained <300ms TTFT and 50+ tok/s in production with 200+ concurrent streams. Happy to run a latency benchmark in your environment." |
| *"Is this just a wrapper around OpenAI Moderation API?"* | "No. We run proprietary classifiers (toxicity, PII, regulatory) on-device with INT8 inference. We use OpenAI/Azure moderation as *optional Tier B* escalation, but our Tier A enforces <10ms on-premise. You control the data." |
| *"We're not in a regulated industry"* | "Even non-regulated companies need prompt injection defense and PII protection. Our security pack prevents data exfiltration and jailbreaks in real-time—critical for SOC 2, ISO 27001, and customer trust. Start with security, expand to compliance when needed." |

---

## Strategic Positioning Canvas

| Dimension | CheckStream Position |
|-----------|---------------------|
| **Category** | Streaming LLM Guardrail Platform (we define this category) |
| **Core Buyer** | Chief Risk Officer / Chief Compliance Officer (regulated) <br> CISO (security) |
| **Budget Line** | Regulatory compliance, operational risk, cybersecurity |
| **Key Competitor** | Build in-house (80% of deals) |
| **Pricing Model** | Usage-based SaaS: $0.001 per 1K tokens (with minimums) <br> Enterprise: Annual license + support |
| **Win Criteria** | 1. Sub-10ms latency (preserve UX) <br> 2. Regulatory policy packs (faster compliance) <br> 3. Cryptographic audit (regulator-ready) |
| **Primary Channel** | Direct sales (enterprise), Self-serve (SMB via docs/trials) |
| **Ecosystem** | Partner with LLM vendors (Anthropic, OpenAI), compliance firms (Deloitte, PwC), cloud platforms (AWS, GCP marketplace) |

---

## Pricing Strategy

### SaaS Tiers

| Tier | Target | Price | Features |
|------|--------|-------|----------|
| **Developer** | POCs, startups | Free (10M tokens/month) | Proxy mode, community policies, aggregate telemetry |
| **Professional** | SMB, scale-ups | $500/month + $0.001/1K tokens | Proxy + sidecar, pre-built policy packs, email support |
| **Enterprise** | Large orgs, regulated | Custom (starts $50K/year) | Control plane, custom policies, SLA, dedicated support, audit services |

### Enterprise Pricing Examples

**Neobank (5M daily active users)**:
- 50M conversations/month × 500 tokens/conversation = 25B tokens/month
- Base license: £200K/year
- Usage: 25B tokens × £0.0008/1K = £20K/month = £240K/year
- **Total**: £440K/year

**Investment Platform (500K users)**:
- 5M conversations/month × 300 tokens = 1.5B tokens/month
- Base license: £100K/year
- Usage: £12K/month = £144K/year
- **Total**: £244K/year

**Volume Discounts**:
- >50B tokens/month: 30% discount
- Multi-year commit: 20% discount
- Bundled control plane + nodes: 15% discount

---

## Partner Ecosystem

### Technology Partners

**LLM Vendors**:
- **Anthropic**: Joint go-to-market for Claude in regulated industries
- **OpenAI**: "Safety-verified" tier for enterprise customers
- **AWS Bedrock**: Marketplace listing, reference architecture

**Cloud Platforms**:
- **AWS**: Marketplace, co-sell motion, Solutions Architect enablement
- **Google Cloud**: Vertex AI integration, joint compliance webinars
- **Microsoft Azure**: Azure OpenAI certified partner

### Services Partners

**Compliance Consultancies**:
- **Deloitte, PwC, EY**: Include CheckStream in Consumer Duty implementation packages
- **Legal firms** (Linklaters, Clifford Chance): AI governance advisory

**System Integrators**:
- **Accenture, Capgemini**: Deployment services for large financial institutions

---

## Marketing & Demand Generation

### Content Marketing

**Thought Leadership**:
- Blog: "FCA Consumer Duty for Streaming AI: A Technical Guide"
- Whitepaper: "Real-Time Compliance: The New Standard for Financial LLMs"
- Case study: "How [Neobank] Achieved 100% Consumer Duty Coverage with CheckStream"

**SEO**:
- Keywords: "FCA Consumer Duty AI", "FINRA LLM compliance", "streaming guardrails", "prompt injection prevention"

### Events

**Conferences**:
- **Money20/20**: Fintech innovation + compliance
- **FCA TechSprints**: Direct regulator engagement
- **Black Hat / RSA**: Security community for AI firewall positioning

**Webinars**:
- "Consumer Duty Compliance for AI: What Financial Services Need to Know"
- "Preventing Prompt Injection in Production LLMs"

### Sales Enablement

**Proof of Concept Kit**:
- 30-day trial with FCA Consumer Duty policy pack
- Pre-loaded sample conversations (compliant vs non-compliant)
- Latency benchmark script
- Compliance report generator

**ROI Calculator**:
- Input: # customer interactions/month, avg tokens/interaction, compliance team size
- Output: Cost savings (manual review hours), risk mitigation (fines avoided), revenue enablement (faster features)

---

## Success Metrics (Company OKRs)

### Year 1 (Product-Market Fit)

- **ARR**: £2M
- **Customers**: 15 (10 financial services, 3 healthcare, 2 enterprise security)
- **NPS**: >50
- **Deployment success rate**: >90% (POCs → production)

### Year 2 (Scale)

- **ARR**: £12M
- **Customers**: 75
- **Expansion revenue**: 40% of new ARR
- **Partnership-sourced revenue**: 30%

### Year 3 (Market Leader)

- **ARR**: £50M
- **Customers**: 300+
- **Category leadership**: "Streaming LLM Guardrails" in Gartner Hype Cycle
- **Ecosystem**: 50+ SI/consultant partners

---

## Next Steps

- **Understand technical details**: [Architecture](architecture.md)
- **Explore deployment options**: [Deployment Modes](deployment-modes.md)
- **Review compliance capabilities**: [Regulatory Compliance](regulatory-compliance.md)
- **See real-world applications**: [Use Cases](use-cases.md)
