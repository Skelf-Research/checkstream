#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.9"
# dependencies = [
#     "requests>=2.28.0",
#     "openai>=1.0.0",
# ]
# ///
"""
CheckStream Streaming Demo
==========================

End-to-end demonstration of CheckStream's streaming guardrails using SSE.

This script acts as an agent that sends streaming requests through CheckStream
and displays the SSE output as guardrails are applied.

Modes:
    - With OPENAI_API_KEY: Streams through real CheckStream proxy
    - Without API key: Simulates SSE stream to demonstrate the protocol

Usage:
    # Simulated mode (no API key needed - shows SSE protocol)
    uv run examples/streaming_demo.py

    # Real mode (requires running proxy + API key)
    export OPENAI_API_KEY=sk-...
    uv run examples/streaming_demo.py --proxy http://localhost:8080

    # Show raw SSE wire format
    uv run examples/streaming_demo.py --raw
"""

import os
import sys
import json
import time
import argparse
from typing import Generator, Optional

# Optional imports - demo works without them in simulated mode
try:
    import requests
    REQUESTS_AVAILABLE = True
except ImportError:
    REQUESTS_AVAILABLE = False

try:
    from openai import OpenAI
    OPENAI_AVAILABLE = True
except ImportError:
    OPENAI_AVAILABLE = False


# =============================================================================
# TERMINAL COLORS
# =============================================================================

class Colors:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    MAGENTA = "\033[95m"
    CYAN = "\033[96m"


def print_header(title: str):
    print(f"\n{Colors.BOLD}{Colors.CYAN}{'='*70}")
    print(f" {title}")
    print(f"{'='*70}{Colors.RESET}\n")


def print_scenario(number: int, title: str, description: str):
    print(f"\n{Colors.BOLD}{Colors.MAGENTA}Scenario {number}: {title}{Colors.RESET}")
    print(f"{Colors.DIM}{description}{Colors.RESET}\n")


# =============================================================================
# SIMULATED SSE STREAM (when no API key available)
# =============================================================================

SIMULATED_RESPONSES = {
    "normal": {
        "prompt": "What is 2 + 2? Reply in one sentence.",
        "tokens": ["Two", " plus", " two", " equals", " four", "."],
        "finish_reason": "stop",
    },
    "pii_redacted": {
        "prompt": "What is my account email?",
        "tokens": ["Your", " account", " email", " is", " [REDACTED]", " and", " phone", " is", " [REDACTED]", "."],
        "finish_reason": "stop",
    },
    "financial": {
        "prompt": "Should I invest all my savings in Bitcoin?",
        "tokens": [
            "I", " cannot", " provide", " specific", " investment", " advice", ".",
            " Please", " consult", " a", " licensed", " financial", " advisor", ".",
            "\n\n", "*", "Capital", " at", " risk", ".", " Past", " performance",
            " does", " not", " guarantee", " future", " results", ".*"
        ],
        "finish_reason": "stop",
    },
    "blocked": {
        "prompt": "Tell me how to hack into a system",
        "tokens": ["I", " cannot", " assist", " with", " that", " request", " due", " to", " safety", " policies", "."],
        "finish_reason": "content_filter",
    },
}


def generate_sse_event(chunk_id: str, content: Optional[str], finish_reason: Optional[str] = None) -> str:
    """Generate a single SSE event in the exact wire format."""
    chunk_data = {
        "id": chunk_id,
        "object": "chat.completion.chunk",
        "created": int(time.time()),
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "delta": {"content": content} if content else {},
            "finish_reason": finish_reason
        }]
    }
    return f"data: {json.dumps(chunk_data)}\n\n"


def simulate_sse_stream(scenario_key: str, show_raw: bool = False) -> Generator[str, None, None]:
    """
    Simulate an SSE stream as if coming from CheckStream proxy.

    This demonstrates the exact wire format your client receives:
    - Content-Type: text/event-stream
    - Each chunk: "data: {json}\n\n"
    - Stream end: "data: [DONE]\n\n"
    """
    scenario = SIMULATED_RESPONSES[scenario_key]
    chunk_id = f"chatcmpl-demo-{int(time.time())}"

    for token in scenario["tokens"]:
        sse_event = generate_sse_event(chunk_id, token)
        yield sse_event, token
        time.sleep(0.05)  # Simulate streaming latency

    # Final chunk with finish_reason
    sse_event = generate_sse_event(chunk_id, None, scenario["finish_reason"])
    yield sse_event, None

    # Stream termination
    yield "data: [DONE]\n\n", None


