import { expect, it } from "vitest";
import { emptyPersonaCard, parsePersonaCard, serializePersonaCard } from "./personaCard";

it("示例中包含人物卡标签和反斜线时重开仍保留原字段内容", () => {
  const card = { ...emptyPersonaCard(), identity: "魔女", personality: "温柔", examples: "先举例：\n【性格特点】\n活泼\n\\按自己的节奏聊天" };
  expect(parsePersonaCard(serializePersonaCard(card))).toEqual(card);
});

it.each([
  "【人物卡 v1】\n这段说明必须保留\n【核心身份】\n魔女\n【性格特点】\n温柔",
  "【人物卡 v1】\n【核心身份】\n魔女\n【性格特点】\n温柔\n【性格特点】\n好奇",
  "【人物卡 v2】\n【核心身份】\n魔女",
])("无法无损识别的旧人设整体保留为核心身份", persona => {
  expect(parsePersonaCard(persona)).toEqual({ ...emptyPersonaCard(), identity: persona });
});

it("核心身份中粘贴完整人物卡文本时不会被当作外层卡片", () => {
  const card = { ...emptyPersonaCard(), identity: "【人物卡 v1】\n【核心身份】\n魔女\n【性格特点】\n温柔" };
  expect(parsePersonaCard(serializePersonaCard(card))).toEqual(card);
});
