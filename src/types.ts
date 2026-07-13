export type Theme = "black" | "light";
export type TranscriptionMode = "local" | "cloud";
export type RecordingState = "idle" | "listening" | "processing" | "success" | "error";

export interface AppConfig {
  transcriptionMode: TranscriptionMode;
  model: string;
  cloudLanguage: string;
  pushToTalkKey: string;
  toggleKey: string | null;
  shortcutsPaused: boolean;
  dismissedSuggestions: string[];
  closeNoticeSeen: boolean;
  recordingMode: "push-to-talk" | "toggle";
  preferredInputDevice: string | null;
  fallbackInputDevice: string | null;
  inputGain: number;
  autoPunctuation: boolean;
  removeFillers: boolean;
  copyToClipboard: boolean;
  autoEnhancePrompt: boolean;
  customInstructions: string;
  dictionaryEnabled: boolean;
  recordingSoundEnabled: boolean;
  autoHideWidget: boolean;
  showWidget: boolean;
  followCursor: boolean;
  lockWidgetPosition: boolean;
  theme: Theme;
  launchAtLogin: boolean;
  onboardingComplete: boolean;
  groqApiKeyConfigured: boolean;
}

export interface ModelInfo { id: string; name: string; detail: string; sizeMb: number; downloaded: boolean; active: boolean; multilingual: boolean; recommended?: boolean; }
export interface HistoryItem { id: string; text: string; rawText: string; timestamp: number; wordCount: number; durationSeconds: number; transcriptionMs: number; model: string; source: "local" | "cloud"; }
export interface DictionaryEntry { id: string; original: string; replacement: string; createdAt: number; }
export interface Stats { totalWords: number; totalSeconds: number; totalSessions: number; averageWpm: number; todayWords: number; weekWords: number; }
export interface AudioDevice { name: string; isDefault: boolean; }
export interface PermissionStatus { microphone: boolean; inputMonitoring: boolean; accessibility: boolean; }
export interface UpdateInfo { currentVersion: string; latestVersion: string; available: boolean; notes: string; downloadUrl: string | null; }
export interface RuntimeSnapshot { config: AppConfig; models: ModelInfo[]; history: HistoryItem[]; dictionary: DictionaryEntry[]; stats: Stats; audioDevices: AudioDevice[]; version: string; }

export const defaultConfig: AppConfig = {
  transcriptionMode: "local", model: "base-en", cloudLanguage: "auto", pushToTalkKey: "Fn / Globe", toggleKey: null, shortcutsPaused: false, dismissedSuggestions: [], closeNoticeSeen: false,
  recordingMode: "push-to-talk", preferredInputDevice: null, fallbackInputDevice: null, inputGain: 1, autoPunctuation: true,
  removeFillers: true, copyToClipboard: false, autoEnhancePrompt: false, customInstructions: "", dictionaryEnabled: true,
  recordingSoundEnabled: true, autoHideWidget: false, showWidget: true, followCursor: true, lockWidgetPosition: false,
  theme: "black", launchAtLogin: false, onboardingComplete: false, groqApiKeyConfigured: false,
};