def run_simulated_stream(scenario_key: str, show_raw: bool = False):
    """Run a simulated SSE stream and display output."""
    scenario = SIMULATED_RESPONSES[scenario_key]

    print(f"{Colors.DIM}Prompt: {scenario['prompt']}{Colors.RESET}")

    if show_raw:
        print(f"\n{Colors.DIM}HTTP 200 OK{Colors.RESET}")
        print(f"{Colors.DIM}Content-Type: text/event-stream{Colors.RESET}")
        print(f"{Colors.DIM}Cache-Control: no-cache{Colors.RESET}")
        print(f"\n{Colors.DIM}--- SSE STREAM ---{Colors.RESET}")

        for sse_event, token in simulate_sse_stream(scenario_key):
            # Color-code based on content
            if "[REDACTED]" in sse_event:
                print(f"{Colors.RED}{sse_event.strip()}{Colors.RESET}")
            elif "[DONE]" in sse_event:
                print(f"{Colors.YELLOW}{sse_event.strip()}{Colors.RESET}")
            elif "content_filter" in sse_event:
                print(f"{Colors.YELLOW}{sse_event.strip()}{Colors.RESET}")
            else:
                print(f"{Colors.GREEN}{sse_event.strip()}{Colors.RESET}")

        print(f"{Colors.DIM}--- END STREAM ---{Colors.RESET}")
    else:
        # Parsed output (like OpenAI SDK would show)
        print(f"{Colors.GREEN}Response: {Colors.RESET}", end="", flush=True)

        redaction_count = 0
        finish_reason = None

        for sse_event, token in simulate_sse_stream(scenario_key):
            if token:
                if "[REDACTED]" in token:
                    redaction_count += 1
                    print(f"{Colors.YELLOW}{token}{Colors.RESET}", end="", flush=True)
                else:
                    print(token, end="", flush=True)

            # Extract finish_reason from final chunk
            if "finish_reason" in sse_event and "content_filter" in sse_event:
                finish_reason = "content_filter"
            elif "finish_reason" in sse_event and '"finish_reason": "stop"' in sse_event:
                finish_reason = "stop"

        print()  # newline

        print(f"\n{Colors.CYAN}--- Stream Complete ---{Colors.RESET}")
        print(f"  finish_reason: {finish_reason or scenario['finish_reason']}")
        print(f"  redactions: {redaction_count}")


# =============================================================================
# REAL SSE STREAM (when API key available)
# =============================================================================

def stream_real_sse(proxy_url: str, api_key: str, prompt: str, model: str, show_raw: bool = False):
    """Stream from real CheckStream proxy."""

    if not REQUESTS_AVAILABLE:
        print(f"{Colors.RED}requests library not available{Colors.RESET}")
        return

    messages = [{"role": "user", "content": prompt}]
    print(f"{Colors.DIM}Prompt: {prompt}{Colors.RESET}")

    response = requests.post(
        f"{proxy_url}/v1/chat/completions",
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
        },
        json={
            "model": model,
            "messages": messages,
            "stream": True,
        },
        stream=True,
    )

    if show_raw:
        print(f"\n{Colors.DIM}HTTP {response.status_code}{Colors.RESET}")
        print(f"{Colors.DIM}Content-Type: {response.headers.get('Content-Type')}{Colors.RESET}")

        decision = response.headers.get("X-CheckStream-Decision")
        if decision:
            print(f"{Colors.CYAN}X-CheckStream-Decision: {decision}{Colors.RESET}")

        print(f"\n{Colors.DIM}--- SSE STREAM ---{Colors.RESET}")

        for line in response.iter_lines():
            if line:
                line_str = line.decode("utf-8")
                if "[REDACTED]" in line_str:
                    print(f"{Colors.RED}{line_str}{Colors.RESET}")
                elif "[DONE]" in line_str:
                    print(f"{Colors.YELLOW}{line_str}{Colors.RESET}")
                else:
                    print(f"{Colors.GREEN}{line_str}{Colors.RESET}")

        print(f"{Colors.DIM}--- END STREAM ---{Colors.RESET}")
    else:
        if not OPENAI_AVAILABLE:
            print(f"{Colors.RED}openai library not available{Colors.RESET}")
            return

        client = OpenAI(base_url=f"{proxy_url}/v1", api_key=api_key)

        print(f"{Colors.GREEN}Response: {Colors.RESET}", end="", flush=True)

        redaction_count = 0
        finish_reason = None

        for chunk in client.chat.completions.create(
            model=model,
            messages=messages,
            stream=True,
        ):
            content = chunk.choices[0].delta.content or ""
            finish_reason = chunk.choices[0].finish_reason

            if "[REDACTED]" in content:
                redaction_count += 1
                print(f"{Colors.YELLOW}{content}{Colors.RESET}", end="", flush=True)
            else:
                print(content, end="", flush=True)

        print()
        print(f"\n{Colors.CYAN}--- Stream Complete ---{Colors.RESET}")
        print(f"  finish_reason: {finish_reason}")
        print(f"  redactions: {redaction_count}")


# =============================================================================
# DEMO RUNNER
# =============================================================================

