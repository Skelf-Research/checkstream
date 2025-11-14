# CheckStream for Agent Systems

**How CheckStream integrates with multi-step agent workflows**

---

## The Agent Challenge

Modern AI systems aren't just simple prompt→response. They're **multi-step agent systems**:

```
User Query: "What's the weather in San Francisco and should I bring an umbrella?"

Agent Steps:
  1. Parse intent (tool planning)
  2. Call weather API (tool execution)
  3. Retrieve forecast data
  4. Analyze results
  5. Formulate response
  6. STREAM final answer to user ← CHECKSTREAM APPLIES HERE
```

**CheckStream doesn't care about steps 1-5.** It only guards step 6 (the final streaming output).

---

## How It Works with Agents

### Agent Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Your Agent System                     │
│                                                           │
│  ┌─────────┐   ┌──────────┐   ┌─────────┐   ┌────────┐ │
│  │ ReAct   │→  │ Tool Use │→  │ RAG     │→  │ Plan   │ │
│  │ Loop    │   │ (API)    │   │ Search  │   │ Execute│ │
│  └─────────┘   └──────────┘   └─────────┘   └────────┘ │
│                                                           │
│                         ↓                                 │
│                                                           │
│              ┌─────────────────────┐                     │
│              │  Final Response     │                     │
│              │  (Stream to user)   │                     │
│              └──────────┬──────────┘                     │
└─────────────────────────┼────────────────────────────────┘
                          │
                          │ This is what CheckStream guards
                          ↓
                  ┌───────────────┐
                  │  CheckStream  │
                  │  3 Phases     │
                  └───────────────┘
                          ↓
                      User sees safe output
```

### Two Integration Patterns

#### Pattern 1: Agent Framework with Streaming Output

**Your agent framework** (LangChain, AutoGen, CrewAI, custom):

```python
# Your agent code
agent = YourAgentFramework()

# Agent does its thing (tools, planning, etc.)
intermediate_steps = agent.run_tools(query)

# Final step: Generate streaming response
# Point THIS to CheckStream
response = agent.stream_final_response(
    intermediate_results=intermediate_steps,
    llm_endpoint="http://localhost:8080/v1"  # CheckStream proxy
)

for chunk in response:
    yield chunk  # Safe, checked by CheckStream
```

**What happens**:
1. Agent runs tools, retrieval, planning (no CheckStream involvement)
2. Agent sends final prompt to LLM **via CheckStream**
3. CheckStream applies 3 phases to the final streaming response
4. User sees safe output

#### Pattern 2: Agent Calls LLM Multiple Times

**Some agents make multiple LLM calls**:

```python
# Agent workflow
agent = YourAgent()

# Step 1: Planning (internal, no CheckStream)
plan = agent.llm.complete("Create a plan for: " + query)

# Step 2: Tool execution (internal, no CheckStream)
results = agent.execute_tools(plan)

# Step 3: Final synthesis (goes through CheckStream)
response = agent.llm.stream_complete(
    "Synthesize results: " + str(results),
    endpoint="http://localhost:8080/v1"  # Only this call is guarded
)
```

**CheckStream only guards the final user-facing output.**

---

## Configuration for Agent Systems

### Flexible Phase Application

```yaml
# config.yaml - Agent-friendly configuration

pipelines:
  # Phase 1: Check final prompt (after agent assembly)
  ingress_pipeline: "agent-aware-safety"

  # Phase 2: Check streaming final response
  midstream_pipeline: "fast-triage"

  # Phase 3: Verify complete agent response
  egress_pipeline: "comprehensive-audit"

  # Agent responses might be longer
  streaming:
    context_chunks: 10     # More context for multi-step reasoning
    max_buffer_size: 200   # Larger buffer for agent responses
```

### Agent-Specific Pipelines

```yaml
# classifiers.yaml

pipelines:
  agent-aware-safety:
    description: "Safety checks for agent-assembled prompts"
    stages:
      - type: parallel
        name: agent-checks
        classifiers:
          - pii                    # Agent might include user data
          - prompt-injection       # Agent might be manipulated
          - tool-misuse-detection  # Custom: Check for tool abuse
        aggregation: max_score

  agent-response-audit:
    description: "Audit trail for agent decisions"
    stages:
      - type: sequential
        classifiers:
          - factual-accuracy       # Agent might hallucinate facts
          - source-attribution     # Agent should cite sources
          - compliance-check       # Final answer compliance
```

---

## Real-World Examples

### Example 1: RAG Agent with Streaming

**Agent**: Retrieval-Augmented Generation chatbot

```python
from langchain.agents import create_openai_functions_agent
from langchain.tools import Tool

