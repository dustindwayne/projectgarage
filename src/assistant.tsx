import { createContext, useContext, useState, type ReactNode } from 'react';
import { Sparkles } from 'lucide-react';
import { native } from './api';
import { today, type Snapshot } from './types';
type Provider = 'openai' | 'anthropic' | 'xai';
const names: Record<Provider, string> = {
  openai: 'OpenAI / ChatGPT',
  anthropic: 'Claude',
  xai: 'Grok',
};
type Connection = {
  provider: Provider;
  model: string;
  key: string;
  setProvider: (p: Provider) => void;
  setModel: (m: string) => void;
  setKey: (k: string) => void;
};
const ConnectionContext = createContext<Connection | null>(null);
export function AssistantProvider({ children }: { children: ReactNode }) {
  const [provider, setP] = useState<Provider>(() => {
    const p = localStorage.getItem('pg-ai-provider');
    return p === 'anthropic' || p === 'xai' ? p : 'openai';
  });
  const [model, setM] = useState(() => localStorage.getItem('pg-ai-model') || '');
  const [key, setKey] = useState('');
  return (
    <ConnectionContext.Provider
      value={{
        provider,
        model,
        key,
        setKey,
        setProvider: (p) => {
          setP(p);
          setM('');
          setKey('');
          localStorage.setItem('pg-ai-provider', p);
          localStorage.removeItem('pg-ai-model');
        },
        setModel: (m) => {
          setM(m);
          localStorage.setItem('pg-ai-model', m);
        },
      }}
    >
      {children}
    </ConnectionContext.Provider>
  );
}
function ConnectionFields() {
  const c = useContext(ConnectionContext)!;
  return (
    <div className="assistant-connect">
      <label>
        AI provider
        <select value={c.provider} onChange={(e) => c.setProvider(e.target.value as Provider)}>
          {Object.entries(names).map(([id, name]) => (
            <option key={id} value={id}>
              {name}
            </option>
          ))}
        </select>
      </label>
      <label>
        Model ID
        <input
          value={c.model}
          maxLength={150}
          autoCapitalize="none"
          autoCorrect="off"
          placeholder="Model available in your provider account"
          onChange={(e) => c.setModel(e.target.value)}
        />
      </label>
      <label>
        Provider API key
        <input
          type="password"
          autoComplete="off"
          value={c.key}
          maxLength={512}
          placeholder="Kept only until you close the app"
          onChange={(e) => c.setKey(e.target.value)}
        />
      </label>
      {c.key && (
        <button className="secondary" onClick={() => c.setKey('')}>
          Disconnect and forget key
        </button>
      )}
      <p>
        Use a provider API key. ChatGPT, Claude, and Grok chat subscriptions do not connect here.
        API usage is billed by your provider.
      </p>
    </div>
  );
}
export function AssistantSettings() {
  return (
    <section className="settings-card">
      <h2>
        <Sparkles size={19} />
        AI assistant
      </h2>
      <p>
        Give your assistant full control of your garage records, using your account’s owner or staff
        permissions.
      </p>
      {native ? <ConnectionFields /> : <p>Connect AI in the installed Android or desktop app.</p>}
    </section>
  );
}
export function AssistantPanel({
  snapshot,
  disabled,
  mutate,
}: {
  snapshot: Snapshot;
  disabled: boolean;
  mutate: (action: unknown, revision?: number) => Promise<void>;
}) {
  const c = useContext(ConnectionContext)!;
  const [prompt, setPrompt] = useState(''),
    [running, setRunning] = useState(false),
    [response, setResponse] = useState(''),
    [error, setError] = useState('');
  const [history, setHistory] = useState<{ request: string; result: string }[]>([]);
  const connected = !!c.key.trim() && !!c.model.trim();
  async function run(e: React.FormEvent) {
    e.preventDefault();
    setRunning(true);
    setError('');
    setResponse('');
    const revision = snapshot.revision;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const plan = await invoke<{ message: string; actions: unknown[] }>('ask_garage_ai', {
        provider: c.provider,
        model: c.model,
        key: c.key,
        prompt,
        snapshot,
        date: today(),
      });
      if (plan.actions.length) await mutate({ type: 'batch', actions: plan.actions }, revision);
      const result =
        (plan.actions.length
          ? `Saved ${plan.actions.length} change${plan.actions.length === 1 ? '' : 's'}.\n\n`
          : 'No records changed.\n\n') + plan.message;
      setResponse(result);
      setHistory((h) => [{ request: prompt, result }, ...h].slice(0, 10));
      setPrompt('');
    } catch (e) {
      setError(
        typeof e === 'string'
          ? e
          : e instanceof Error
            ? e.message
            : 'Could not complete your request. Nothing was saved.',
      );
    } finally {
      setRunning(false);
    }
  }
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>
            Garage assistant<span className="heading-dot">.</span>
          </h1>
          <p>Tell it what happened. Let it handle the records.</p>
        </div>
      </div>
      {!native ? (
        <section className="settings-card">
          <p>
            AI is available in the installed app. It connects directly to your provider without a
            local server.
          </p>
        </section>
      ) : (
        <>
          {!connected && (
            <section className="settings-card">
              <h2>Connect your assistant</h2>
              <ConnectionFields />
            </section>
          )}
          <p className="assistant-disclosure">
            Your garage’s text records and this request are sent to {names[c.provider]}. Photos,
            receipt files, and member contact details stay on your device. Requested edits and
            deletions save automatically with your permissions.
          </p>
          <div className="assistant-examples">
            {[
              'What does each car still need?',
              'Record my parts purchase',
              'Add today’s build progress',
            ].map((t) => (
              <button key={t} onClick={() => setPrompt(t)}>
                {t}
              </button>
            ))}
          </div>
          {response && (
            <div className="assistant-response" role="status">
              {response}
            </div>
          )}
          {error && (
            <div className="assistant-response" role="alert">
              {error}
            </div>
          )}
          <form className="assistant-form" onSubmit={run}>
            <label htmlFor="garage-request">What would you like done?</label>
            <textarea
              id="garage-request"
              value={prompt}
              maxLength={12000}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="On the E30, mark the front brakes installed and add a journal entry for today."
              required
              disabled={running}
            />
            <button
              className="primary"
              disabled={disabled || running || !connected || !prompt.trim()}
            >
              <Sparkles size={18} />
              {running ? 'Working on your garage…' : 'Run request'}
            </button>
          </form>
          {history.length > 0 && (
            <details className="assistant-history">
              <summary>Session activity ({history.length})</summary>
              {history.map((h, i) => (
                <div key={i} className="assistant-response">
                  <strong>{h.request}</strong>
                  <p>{h.result}</p>
                </div>
              ))}
            </details>
          )}
        </>
      )}
    </>
  );
}
