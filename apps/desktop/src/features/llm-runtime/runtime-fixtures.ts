import type { AgentActivitySnapshot, AgentRuntimeSettingsSnapshot, LlmUsageSnapshot } from "@meowlive/contracts";

export function runtimeSnapshot(): AgentRuntimeSettingsSnapshot {
  return { settings: { cache_enabled: true, streaming: true, tools_enabled: true, environment_enabled: true,
    web_search_enabled: false, search_provider: "brave", search_endpoint: "https://api.search.brave.com/res/v1/web/search",
    max_tool_rounds: 3, tool_timeout_seconds: 8, prices: [] }, search_key_configured: true, storage_available: true };
}
export function usageSnapshot(): LlmUsageSnapshot {
  const totals = { calls: 2, input_tokens: 1200, output_tokens: 180, cache_read_tokens: 1000,
    cache_write_tokens: 0, reasoning_tokens: 50, estimated_cost_microusd: 12500, unpriced_calls: 1, unknown_usage_calls: 1 };
  return { totals, groups: [{ provider: "openai", base_url: "https://api.openai.com/v1", model: "chat-model", totals }],
    records: [{ id: "call-1", started_at_ms: 1758502800000, provider: "openai", base_url: "https://api.openai.com/v1",
      api_format: "openai_responses", model: "chat-model", operation: "agent", status: "failed", latency_ms: 500,
      first_token_ms: null, estimated_cost_microusd: null,
      usage: { input_tokens: null, output_tokens: null, cache_read_tokens: null, cache_write_tokens: null, reasoning_tokens: null } }],
    storage_available: true, truncated: false };
}
export function activitySnapshot(): AgentActivitySnapshot {
  return { run_id: "run-1", phase: "tool", started_at_ms: 1758502800000, updated_at_ms: 1758502801500,
    output_characters: 80, tool_round: 1, tools: [{ name: "web_search", status: "running", elapsed_ms: 350,
      sources: ["https://example.com/article"] }], message: "PRIVATE_MODEL_TEXT" };
}
