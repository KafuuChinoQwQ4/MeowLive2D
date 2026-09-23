export type PersonaCard = {
  identity: string;
  name: string;
  background: string;
  personality: string;
  speechStyle: string;
  interests: string;
  viewerRelationship: string;
  interactionHabits: string;
  values: string;
  boundaries: string;
  examples: string;
};

export const emptyPersonaCard = (): PersonaCard => ({
  identity: "",
  name: "",
  background: "",
  personality: "",
  speechStyle: "",
  interests: "",
  viewerRelationship: "",
  interactionHabits: "",
  values: "",
  boundaries: "",
  examples: "",
});

const HEADER = "【人物卡 v1】";
const fields: readonly [keyof PersonaCard, string][] = [
  ["identity", "核心身份"],
  ["name", "角色姓名与称呼"],
  ["background", "背景经历"],
  ["personality", "性格特点"],
  ["speechStyle", "说话风格"],
  ["interests", "兴趣与擅长"],
  ["viewerRelationship", "与观众的关系"],
  ["interactionHabits", "互动习惯"],
  ["values", "价值观与目标"],
  ["boundaries", "互动禁忌与边界"],
  ["examples", "示例表达"],
];
const markers = new Map(fields.map(([key, label]) => [`【${label}】`, key]));

function escapeContent(content: string): string {
  return content.split("\n").map(line =>
    line.startsWith("\\") || markers.has(line.trim()) ? `\\${line}` : line,
  ).join("\n");
}

export function serializePersonaCard(card: PersonaCard): string {
  const normalized = Object.fromEntries(
    fields.map(([key]) => [key, card[key].trim()]),
  ) as PersonaCard;
  const hasDetails = fields.slice(1).some(([key]) => normalized[key].length > 0);
  if (!hasDetails && !parseStructuredCard(normalized.identity)) return normalized.identity;
  return serializeStructuredCard(normalized);
}

function serializeStructuredCard(card: PersonaCard): string {
  const sections = fields
    .filter(([key]) => card[key])
    .map(([key, label]) => `【${label}】\n${escapeContent(card[key])}`);
  return [HEADER, ...sections].join("\n");
}

export function parsePersonaCard(persona: string): PersonaCard {
  return parseStructuredCard(persona) ?? { ...emptyPersonaCard(), identity: persona };
}

function parseStructuredCard(persona: string): PersonaCard | undefined {
  const card = emptyPersonaCard();
  const lines = persona.split("\n");
  if (lines[0] !== HEADER) return undefined;

  let current: keyof PersonaCard | undefined;
  const content: string[] = [];
  const flush = () => {
    if (current) card[current] = content.join("\n").trim();
    content.length = 0;
  };
  for (const line of lines.slice(1)) {
    const key = markers.get(line.trim());
    if (key) {
      flush();
      current = key;
    } else if (current) {
      const unescaped = line.slice(1);
      content.push(line.startsWith("\\") && (unescaped.startsWith("\\") || markers.has(unescaped.trim())) ? unescaped : line);
    }
  }
  flush();
  // Only split a canonical card that can be reproduced without dropping content.
  // Unknown formats, duplicate sections and older freeform prompts stay intact.
  return card.identity && serializeStructuredCard(card) === persona ? card : undefined;
}
