import { interactionSettings } from "../../test/agent-fixtures";
import { expect, it, vi } from "vitest";
import { createAgentClient } from "./agent";
import { jsonResponse } from "../../test/server-fixtures";

function snapshot() {
  return { paused:true,phase:"paused",settings:{persona:"猫咪主播",system_prompt:"",topic:"",proactive_enabled:false,cooldown_ms:30000,interaction:{...interactionSettings}},
    events:[{event:{id:"e1",source:"simulator",viewer:"观众",kind:{type:"gift",name:"花",count:1}},status:"pending",speech_id:null,error:null}],
    last_error:null,current_speech_id:null,llm_configured:false,bridge_connected:false };
}
it("接受合法的最大调度配置下的历史与活动事件总量", async () => {
  const value=snapshot(); value.events=Array.from({length:2528},(_,i)=>({...value.events[0]!,event:{...value.events[0]!.event,id:`e${i}`}}));
  const fetcher=vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
  expect((await createAgentClient({fetcher}).getStatus()).events).toHaveLength(2528);
});
it.each([0,3600001])("拒绝不合法的冷却 %s", async (cooldown_ms) => {
  const value=snapshot();value.settings.cooldown_ms=cooldown_ms;
  const fetcher=vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
  await expect(createAgentClient({fetcher}).getStatus()).rejects.toMatchObject({code:"invalid_response"});
});
it.each([0,10001])("拒绝不合法的礼物数量 %s", async (count) => {
  const value=snapshot();value.events[0]!.event.kind.count=count;
  const fetcher=vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
  await expect(createAgentClient({fetcher}).getStatus()).rejects.toMatchObject({code:"invalid_response"});
});
