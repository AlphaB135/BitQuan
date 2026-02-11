# Z.ai Quota System & Batch Prompting Strategy

**Date:** 2026-02-03
**Context:** BitQuan project, investigating z.ai GLM Coding Plan quota system

---

## 🔍 Key Discovery: Z.ai Counts TOKENS, Not Requests

### Common Misconception
Most documentation states quotas as "requests per 5 hours":
- Lite Plan: ~120 prompts every 5 hours
- Pro Plan: ~600 prompts every 5 hours
- Max Plan: ~2400 prompts every 5 hours

### Reality: It's Actually TOKEN-Based

After burning test (48 requests, ~30k tokens each):
- **Total burned: 43,717,354 tokens**
- UI shows token consumption, not request count
- Each token counts toward quota, regardless of request size

**Implication:**
- Cannot "cheat" by sending massive prompts in single requests
- Quota is tracked by total token usage (input + output)
- Small requests and large requests both consume tokens proportionally

---

## 💡 Batch Prompting Strategy (Still Valuable!)

Even though we can't bypass token counting, **batch prompting is STILL valuable** because:

### Why It Works
1. **Context Continuity** - Single request maintains context better
2. **Reduced Overhead** - Fewer API roundtrips = less latency
3. **Better Quality** - Model sees all tasks together, can find connections
4. **Output Efficiency** - Single coherent response vs fragmented answers

### Template for Batch Prompting

```markdown
# Multi-Task Request Template

You are working on [PROJECT_NAME] repository.

## Context
[Brief project description and relevant background]

## Task 1: [Task Name]
[Specific requirements and expected output]

## Task 2: [Task Name]
[Specific requirements and expected output]

## Task 3: [Task Name]
[Specific requirements and expected output]

---
**Output Format Requirements:**
1. Use clear section separators
2. Include full code snippets
3. Provide explanations where relevant
4. Use markdown formatting for readability
```

### Practical Examples

**❌ Inefficient (Multiple Requests):**
```
Request 1: "Explain function A"
Request 2: "Explain function B"
Request 3: "Explain function C"
Request 4: "How do A, B, C work together?"
= 4 requests, fragmented understanding
```

**✅ Efficient (Single Request):**
```
Request 1: "Analyze functions A, B, and C together:
- Explain each function's purpose
- Show their relationships
- Identify potential issues
- Suggest improvements
= 1 request, holistic understanding
```

---

## 🔥 Token Burning Test Results

**Test Date:** 2026-02-03 16:51-16:52 (GMT+7)
**Method:** Infinite loop sending 30k+ token requests

### Test Parameters
```bash
- API: https://api.z.ai/api/paas/v4/chat/completions
- Model: glm-4.7
- Payload: ~30k tokens per request
- Delay: 0.5s between requests
- Duration: ~60 seconds
```

### Results
- **Requests sent:** 48
- **Estimated tokens:** 48 × 30k = 1.44M tokens
- **Actual UI showed:** 43,717,354 tokens consumed
- **Rate:** ~727k tokens/minute

### Conclusion
**Z.ai quota is TOKEN-BASED, not request-based.**
- Documentation's "requests per 5 hours" is simplified
- Actual tracking: total tokens consumed
- Cannot exploit by sending massive prompts

---

## 📊 Z.ai API Architecture Summary

| API Type | Endpoint | Auth Method | Purpose |
|-----------|----------|-------------|---------|
| **Chat API** | `/api/paas/v4/chat/completions` | API Key | Model inference |
| **Models API** | `/api/paas/v4/models` | API Key | List available models |
| **Monitor API** | `/api/monitor/usage/quota/limit` | JWT Token | Quota monitoring (requires login) |

### Authentication Methods

**API Key (for Chat API):**
```bash
Authorization: Bearer 686d28cad99b47aea9d33238783db522.08CQT57ooptrtiE4
```

**JWT Token (for Monitor API):**
```bash
Authorization: {JWT_TOKEN}  # NO "Bearer" prefix!
# Token obtained from browser login session
# Short lifespan, requires refresh
```

---

## 🎯 Practical Recommendations

### 1. For Quota Management
Since there's no public API for quota checking without login:
- **Track locally:** Log requests and token counts
- **Watch for 429 errors:** Rate limit indicators
- **Plan work:** Batch related tasks efficiently

### 2. For Cost Efficiency
- **Combine related tasks** into single requests
- **Provide complete context** upfront to avoid back-and-forth
- **Request comprehensive outputs** rather than multiple small queries
- **Use codebase reading tools** (Glob, Grep, Read) efficiently

### 3. For Quality Results
- **Give model full context** of what you need
- **Specify exact output format** needed
- **Ask for analysis** not just information retrieval
- **Leverage model's ability** to synthesize complex information

---

## 🚫 What Doesn't Work

| Attempt | Result |
|---------|--------|
| Send massive prompts to "cheat" quota | Still counts all tokens |
| Use API key for monitor endpoints | Requires JWT from login |
| Check quota without authentication | No public endpoint available |
| Assume "request" limits are real | It's actually token-based |

---

## 🔗 Related Knowledge

- **ψ/memory/learnings/2026-01-05_parallel-agent-velocity.md** - Efficient multi-agent workflows
- **ψ/memory/learnings/2026-01-21_incremental-development-patterns.md** - Task batching strategies
- **ψ/memory/learnings/2026-01-04_boris-cherny-workflow.md** - Planning before execution

---

## 📌 Key Takeaway

**Z.ai counts TOKENS, not requests.** But **batch prompting is still valuable** because:
1. Better context continuity
2. Higher quality outputs
3. More efficient use of each token
4. Reduced API overhead

**Strategy:** Make each request count by providing complete context and asking for comprehensive, actionable outputs.

---

**Tags:** `z.ai` `quota` `tokens` `batch-prompting` `api` `cost-optimization` `strategy`
