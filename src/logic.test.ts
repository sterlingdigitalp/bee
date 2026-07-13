import { describe, expect, it } from "vitest";
import { defaultConfig, type RuntimeSnapshot } from "./types";
import { findSuggestions, weeklyWords } from "./logic";

function snapshot(): RuntimeSnapshot {
  return {
    config: { ...defaultConfig, onboardingComplete: true }, models: [], history: [], dictionary: [],
    stats: { totalWords: 0, totalSeconds: 0, totalSessions: 0, averageWpm: 0, todayWords: 0, weekWords: 0 },
    audioDevices: [], version: "test",
  };
}

describe("local learning", () => {
  it("suggests canonical terms and respects dismissals", () => {
    const data = snapshot();
    data.history.push({ id: "1", text: "use the api", rawText: "use the api with next js", timestamp: 1, wordCount: 6, durationSeconds: 2, transcriptionMs: 20, model: "Tiny", source: "local" });
    expect(findSuggestions(data)).toEqual(expect.arrayContaining([
      { original: "api", replacement: "API" }, { original: "next js", replacement: "Next.js" },
    ]));
    data.config.dismissedSuggestions = ["api"];
    expect(findSuggestions(data)).not.toContainEqual({ original: "api", replacement: "API" });
  });

  it("buckets history into twelve weeks", () => {
    const data = snapshot();
    const now = 1_800_000_000_000;
    data.history.push({ id: "1", text: "today", rawText: "today", timestamp: now, wordCount: 4, durationSeconds: 1, transcriptionMs: 10, model: "Tiny", source: "local" });
    data.history.push({ id: "2", text: "last week", rawText: "last week", timestamp: now - 8 * 24 * 60 * 60 * 1000, wordCount: 7, durationSeconds: 1, transcriptionMs: 10, model: "Tiny", source: "local" });
    const values = weeklyWords(data, now);
    expect(values.at(-1)).toBe(4);
    expect(values.at(-2)).toBe(7);
  });
});
