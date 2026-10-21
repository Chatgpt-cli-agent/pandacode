import { call, preview } from './bridge';
import { escape } from './dom';

interface TurnReply {
  answer: string;
  messages: unknown[];
  toolCalls: number;
  finished: boolean;
}
const STORAGE_KEY = 'pandacode.native-harness.preferences.v1';
type Settings = { profile: string; endpoint: string; model: string; projectDir: string; skills: string; mcp: string };
const defaults: Settings = {
  profile: 'local', endpoint: 'http://127.0.0.1:1234/v1/chat/completions',
  model: '', projectDir: '', skills: '', mcp: '[]',
};
function restore(): Settings {
  try { return { ...defaults, ...JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}') } as Settings; }
  catch { return { ...defaults }; }
}
/** Native harness workbench: no external CLI and no secrets in localStorage. */
export function mountNativeHarnessPanel(host: HTMLElement): () => void {
  const previous = restore();
  const section = document.createElement('section');
  section.className = 'connection-card';
  section.id = 'native-harness-panel';
  section.innerHTML = '<h3>Native harness <small>BYOK + MCP + Skills</small></h3>' +
    '<p>Run a model directly through an OpenAI-compatible chat-completions endpoint. External coding CLIs are not required. This is an experimental workbench; it does not yet create an ordinary PandaCode task.</p>' +
    '<form id="native-harness-config">' +
    '<label for="native-profile">Credential profile</label><input id="native-profile" maxlength="80" required value="' + escape(previous.profile) + '">' +
    '<label for="native-endpoint">Chat completions URL</label><input id="native-endpoint" required value="' + escape(previous.endpoint) + '">' +
    '<label for="native-model">Model identifier</label><input id="native-model" required value="' + escape(previous.model) + '">' +
    '<label for="native-project">Project directory (absolute path)</label><input id="native-project" required value="' + escape(previous.projectDir) + '">' +
    '<label for="native-key">API key (stored in OS credential manager)</label><div class="connection-input-row"><input id="native-key" type="password" autocomplete="off" spellcheck="false"><button type="button" class="secondary-button" data-native="save-key">Save key</button><button type="button" class="secondary-button" data-native="delete-key">Remove key</button></div>' +
    '<label for="native-skills">Skill names (comma-separated, from SKILL.md directories)</label><input id="native-skills" value="' + escape(previous.skills) + '">' +
    '<label for="native-mcp">MCP servers (JSON array; stdio command/args or HTTP URL)</label><textarea id="native-mcp" rows="4" spellcheck="false">' + escape(previous.mcp) + '</textarea>' +
    '<div><label><input type="checkbox" id="native-writes"> Allow project file writes</label></div>' +
    '<div><label><input type="checkbox" id="native-commands"> Allow program execution (full local permissions)</label></div>' +
    '<label for="native-prompt">Instruction</label><textarea id="native-prompt" rows="4" required placeholder="Inspect this project and suggest improvements"></textarea>' +
    '<div class="connection-input-row"><button class="primary-button" type="submit">Run native agent</button><button class="secondary-button" type="button" data-native="skills">Find skills</button><button class="secondary-button" type="button" data-native="reset">New conversation</button></div>' +
    '</form><p id="native-status" role="status"></p><pre id="native-result" style="white-space:pre-wrap;overflow-wrap:anywhere;max-height:400px;overflow:auto"></pre>';
  host.append(section);
  let disposed = false, busy = false;
  let history: unknown[] = [];
  const el = (id: string) => section.querySelector<HTMLInputElement | HTMLTextAreaElement>('#' + id)!;
  const status = section.querySelector<HTMLElement>('#native-status')!;
  const result = section.querySelector<HTMLElement>('#native-result')!;
  const savePreferences = (): Settings => {
    const settings = {
      profile: el('native-profile').value.trim(), endpoint: el('native-endpoint').value.trim(),
      model: el('native-model').value.trim(), projectDir: el('native-project').value.trim(),
      skills: el('native-skills').value.trim(), mcp: el('native-mcp').value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    return settings;
  };
  const click = async (e: Event) => {
    const action = (e.target as HTMLElement).closest<HTMLElement>('[data-native]')?.dataset.native;
    if (!action || busy) return;
    if (action === 'reset') { history = []; result.textContent = ''; status.textContent = 'Conversation reset.'; return; }
    if (preview) { status.textContent = 'Open the desktop application to use native agents.'; return; }
    busy = true;
    try {
      const conf = savePreferences();
      if (action === 'save-key') {
        await call('native_store_key', { profile: conf.profile, apiKey: el('native-key').value });
        el('native-key').value = '';
        status.textContent = 'API key saved securely.';
      } else if (action === 'delete-key') {
        await call('native_delete_key', { profile: conf.profile });
        el('native-key').value = '';
        status.textContent = 'Stored key removed.';
      } else if (action === 'skills') {
        const names = await call<string[]>('native_list_skills', { projectDir: conf.projectDir });
        status.textContent = names.length ? 'Available skills: ' + names.join(', ') : 'No SKILL.md skills found.';
      }
    } catch (error) { status.textContent = String(error); }
    finally { busy = false; }
  };
  const submit = async (e: Event) => {
    e.preventDefault();
    if (busy) return;
    if (preview) { status.textContent = 'Desktop app required.'; return; }
    busy = true;
    status.textContent = 'Running agent…';
    try {
      const conf = savePreferences();
      const servers = JSON.parse(conf.mcp);
      if (!Array.isArray(servers)) throw Error('MCP must be an array');
      const request = {
        model: { profile: conf.profile, endpoint: conf.endpoint, model: conf.model, apiKeyRequired: false },
        projectDir: conf.projectDir, prompt: el('native-prompt').value,
        history, skillNames: conf.skills.split(',').map(x => x.trim()).filter(Boolean),
        mcpServers: servers,
        allowWrites: el('native-writes').checked,
        allowCommands: el('native-commands').checked,
      };
      const answer = await call<TurnReply>('native_run_turn', { req: request });
      history = answer.messages.filter((m: unknown) => {
        const role = (m as { role?: string }).role;
        return role && role !== 'system';
      });
      result.textContent = answer.answer;
      status.textContent = answer.finished ? 'Turn complete (' + answer.toolCalls + ' tool calls).' : 'Turn limit reached; continue in a new request.';
    } catch (error) {
      status.textContent = String(error);
    } finally { busy = false; }
  };
  section.addEventListener('click', click);
  section.querySelector('form')!.addEventListener('submit', submit);
  return () => {
    disposed = true; void disposed;
    el('native-key').value = '';
    section.removeEventListener('click', click);
    section.querySelector('form')!.removeEventListener('submit', submit);
    section.remove();
  };
}
