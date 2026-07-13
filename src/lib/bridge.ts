import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppConfig, DictionaryEntry, HistoryItem, ModelInfo, PermissionStatus, RecordingState, RuntimeSnapshot, UpdateInfo } from "../types";
import { defaultConfig } from "../types";

const inTauri = () => "__TAURI_INTERNALS__" in window;
const now = Date.now();
let webSnapshot: RuntimeSnapshot = {
  config: { ...defaultConfig, onboardingComplete: true },
  models: [
    { id:"tiny-en", name:"Tiny (English)", detail:"Fastest · Basic accuracy", sizeMb:75, downloaded:false, active:false, multilingual:false },
    { id:"base-en", name:"Base (English)", detail:"Fast · Good accuracy", sizeMb:142, downloaded:true, active:true, multilingual:false, recommended:true },
    { id:"small-en", name:"Small (English)", detail:"Moderate · Better accuracy", sizeMb:466, downloaded:false, active:false, multilingual:false },
    { id:"medium-en", name:"Medium (English)", detail:"Slower · Great accuracy", sizeMb:1500, downloaded:false, active:false, multilingual:false },
    { id:"large-v3", name:"Large v3", detail:"Best accuracy · Multilingual", sizeMb:3100, downloaded:false, active:false, multilingual:true },
    { id:"distil-large-v3", name:"Distil Large v3", detail:"Fast · Great accuracy", sizeMb:1500, downloaded:false, active:false, multilingual:true },
    { id:"parakeet-v3", name:"Parakeet V3", detail:"Fast multilingual · 25 languages", sizeMb:640, downloaded:false, active:false, multilingual:true },
  ],
  history: [
    { id:"demo-1", text:"Refactor the authentication middleware and add tests for expired refresh tokens.", rawText:"Refactor the authentication middleware and add tests for expired refresh tokens.", timestamp:now-52*60_000, wordCount:11, durationSeconds:5.8, transcriptionMs:412, model:"Base (English)", source:"local" },
    { id:"demo-2", text:"Ship the landing page with responsive states and accessible keyboard navigation.", rawText:"Ship the landing page with responsive states and accessible keyboard navigation.", timestamp:now-26*3600_000, wordCount:10, durationSeconds:4.9, transcriptionMs:376, model:"Base (English)", source:"local" },
  ],
  dictionary: [
    { id:"dict-1", original:"next js", replacement:"Next.js", createdAt:now-86400_000 },
    { id:"dict-2", original:"typescript", replacement:"TypeScript", createdAt:now-86400_000 },
    { id:"dict-3", original:"bridge mind", replacement:"BridgeMind", createdAt:now-86400_000 },
  ],
  stats:{ totalWords:21,totalSeconds:10.7,totalSessions:2,averageWpm:118,todayWords:11,weekWords:21 },
  audioDevices:[{name:"MacBook Pro Microphone",isDefault:true}], version:"0.1.0",
};

export async function snapshot(): Promise<RuntimeSnapshot> { return inTauri() ? invoke("get_snapshot") : structuredClone(webSnapshot); }
export async function updateConfig(patch: Partial<AppConfig>): Promise<AppConfig> {
  if (inTauri()) return invoke("update_config", { patch });
  webSnapshot.config = { ...webSnapshot.config, ...patch }; return webSnapshot.config;
}
export async function startRecording(): Promise<void> { if(inTauri()) await invoke("start_recording"); else window.dispatchEvent(new CustomEvent("bv-recording-state",{detail:"listening"})); }
export async function stopRecording(): Promise<HistoryItem | null> { if(inTauri()) return invoke("stop_recording"); window.dispatchEvent(new CustomEvent("bv-recording-state",{detail:"processing"})); setTimeout(()=>window.dispatchEvent(new CustomEvent("bv-recording-state",{detail:"success"})),700); return null; }
export async function cancelRecording(): Promise<void> { if(inTauri()) await invoke("cancel_recording"); }
export async function selectModel(modelId:string): Promise<void> { if(inTauri()) await invoke("select_model",{modelId}); else { webSnapshot.models.forEach(m=>m.active=m.id===modelId); webSnapshot.config.model=modelId; } }
export async function downloadModel(modelId:string): Promise<void> { if(inTauri()) await invoke("download_model",{modelId}); else { const m=webSnapshot.models.find(x=>x.id===modelId); if(m)m.downloaded=true; } }
export async function deleteModel(modelId:string): Promise<void> { if(inTauri()) await invoke("delete_model",{modelId}); else { const m=webSnapshot.models.find(x=>x.id===modelId); if(m)m.downloaded=false; } }
export async function deleteHistory(id:string): Promise<void> { if(inTauri()) await invoke("delete_history",{id}); else webSnapshot.history=webSnapshot.history.filter(x=>x.id!==id); }
export async function clearHistory(): Promise<void> { if(inTauri()) await invoke("clear_history"); else webSnapshot.history=[]; }
export async function exportHistory(): Promise<string> { return inTauri()?invoke("export_history"):JSON.stringify(webSnapshot.history,null,2); }
export async function addDictionary(original:string,replacement:string): Promise<DictionaryEntry> { if(inTauri())return invoke("upsert_dictionary",{original,replacement}); const item={id:crypto.randomUUID(),original,replacement,createdAt:Date.now()}; webSnapshot.dictionary.push(item);return item; }
export async function deleteDictionary(id:string): Promise<void> { if(inTauri())await invoke("delete_dictionary",{id});else webSnapshot.dictionary=webSnapshot.dictionary.filter(x=>x.id!==id); }
export async function copyText(text:string): Promise<void> { if(inTauri()) await invoke("copy_text",{text}); else await navigator.clipboard.writeText(text); }
export async function polishText(id:string,mode:"polish"|"enhance"): Promise<HistoryItem> { return inTauri()?invoke("polish_transcription",{id,mode}):webSnapshot.history.find(x=>x.id===id)!; }
export async function setGroqKey(key:string): Promise<void> { if(inTauri()) await invoke("set_groq_api_key",{key}); webSnapshot.config.groqApiKeyConfigured=!!key; }
export async function checkPermissions(): Promise<PermissionStatus> { return inTauri()?invoke("check_permissions"):{microphone:true,inputMonitoring:true,accessibility:true}; }
export async function requestPermissions(): Promise<PermissionStatus> { return inTauri()?invoke("request_permissions"):{microphone:true,inputMonitoring:true,accessibility:true}; }
export async function checkForUpdates(): Promise<UpdateInfo> { return inTauri()?invoke("check_for_updates"):{currentVersion:"0.1.0",latestVersion:"0.1.0",available:false,notes:"Browser preview does not use a release feed.",downloadUrl:null}; }
export async function showDashboard(): Promise<void> { if(inTauri()) await invoke("show_dashboard"); }
export async function acknowledgeCloseNotice(): Promise<void> { if(inTauri()) await invoke("acknowledge_close_notice"); }
export async function quitApp(): Promise<void> { if(inTauri()) await invoke("quit_app"); }
export async function openExternal(url:string): Promise<void> { if(inTauri()) await openUrl(url); else window.open(url,"_blank","noopener,noreferrer"); }
export async function listenEvent<T>(name:string,handler:(payload:T)=>void):Promise<UnlistenFn>{
  if(inTauri()) return listen<T>(name,e=>handler(e.payload));
  const fn=(e:Event)=>handler((e as CustomEvent<T>).detail); window.addEventListener(name,fn); return ()=>window.removeEventListener(name,fn);
}
export const recordingEvents=(handler:(state:RecordingState)=>void)=>listenEvent<RecordingState>("recording-state",handler);