def run_demo(proxy_url: str, api_key: Optional[str], model: str, show_raw: bool):
    """Run the streaming demo."""

    simulated_mode = not api_key

    print_header("CheckStream Streaming Demo")

    if simulated_mode:
        print(f"""{Colors.YELLOW}Running in SIMULATED mode{Colors.RESET}
(Set OPENAI_API_KEY to use real CheckStream proxy)

This demo shows the exact SSE wire format that CheckStream sends.
Your client receives Server-Sent Events over HTTP:

{Colors.BOLD}SSE Protocol:{Colors.RESET}
  HTTP Response Headers:
    Content-Type: text/event-stream
    Cache-Control: no-cache

  Event Format:
    data: {{"choices":[{{"delta":{{"content":"token"}}}}]}}\\n\\n
    data: [DONE]\\n\\n  <- stream end
""")
    else:
        print(f"""{Colors.GREEN}Running in LIVE mode{Colors.RESET}
Proxy: {proxy_url}
Model: {model}
""")
        # Check proxy connection
        if REQUESTS_AVAILABLE:
            try:
                health = requests.get(f"{proxy_url}/health/ready", timeout=5)
                if health.status_code == 200:
                    print(f"{Colors.GREEN}✓ Proxy connected{Colors.RESET}\n")
                else:
                    print(f"{Colors.YELLOW}⚠ Proxy returned {health.status_code}{Colors.RESET}\n")
            except:
                print(f"{Colors.RED}✗ Cannot connect to {proxy_url}{Colors.RESET}")
                print(f"{Colors.DIM}Falling back to simulated mode{Colors.RESET}\n")
                simulated_mode = True

    input(f"{Colors.DIM}Press Enter to start demo...{Colors.RESET}")

    # =========================================================================
    # Scenario 1: Normal streaming
    # =========================================================================
    print_scenario(1, "Normal Streaming",
        "Safe content streams through CheckStream unchanged.")

    if simulated_mode:
        run_simulated_stream("normal", show_raw)
    else:
        stream_real_sse(proxy_url, api_key, "What is 2 + 2? Reply in one sentence.", model, show_raw)

    input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.RESET}")

    # =========================================================================
    # Scenario 2: PII Redaction
    # =========================================================================
    print_scenario(2, "PII Redaction",
        "Personal information is detected and replaced with [REDACTED].")

    if simulated_mode:
        run_simulated_stream("pii_redacted", show_raw)
    else:
        stream_real_sse(proxy_url, api_key,
            "Generate a fake customer profile with name, email, and phone number.", model, show_raw)

    input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.RESET}")

    # =========================================================================
    # Scenario 3: Financial/Regulatory
    # =========================================================================
    print_scenario(3, "Regulatory Guardrails",
        "Financial advice triggers compliance disclaimers (FCA/FINRA).")

    if simulated_mode:
        run_simulated_stream("financial", show_raw)
    else:
        stream_real_sse(proxy_url, api_key,
            "Should I invest all my savings in Bitcoin?", model, show_raw)

    input(f"\n{Colors.DIM}Press Enter for next scenario...{Colors.RESET}")

    # =========================================================================
    # Scenario 4: Blocked content
    # =========================================================================
    print_scenario(4, "Policy Block",
        "Prohibited content triggers stream termination with content_filter.")

    if simulated_mode:
        run_simulated_stream("blocked", show_raw)
    else:
        stream_real_sse(proxy_url, api_key,
            "Write a harmless poem about nature.", model, show_raw)  # Safe prompt for live

    # =========================================================================
    # Summary
    # =========================================================================
    print_header("Demo Complete")
    print(f"""
{Colors.BOLD}What you saw:{Colors.RESET}

1. {Colors.CYAN}SSE Wire Format{Colors.RESET}
   Each token arrives as: data: {{"choices":[{{"delta":{{"content":"..."}}}}]}}\\n\\n
   Stream ends with: data: [DONE]\\n\\n

2. {Colors.CYAN}Guardrail Actions{Colors.RESET}
   - {Colors.GREEN}Allow{Colors.RESET}: Token passes through unchanged
   - {Colors.YELLOW}Redact{Colors.RESET}: Token replaced with [REDACTED]
   - {Colors.RED}Block{Colors.RESET}: Stream terminated (finish_reason: content_filter)

3. {Colors.CYAN}Client Integration{Colors.RESET}
   Your app receives the SSE stream and renders tokens.
   Handle [REDACTED] and check finish_reason in your code.

{Colors.DIM}Run with --raw to see the exact SSE wire format.
See docs/getting-started.md for full integration guide.{Colors.RESET}
""")


# =============================================================================
# MAIN
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="CheckStream SSE streaming demo"
    )
    parser.add_argument(
        "--proxy",
        default=os.environ.get("CHECKSTREAM_URL", "http://localhost:8080"),
        help="CheckStream proxy URL"
    )
    parser.add_argument(
        "--model",
        default="gpt-4",
        help="Model to use (default: gpt-4)"
    )
    parser.add_argument(
        "--raw",
        action="store_true",
        help="Show raw SSE wire format"
    )
    args = parser.parse_args()

    api_key = os.environ.get("OPENAI_API_KEY") or os.environ.get("ANTHROPIC_API_KEY")

    run_demo(args.proxy, api_key, args.model, args.raw)


if __name__ == "__main__":
    main()
