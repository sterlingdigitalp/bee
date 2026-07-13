import { useEffect, useState } from "react";
import { Check, Settings, X } from "lucide-react";
import * as bridge from "./lib/bridge";
import type { RecordingState } from "./types";
import BeeMark from "./BeeMark";

export default function Widget(){
  const [state,setState]=useState<RecordingState>("idle"); const [levels,setLevels]=useState([.16,.28,.42,.72,.47,.3,.18]); const [locked,setLocked]=useState(false); const [shortcut,setShortcut]=useState("fn");
  useEffect(()=>{const refresh=()=>bridge.snapshot().then(data=>{setLocked(data.config.lockWidgetPosition);setShortcut(data.config.pushToTalkKey.includes("Fn")?"fn":data.config.pushToTalkKey.replaceAll(" ",""))});refresh();const unlisten=bridge.recordingEvents(setState);const unlevel=bridge.listenEvent<number[]>("audio-levels",setLevels);const unconfig=bridge.listenEvent("config-changed",refresh);return()=>{unlisten.then(fn=>fn());unlevel.then(fn=>fn());unconfig.then(fn=>fn())}},[]);
  const toggle=async()=>{try{if(state==="idle"||state==="success"||state==="error")await bridge.startRecording();else if(state==="listening")await bridge.stopRecording()}catch(error){console.error(error)}};
  return <div className={`widget ${state}`} onDoubleClick={toggle} {...(!locked?{"data-tauri-drag-region":true}:{})}>
    <button className="widget-logo" onClick={toggle} aria-label={state==="listening"?"Stop recording":"Start recording"}><BeeMark/></button>
    {state==="idle"&&<><span className="widget-title">Bee</span><kbd>{shortcut}</kbd></>}
    {state==="listening"&&<><div className="widget-wave" aria-label="Listening">{levels.slice(0,7).map((v,i)=><i key={i} style={{height:`${Math.max(5,v*30)}px`}}/>)}</div><span className="widget-time">Listening</span><button className="widget-action" onClick={()=>bridge.cancelRecording()} aria-label="Cancel"><X size={13}/></button></>}
    {state==="processing"&&<><span className="widget-spinner"/><span className="widget-title">Transcribing…</span></>}
    {state==="success"&&<><span className="widget-success"><Check size={13}/></span><span className="widget-title">Text inserted</span></>}
    {state==="error"&&<><span className="widget-error">!</span><span className="widget-title">Check Bee</span></>}
    <button className="widget-settings" onClick={()=>bridge.showDashboard()} aria-label="Open dashboard"><Settings size={13}/></button>
  </div>
}