# Your RAG tools
tools = [
    Tool(name="search_docs", func=search_documents),
    Tool(name="query_db", func=query_database),
]

# Agent configuration
agent = create_openai_functions_agent(
    llm=ChatOpenAI(
        base_url="http://localhost:8080/v1",  # CheckStream proxy
        api_key=os.environ["OPENAI_API_KEY"],
        streaming=True
    ),
    tools=tools,
)

# Agent workflow
for query in user_queries:
    # 1. Agent retrieves docs (internal, not guarded)
    docs = agent.tools["search_docs"](query)

    # 2. Agent analyzes (internal, not guarded)
    analysis = agent.analyze(docs)

    # 3. Agent streams final answer (GUARDED by CheckStream)
    for chunk in agent.stream(query):
        # CheckStream checks each chunk
        yield chunk
```

**CheckStream Involvement**:
- ❌ Doesn't see tool calls
- ❌ Doesn't see retrieval
- ❌ Doesn't see internal reasoning
- ✅ **Only sees and guards the final streaming response**

### Example 2: Multi-Agent System

**System**: Multiple specialized agents collaborate

```python
# Three agents with different roles
researcher_agent = Agent(role="researcher")
analyst_agent = Agent(role="analyst")
writer_agent = Agent(role="writer",
                     llm_endpoint="http://localhost:8080/v1")  # Only writer uses CheckStream

# Multi-step workflow
query = "Analyze Q4 earnings"

# Step 1: Researcher gathers data (internal)
data = researcher_agent.gather_data(query)

# Step 2: Analyst processes (internal)
insights = analyst_agent.analyze(data)

# Step 3: Writer generates final report (GUARDED)
for chunk in writer_agent.stream_report(insights):
    yield chunk  # Safe, checked by CheckStream
```

**CheckStream only guards the final user-facing output from the writer agent.**

### Example 3: Function Calling Agent

**Agent**: Uses OpenAI function calling

```python
import openai

openai.api_base = "http://localhost:8080/v1"  # CheckStream proxy

# Define functions
functions = [
    {
        "name": "get_weather",
        "description": "Get weather for a location",
        "parameters": {...}
    },
    {
        "name": "get_news",
        "description": "Get latest news",
        "parameters": {...}
    }
]

# Agent loop
messages = [{"role": "user", "content": "What's the weather and news in SF?"}]

while True:
    response = openai.ChatCompletion.create(
        model="gpt-4",
        messages=messages,
        functions=functions,
        stream=True  # Streaming final response
    )

    # Check if function call needed
    if response.function_call:
        # Execute function (internal, not guarded)
        result = execute_function(response.function_call)
        messages.append(result)
        continue  # Loop back

    # Final response (GUARDED by CheckStream)
    for chunk in response:
        yield chunk.choices[0].delta.content
    break
```

**CheckStream sees**:
- ✅ The final streaming response to user
- ❌ Not the function call requests (internal agent decisions)

---

## Why This Flexibility Matters

### 1. Agent Autonomy

Agents need to make **internal decisions** without guardrail interference:

```
Agent Internal Steps (No CheckStream):
  ✓ Which tools to call
  ✓ How to parse results
  ✓ Planning and reasoning
  ✓ Intermediate LLM calls for planning

Agent Final Output (CheckStream Applied):
  ✓ The streaming response to user
```

### 2. Performance

Only guarding the **final output** keeps agent workflows fast:

```
Without selective guarding:
  Tool call 1 → CheckStream (10ms) ❌
  Tool call 2 → CheckStream (10ms) ❌
  Planning → CheckStream (10ms) ❌
  Final output → CheckStream (10ms) ❌
  Total overhead: 40ms

With selective guarding:
  Tool call 1 → Direct (0ms) ✓
  Tool call 2 → Direct (0ms) ✓
  Planning → Direct (0ms) ✓
  Final output → CheckStream (10ms) ✓
  Total overhead: 10ms
```

### 3. Flexibility

Different agent architectures have different streaming points:

```yaml
# Configuration adapts to YOUR agent architecture

# Option 1: Agent streams final synthesis
streaming:
  context_chunks: 10   # Longer context for multi-step reasoning

# Option 2: Agent returns complete response
streaming:
  context_chunks: 0    # Full context for complete analysis

# Option 3: Agent streams intermediate outputs
streaming:
  context_chunks: 3    # Less context, faster checks
