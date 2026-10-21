// Panda's official OpenClaw Gateway client. Runtime logic lives in OpenClaw.
// Install dependencies locally in this directory (npm install), then launch via PandaCode.
// Stdout is JSON-lines events for the Rust host; credentials never travel to the renderer.
import { createInterface } from 'node:readline';
import { randomUUID } from 'node:crypto';
import { GatewayClient } from '@openclaw/gateway-client';
import { PROTOCOL_VERSION } from '@openclaw/gateway-protocol/version';
import { GATEWAY_CLIENT_CAPS } from '@openclaw/gateway-protocol/client-info';

const line = (message) => process.stdout.write(JSON.stringify(message) + '\n');
const input = createInterface({ input: process.stdin, crlfDelay: Infinity });
const first = await new Promise(resolve => input.once('line', resolve));
let spec;
try { spec = JSON.parse(first); }
catch { line({kind:'error',text:'Invalid gateway startup payload'}); process.exit(2); }
const sessionKey = spec.sessionKey;
if (typeof sessionKey !== 'string' || !sessionKey.startsWith('agent:panda:')) {
  line({kind:'error',text:'Invalid isolated Panda session key'}); process.exit(2);
}
const url = spec.url || 'ws://127.0.0.1:18789';
if (!/^wss:\/\//.test(url) && !/^ws:\/\/(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?(?:\/|$)/.test(url)) {
  line({kind:'error',text:'Remote gateways must use wss://'}); process.exit(2);
}
let started = false;
let activeRun = null;
let lastText = '';
let settled = false;
let settle;
const completion = new Promise(resolve => { settle = resolve; });
let readyResolve, readyReject;
const ready = new Promise((resolve,reject)=>{readyResolve=resolve;readyReject=reject;});
const client = new GatewayClient({
  url,
  token: process.env.PANDA_GATEWAY_TOKEN,
  minProtocol: PROTOCOL_VERSION,
  maxProtocol: PROTOCOL_VERSION,
  role: 'operator',
  scopes: ['operator.read','operator.write'],
  caps: [GATEWAY_CLIENT_CAPS.TOOL_EVENTS],
  onHelloOk: ()=>{line({kind:'status',text:'Gateway connected'});readyResolve();},
  onConnectError: error => {
    if (!started) readyReject(error);
    line({kind:'status',text:'Gateway connection interrupted'});
  },
  onEvent: event => {
    const p=event.payload||{};
    if (event.event==='chat' && p.sessionKey===sessionKey) {
      if (activeRun && p.runId && p.runId!==activeRun) return;
      const blocks=Array.isArray(p.message?.content)?p.message.content:[];
      const next = blocks.map(block=>block?.type==='text'?block.text||'':'').join('') ||
        (typeof p.message?.content === 'string' ? p.message.content :
        typeof p.message?.text === 'string' ? p.message.text :
        typeof p.text === 'string' ? p.text : '');
      if (next) {
        const suffix=next.startsWith(lastText)?next.slice(lastText.length):next;
        if (suffix) line({kind:'text',text:suffix,replace:!next.startsWith(lastText)});
        lastText=next;
      }
      if (p.state==='final'||p.state==='done'||p.state==='error'||p.state==='aborted') {
        settled=true;
        settle();
        line({kind:p.state==='error'?'error':'complete',text:lastText, state:p.state});
      }
    }
    if (event.event==='session.tool' && p.sessionKey===sessionKey) {
      line({kind:'tool',data:{runId:p.runId,tool:p.toolName||p.name,phase:p.phase,status:p.status}});
    }
  },
});
try {
  client.start();
  await Promise.race([ready,new Promise((_,reject)=>setTimeout(()=>reject(new Error('Gateway connection timeout')),20000))]);
  started = true;
  const key=randomUUID();
  const receipt=await client.request('chat.send',{sessionKey,message:spec.prompt,idempotencyKey:key});
  activeRun=receipt?.runId || null;
  line({kind:'accepted',runId:activeRun});
  const controls = (async () => { for await (const message of input) {
    let command; try{command=JSON.parse(message);}catch{continue;}
    if (command.kind==='abort') {
      await client.request('chat.abort',{sessionKey,runId:activeRun}).catch(err=>line({kind:'error',text:String(err)}));
      line({kind:'aborted'});
      settle();
      break;
    }
  } })();
  await Promise.race([completion, new Promise((_,reject)=>setTimeout(()=>reject(new Error('Gateway turn timed out')),900000))]);
  void controls;
} catch(error) {
  line({kind:'error',text:error instanceof Error?error.message:String(error)});
  process.exitCode=1;
} finally {client.stop();input.close();}
