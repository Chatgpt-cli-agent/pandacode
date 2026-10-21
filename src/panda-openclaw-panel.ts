import { call, preview } from './bridge';
import { escape } from './dom';
type Reply={text:string;raw:unknown;profile:string};
const STORE='panda.openclaw.workbench.v1';
interface Pref { projectDir:string; model:string; sessionKey:string; direct:boolean }
function restore():Pref {
  try{return {projectDir:'',model:'',sessionKey:'panda-main',direct:false,...JSON.parse(localStorage.getItem(STORE)||'{}')}}
  catch{return {projectDir:'',model:'',sessionKey:'panda-main',direct:false}}
}
export function mountPandaOpenClaw(host:HTMLElement):()=>void {
  const saved=restore();
  const section=document.createElement('section');
  section.className='connection-card';
  section.innerHTML='<h3>Panda <small>OpenClaw agent engine</small></h3>'+
    '<p>Panda uses its own OpenClaw profile, memory and credentials. Hermes/Koda is not used. This initial bridge executes an OpenClaw turn; conversation streaming and task-board integration are still in development.</p>'+
    '<form data-panda-form>'+
    '<label for="panda-project">Project folder (absolute path)</label><input id="panda-project" required value="'+escape(saved.projectDir)+'">'+
    '<label for="panda-model">Model override (optional; provider/model)</label><input id="panda-model" value="'+escape(saved.model)+'">'+
    '<label for="panda-session">Panda session key</label><input id="panda-session" required value="'+escape(saved.sessionKey)+'">'+
    '<label><input id="panda-direct" type="checkbox" '+(saved.direct?'checked':'')+'> Use embedded OpenClaw runtime (skip Gateway)</label>'+
    '<label for="panda-prompt">Tell Panda what to do</label><textarea id="panda-prompt" rows="4" required></textarea>'+
    '<div class="connection-input-row"><button type="button" data-panda-check class="secondary-button">Check OpenClaw</button><button type="submit" class="primary-button">Run Panda</button></div>'+
    '</form><p data-panda-status role="status"></p><pre data-panda-output style="white-space:pre-wrap;overflow-wrap:anywhere;max-height:400px;overflow:auto"></pre>';
  host.appendChild(section);
  const input=(id:string)=>section.querySelector<HTMLInputElement|HTMLTextAreaElement>('#'+id)!;
  const status=section.querySelector<HTMLElement>('[data-panda-status]')!;
  const result=section.querySelector<HTMLElement>('[data-panda-output]')!;
  let busy=false;
  const check=async()=>{
    if(busy)return;
    if(preview){status.textContent='Open the installed PandaCode desktop app.';return;}
    busy=true;
    try{const version=await call<string>('panda_openclaw_check');status.textContent='OpenClaw available: '+version}
    catch(error){status.textContent=String(error)}
    finally{busy=false}
  };
  const submit=async(event:Event)=>{
    event.preventDefault();
    if(busy)return;
    if(preview){status.textContent='Desktop application required.';return;}
    busy=true; status.textContent='Panda is working…';
    const state:Pref={
      projectDir:input('panda-project').value.trim(),
      model:input('panda-model').value.trim(),
      sessionKey:input('panda-session').value.trim(),
      direct:(input('panda-direct') as HTMLInputElement).checked,
    };
    try{
      localStorage.setItem(STORE,JSON.stringify(state));
      const reply=await call<Reply>('panda_openclaw_turn',{req:{
        projectDir:state.projectDir,prompt:input('panda-prompt').value,
        model:state.model||null,sessionKey:state.sessionKey,direct:state.direct,
      }});
      result.textContent=reply.text||JSON.stringify(reply.raw,null,2);
      status.textContent='OpenClaw turn complete using Panda profile.';
    }catch(error){status.textContent=String(error)}
    finally{busy=false}
  };
  section.querySelector<HTMLButtonElement>('[data-panda-check]')!.addEventListener('click',check);
  section.querySelector('form')!.addEventListener('submit',submit);
  return ()=>{section.remove()};
}