```

---

## Integration Patterns

### Pattern A: Agent Framework Wrapper

**Wrap your agent framework**:

```python
class SafeAgent:
    def __init__(self, agent_framework):
        self.agent = agent_framework
        self.checkstream_url = "http://localhost:8080/v1"

    async def run(self, query):
        # Agent does internal steps (no CheckStream)
        intermediate = await self.agent.plan_and_execute(query)

        # Final synthesis goes through CheckStream
        final_prompt = self.agent.build_final_prompt(intermediate)

        async for chunk in self.stream_via_checkstream(final_prompt):
            yield chunk

    async def stream_via_checkstream(self, prompt):
        # Point to CheckStream for final streaming
        response = await openai.ChatCompletion.create(
            base_url=self.checkstream_url,
            messages=[{"role": "user", "content": prompt}],
            stream=True
        )

        async for chunk in response:
            yield chunk
```

### Pattern B: Selective LLM Routing

**Route only final calls through CheckStream**:

```python
class AgentWithSelectiveGuarding:
    def __init__(self):
        # Internal LLM (no guardrails, fast)
        self.internal_llm = ChatOpenAI(
            base_url="https://api.openai.com/v1"
        )

        # External LLM (with guardrails)
        self.external_llm = ChatOpenAI(
            base_url="http://localhost:8080/v1"  # CheckStream
        )

    async def run(self, query):
        # Planning: Use internal LLM (fast, no guardrails)
        plan = await self.internal_llm.complete("Plan: " + query)

        # Tool execution: Direct (no LLM)
        results = self.execute_tools(plan)

        # Final response: Use external LLM (with guardrails)
        async for chunk in self.external_llm.stream("Synthesize: " + str(results)):
            yield chunk  # Safe, checked
```

### Pattern C: Post-Agent Streaming Wrapper

**Apply CheckStream AFTER agent completes**:

```python
class PostAgentGuarding:
    async def run_agent_with_guardrails(self, query):
        # 1. Agent runs completely (internal LLM, no CheckStream)
        agent_result = await self.agent.complete(query)

        # 2. Stream result through CheckStream for final check
        async for chunk in self.stream_through_checkstream(agent_result):
            yield chunk

    async def stream_through_checkstream(self, text):
        # Send pre-generated text through CheckStream
        # CheckStream checks it as it streams to user
        response = requests.post(
            "http://localhost:8080/v1/chat/completions",
            json={
                "model": "passthrough",
                "messages": [{"role": "assistant", "content": text}],
                "stream": True
            },
            stream=True
        )

        for line in response.iter_lines():
            yield parse_sse(line)
```

---

## Configuration Best Practices

### For Fast Agent Loops

```yaml
# Minimal overhead for rapid agent iterations
pipelines:
  ingress_pipeline: "pattern-only"      # Regex only, <1ms
  midstream_pipeline: "ultra-fast"      # Distilled models
  chunk_threshold: 0.95                 # Permissive

  streaming:
    context_chunks: 1                   # Minimal context
```

### For Compliance-Critical Agents

```yaml
# Comprehensive checking for regulated domains
pipelines:
  ingress_pipeline: "comprehensive"
  midstream_pipeline: "full-safety"
  egress_pipeline: "audit-trail"
  chunk_threshold: 0.6                  # Strict

  streaming:
    context_chunks: 0                   # Full context
    max_buffer_size: 500                # Large buffer
```

### For Multi-Turn Agent Conversations

```yaml
# Context-aware for conversational agents
pipelines:
  streaming:
    context_chunks: 20                  # Long context window
    # Agents build on previous turns
```

---

## Summary

### ✅ CheckStream is Agent-Friendly

1. **Flexible Integration**: Guard only what needs guarding (final output)
2. **Performance**: No overhead on internal agent steps
3. **Configurable**: Adapt to any agent architecture
4. **Non-Invasive**: Works with existing agent frameworks

### Agent Integration Checklist

- [ ] Identify where agent streams final response to user
- [ ] Point THAT endpoint to CheckStream
- [ ] Keep internal agent decisions direct (no proxy)
- [ ] Configure context windows for agent response length
- [ ] Test with your agent framework

### Supported Agent Frameworks

✅ **LangChain** - Works with `streaming=True` LLM calls
✅ **AutoGen** - Works with final synthesis step
✅ **CrewAI** - Works with writer agent output
✅ **Semantic Kernel** - Works with planner final output
✅ **Custom Agents** - Works with any final streaming step

### Key Insight

> **CheckStream doesn't need to understand your agent's internals.**
> It just guards the final streaming output to users.
> Your agent's tools, planning, and reasoning stay private and fast.

---

## See Also

- [Design Principles](DESIGN_PRINCIPLES.md) - Provider agnosticism
- [Architecture](architecture.md) - Three-phase system
- [Integration Guide](INTEGRATION_GUIDE.md) - General integration
- [Examples](../examples/) - Code examples

---

**Questions about agent integration?** Check the examples or open an issue on GitHub.
