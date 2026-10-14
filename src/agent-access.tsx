import { useRef, useState } from 'react';
import { api, native, ApiError } from './api';
import type { Snapshot } from './types';
import { today } from './types';
import { AssistantSettings } from './assistant';
export const AGENT_PROMPT = `Use the Project Garage skill to manage my garage records. Read https://raw.githubusercontent.com/dustindwayne/projectgarage/main/downloads/project-garage-agent-skill.md and follow its workflow. On my desktop, use the verified garage-agent bridge to access the existing garage; do not configure AI providers in the app. If I attach a phone context file, use that data and return an importable changes file instead. Ask what I want done if I have not given a task. Only make the changes I request, preserve receipts and photos, and verify the result.`;
function handoff(snapshot: Snapshot) {
  return {
    protocol: 'projectgarage-agent/1',
    revision: snapshot.revision,
    today: today(),
    user: { name: snapshot.user.name, role: snapshot.user.role },
    garage: {
      ...snapshot.garage,
      cars: snapshot.garage.cars.map(({ cover, ...c }) => ({
        ...c,
        items: c.items.map(({ receipt, ...i }) => i),
        journal: c.journal.map(({ photos, author, ...e }) => e),
      })),
    },
  };
}
export function AgentAccess({
  snapshot,
  disabled,
  onSnapshot,
  onNotice,
  onError,
}: {
  snapshot: Snapshot;
  disabled: boolean;
  onSnapshot: (s: Snapshot) => Promise<void>;
  onNotice: (s: string) => void;
  onError: (s: string) => void;
}) {
  const [changes, setChanges] = useState<{ revision: number; actions: unknown[] } | null>(null),
    [working, setWorking] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  async function share() {
    try {
      const contents = JSON.stringify(handoff(snapshot), null, 2);
      if (native) {
        const { invoke } = await import('@tauri-apps/api/core');
        const path = await invoke<string>('export_agent_context', { contents });
        onNotice(`Agent context ready: ${path}`);
      } else {
        const u = URL.createObjectURL(new Blob([contents], { type: 'application/json' })),
          a = document.createElement('a');
        a.href = u;
        a.download = 'project-garage-agent-context.json';
        a.click();
        setTimeout(() => URL.revokeObjectURL(u), 1000);
      }
    } catch (e) {
      onError(String(e));
    }
  }
  return (
    <section className="settings-card">
      <details className="agent-access">
        <summary>Use your own agent (optional)</summary>
        <p>
          Use ChatGPT, Claude, Grok, or another agent you already work with. No AI account setup
          inside Project Garage.
        </p>
        <button
          className="secondary"
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(AGENT_PROMPT);
              onNotice('Agent prompt copied');
            } catch {
              onError('Clipboard unavailable. Select the prompt below and copy it.');
            }
          }}
        >
          Copy agent prompt
        </button>
        <details>
          <summary>Show prompt</summary>
          <textarea readOnly value={AGENT_PROMPT} aria-label="Agent prompt" />
        </details>
        <p>
          Desktop agents with file and command access can update your local garage directly using
          the skill. For a phone or a chat without desktop access, share the context file and import
          the changes it returns.
        </p>
        <div className="button-row">
          <button className="secondary" disabled={disabled || working} onClick={() => void share()}>
            Share garage context
          </button>
          <button
            className="secondary"
            disabled={disabled || working}
            onClick={() => input.current?.click()}
          >
            Import agent changes
          </button>
        </div>
        <p className="settings-help">
          The context contains your garage’s text records. Photos, receipt files, and member
          contacts are excluded. Share it only with the agent you choose.
        </p>
        <input
          ref={input}
          type="file"
          accept="application/json,.json"
          hidden
          onChange={async (e) => {
            const f = e.target.files?.[0];
            e.target.value = '';
            if (!f) return;
            try {
              if (f.size > 1_000_000) throw Error('Changes file exceeds 1 MB');
              const v = JSON.parse(await f.text());
              if (
                v.protocol !== 'projectgarage-agent/1' ||
                !Number.isSafeInteger(v.revision) ||
                !Array.isArray(v.actions) ||
                v.actions.length < 1 ||
                v.actions.length > 32
              )
                throw Error('Choose a Project Garage agent changes file');
              setChanges(v);
            } catch (e) {
              setChanges(null);
              onError(e instanceof Error ? e.message : String(e));
            }
          }}
        />
        {changes && (
          <div className="agent-import">
            <h3>{changes.actions.length} requested changes</h3>
            <ul>
              {changes.actions.map((a: any, i) => (
                <li key={i}>
                  {String(a.type || 'Unknown change').replaceAll('_', ' ')}
                  {a.car?.name
                    ? ` — ${a.car.name}`
                    : a.item?.name
                      ? ` — ${a.item.name}`
                      : a.entry?.title
                        ? ` — ${a.entry.title}`
                        : ''}
                </li>
              ))}
            </ul>
            <button
              className="primary"
              disabled={disabled || working}
              onClick={async () => {
                setWorking(true);
                try {
                  const s = await api<Snapshot>('agent/action', changes);
                  await onSnapshot(s);
                  onNotice('Agent changes saved');
                  setChanges(null);
                } catch (e) {
                  if (e instanceof ApiError && e.status === 409) {
                    await onSnapshot(await api<Snapshot>('garage'));
                    onError(
                      'Your garage changed since this context was shared. Share a fresh context and have the agent update its changes.',
                    );
                  } else onError(e instanceof Error ? e.message : String(e));
                } finally {
                  setWorking(false);
                }
              }}
            >
              Apply changes
            </button>
            <button className="secondary" disabled={working} onClick={() => setChanges(null)}>
              Cancel
            </button>
          </div>
        )}
        <AssistantSettings />
      </details>
    </section>
  );
}
