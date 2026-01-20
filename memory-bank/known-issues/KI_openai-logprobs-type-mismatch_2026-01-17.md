---
title: OpenAI Logprobs Type Mismatch in async-openai
date: 2026-01-17
severity: medium
status: resolved
tags: [openai, logprobs, type-error, rust]
crate: async-openai
version: "0.25"
---

# Known Issue: OpenAI Logprobs Type Mismatch

## Problem

When working with OpenAI logprobs in the `async-openai` crate (v0.25), the `token.logprob` field is a direct `f32` value, not `Option<f32>` as might be expected.

## Symptoms

Compilation error when trying to use `filter_map` or treating `token.logprob` as optional:

```rust
// This fails:
let sum: f64 = content_logprobs.iter()
    .filter_map(|token| token.logprob)  // ❌ Error
    .map(|logprob| logprob.exp())
    .sum();

// Error message:
error[E0308]: mismatched types
   --> crates\backend\src\shared\llm\openai_provider.rs:133:41
    |
133 |                     .filter_map(|token| token.logprob)
    |                                         ^^^^^^^^^^^^^ expected `Option<_>`, found `f32`
```

## Root Cause

The `async-openai` crate defines logprobs as non-optional `f32` values in the response structure. This is based on OpenAI API behavior where logprobs, when requested, are always present for each token.

## Detection

- Compilation errors with `filter_map` on `token.logprob`
- Type mismatch errors expecting `Option<f32>` but finding `f32`
- Occurs when processing `choice.logprobs.content` in OpenAI responses

## Solution

Directly access and cast the `f32` value without treating it as optional:

```rust
// ✅ Correct approach:
let confidence = choice.logprobs.as_ref().and_then(|logprobs| {
    if let Some(content_logprobs) = &logprobs.content {
        if content_logprobs.is_empty() {
            return None;
        }
        
        // Directly map the f32 values
        let sum: f64 = content_logprobs.iter()
            .map(|token| (token.logprob as f64).exp())
            .sum();
        let count = content_logprobs.len();
        
        if count > 0 {
            Some(sum / count as f64)
        } else {
            None
        }
    } else {
        None
    }
});
```

## Key Points

1. **Check outer structures first**: `choice.logprobs` and `logprobs.content` are `Option`, but individual `token.logprob` is not
2. **No filter_map needed**: Since logprob is always present, just use `.map()`
3. **Cast for calculations**: Cast `f32` to `f64` for precision in calculations

## Prevention

When working with OpenAI responses:
- Check the crate documentation for actual types
- Test with small example before writing complex logic
- Use `cargo check` frequently during development

## Related

- [[RB_llm-chat-enhancement_v1]] - Full runbook for LLM feature enhancements
- OpenAI logprobs documentation: https://platform.openai.com/docs/api-reference/chat/create#chat-create-logprobs

## Impact

- **Severity**: Medium (blocks compilation, but easy to fix once identified)
- **Frequency**: Occurs when first implementing logprobs support
- **Time to Fix**: ~5 minutes once issue is understood
