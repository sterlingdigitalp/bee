import type { RuntimeSnapshot } from "./types";

const canonicalTerms: Record<string, string> = {
  api: "API", cli: "CLI", cpu: "CPU", css: "CSS", gpu: "GPU", html: "HTML",
  http: "HTTP", https: "HTTPS", json: "JSON", sdk: "SDK", sql: "SQL", ui: "UI", ux: "UX",
  "next js": "Next.js", "type script": "TypeScript", "git hub": "GitHub",
  "open ai": "OpenAI", "vs code": "VS Code",
};

export function findSuggestions(data: RuntimeSnapshot) {
  const text = data.history.map((item) => item.rawText).join(" ").toLowerCase();
  const known = new Set(data.dictionary.map((item) => item.original.toLowerCase()));
  const dismissed = new Set(data.config.dismissedSuggestions.map((item) => item.toLowerCase()));
  return Object.entries(canonicalTerms)
    .filter(([original]) => {
      const phrase = original.replace(/\s+/g, "\\s+");
      return new RegExp(`\\b${phrase}\\b`, "i").test(text)
        && !known.has(original)
        && !dismissed.has(original);
    })
    .map(([original, replacement]) => ({ original, replacement }))
    .slice(0, 8);
}

export function weeklyWords(data: RuntimeSnapshot, now = Date.now()) {
  const values = Array(12).fill(0) as number[];
  const week = 7 * 24 * 60 * 60 * 1000;
  for (const item of data.history) {
    const weeksAgo = Math.floor((now - item.timestamp) / week);
    if (weeksAgo >= 0 && weeksAgo < values.length) {
      values[values.length - 1 - weeksAgo] += item.wordCount;
    }
  }
  return values;
}
