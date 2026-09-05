import { invoke } from '@tauri-apps/api/core';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { Circle, Copy, Plug, RefreshCw, Trash2 } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { useAppConfig, useUpdateAppConfig } from '@/app-config/useAppConfig';
import { DEFAULT_MCP_PORT } from '@/app-config/app-config';
import { cn } from '@/utils/components-utils';
import {
  AGENT_ORDER,
  AgentInstallPlan,
  AgentKind,
  useAgentInstall,
  useAgentStatus,
  useAgentUninstall,
  useMcpLog,
  useMcpLogClear,
  useMcpRestart,
  useMcpStart,
  useMcpStatus,
  useMcpStop,
  McpLogEntry,
} from './useMcp';

export function McpPage() {
  return (
    <div className='mx-auto flex max-w-5xl flex-col gap-8 px-6 py-8'>
      <header className='flex items-center gap-3'>
        <Plug className='h-6 w-6' />
        <div>
          <h1 className='text-2xl font-semibold'>MCP server</h1>
          <p className='text-sm text-muted-foreground'>
            Expose codeowners-viewer to your local coding agents over Model Context Protocol.
          </p>
        </div>
      </header>

      <ServerSection />
      <ToolDescriptionSection />
      <AgentsSection />
      <LogSection />
    </div>
  );
}

// ---------- Server status ---------------------------------------------

function ServerSection() {
  const status = useMcpStatus();
  const start = useMcpStart();
  const stop = useMcpStop();
  const restart = useMcpRestart();
  const appConfig = useAppConfig();
  const updateAppConfig = useUpdateAppConfig();

  const configuredPort = appConfig.data?.mcp?.port ?? DEFAULT_MCP_PORT;
  const port = status.data?.port ?? configuredPort;
  const url = status.data?.url ?? `http://127.0.0.1:${port}/mcp`;
  const running = !!status.data?.running;
  const error = status.data?.error ?? null;

  const enabled = appConfig.data?.mcp?.enabled ?? true;

  const [portInput, setPortInput] = useState<string>(String(configuredPort));
  useEffect(() => {
    setPortInput(String(configuredPort));
  }, [configuredPort]);
  const parsedPort = Number(portInput);
  const portValid =
    Number.isInteger(parsedPort) && parsedPort >= 1024 && parsedPort <= 65535;
  const portDirty = parsedPort !== configuredPort;

  const setEnabled = async (next: boolean) => {
    if (!appConfig.data) return;
    await updateAppConfig.mutateAsync({
      appConfig: {
        ...appConfig.data,
        mcp: { ...(appConfig.data.mcp ?? { port, enabled: true }), enabled: next },
      },
    });
    if (next) {
      start.mutate(port);
    } else {
      stop.mutate();
    }
  };

  const applyPort = async () => {
    if (!appConfig.data || !portValid || !portDirty) return;
    await updateAppConfig.mutateAsync({
      appConfig: {
        ...appConfig.data,
        mcp: {
          enabled: appConfig.data.mcp?.enabled ?? true,
          port: parsedPort,
        },
      },
    });
    restart.mutate(parsedPort);
  };

  const copyUrl = async () => {
    await navigator.clipboard.writeText(url);
    toast.success('URL copied');
  };

  return (
    <section className='rounded-lg border p-5'>
      <div className='flex flex-wrap items-center justify-between gap-3'>
        <div>
          <h2 className='text-lg font-semibold'>Server</h2>
          <p className='text-sm text-muted-foreground'>
            Local Streamable-HTTP MCP endpoint bound to <code>127.0.0.1</code>.
          </p>
        </div>
        <StatusPill running={running} error={error} enabled={enabled} />
      </div>
      <div className='mt-5 grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto_auto]'>
        <div className='flex flex-col gap-1'>
          <label className='text-xs font-medium text-muted-foreground'>URL</label>
          <div className='flex items-center gap-2'>
            <code className='truncate rounded-md bg-muted px-2 py-1 text-sm'>{url}</code>
            <Button variant='outline' size='icon' onClick={copyUrl} aria-label='Copy URL'>
              <Copy className='h-4 w-4' />
            </Button>
          </div>
        </div>
        <div className='flex items-end'>
          <Button
            variant={enabled ? 'destructive' : 'default'}
            onClick={() => setEnabled(!enabled)}
            disabled={updateAppConfig.isPending}
          >
            {enabled ? 'Disable server' : 'Enable server'}
          </Button>
        </div>
        <div className='flex items-end'>
          <Button
            variant='secondary'
            onClick={() => restart.mutate(port)}
            disabled={!enabled || restart.isPending}
          >
            <RefreshCw className={cn('mr-2 h-4 w-4', restart.isPending && 'animate-spin')} />
            Restart
          </Button>
        </div>
      </div>
      <div className='mt-5 flex flex-col gap-1'>
        <label htmlFor='mcp-port' className='text-xs font-medium text-muted-foreground'>
          Port
        </label>
        <div className='flex items-center gap-2'>
          <Input
            id='mcp-port'
            type='number'
            min={1024}
            max={65535}
            value={portInput}
            onChange={e => setPortInput(e.target.value)}
            className='w-32'
          />
          <Button
            size='sm'
            onClick={applyPort}
            disabled={!portValid || !portDirty || updateAppConfig.isPending}
          >
            Apply
          </Button>
          {portDirty && portValid && (
            <span className='text-xs text-muted-foreground'>
              Server will restart on the new port.
            </span>
          )}
        </div>
        {!portValid && (
          <p className='text-xs text-destructive'>
            Port must be between 1024 and 65535.
          </p>
        )}
      </div>
      {error && (
        <p className='mt-3 rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive'>
          {error}
        </p>
      )}
    </section>
  );
}

function StatusPill({
  running,
  error,
  enabled,
}: {
  running: boolean;
  error: string | null;
  enabled: boolean;
}) {
  const [label, color] = useMemo(() => {
    if (error) return ['Error', 'bg-destructive text-destructive-foreground'] as const;
    if (running) return ['Running', 'bg-green-600 text-white'] as const;
    if (!enabled) return ['Disabled', 'bg-muted text-muted-foreground'] as const;
    return ['Stopped', 'bg-muted text-muted-foreground'] as const;
  }, [running, error, enabled]);
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 rounded-full px-3 py-1 text-xs font-medium',
        color,
      )}
    >
      <Circle className='h-2 w-2 fill-current' />
      {label}
    </span>
  );
}

// ---------- What the MCP exposes --------------------------------------

function ToolDescriptionSection() {
  return (
    <section className='rounded-lg border p-5'>
      <h2 className='text-lg font-semibold'>What agents can call</h2>
      <p className='mt-2 text-sm text-muted-foreground'>
        The server exposes a single tool <code className='rounded bg-muted px-1'>get_codeowners</code>.
      </p>
      <div className='mt-3 space-y-2 text-sm'>
        <p>
          <strong>Inputs:</strong> <code className='rounded bg-muted px-1'>repo</code> (absolute
          path),{' '}
          <code className='rounded bg-muted px-1'>
            for = "branch" | "changed_files"
          </code>
          , <code className='rounded bg-muted px-1'>branch?</code> (defaults to{' '}
          <code>HEAD</code>), <code className='rounded bg-muted px-1'>paths?</code>{' '}
          (repo-root-relative files, directories (recursive), or globs — mixed types OK),{' '}
          <code className='rounded bg-muted px-1'>
            responseMode = "compact" | "normal" | "full"
          </code>
          .
        </p>
        <p>
          When <code>for = "changed_files"</code> the <code>paths</code> list is additive: omit it
          for just the working-tree changed set, or include files/dirs/globs to also resolve
          ownership for those paths (against <code>branch</code>).
        </p>
        <p>
          <strong>Response modes:</strong> <em>compact</em> groups by owner-set and collapses
          fully-covered directories; <em>normal</em> returns one entry per file; <em>full</em>{' '}
          also includes the matching CODEOWNERS line number.
        </p>
      </div>
    </section>
  );
}

// ---------- Install into agents ---------------------------------------

function AgentsSection() {
  return (
    <section className='rounded-lg border p-5'>
      <h2 className='text-lg font-semibold'>Install into agents</h2>
      <p className='mt-1 text-sm text-muted-foreground'>
        One-click install writes an <code>http</code> MCP entry into each agent's own config file.
        A backup is saved before any change.
      </p>
      <div className='mt-4 flex flex-col divide-y rounded-md border'>
        {AGENT_ORDER.map(agent => (
          <AgentRow key={agent} agent={agent} />
        ))}
      </div>
    </section>
  );
}

function AgentRow({ agent }: { agent: AgentKind }) {
  const status = useAgentStatus(agent);
  const install = useAgentInstall();
  const uninstall = useAgentUninstall();
  const [confirmPlan, setConfirmPlan] = useState<AgentInstallPlan | null>(null);
  const [confirmMode, setConfirmMode] = useState<'install' | 'uninstall'>('install');

  if (!status.data) {
    return <div className='p-3 text-sm text-muted-foreground'>Loading {agent}…</div>;
  }
  const s = status.data;
  const isUnsupported = s.state === 'unsupported';

  return (
    <>
      <div className='flex flex-wrap items-center justify-between gap-3 p-3'>
        <div className='min-w-0'>
          <div className='flex items-center gap-2'>
            <span className='font-medium'>{s.displayName}</span>
            <AgentStatePill state={s.state} />
          </div>
          <p className='truncate text-xs text-muted-foreground'>{s.configPath || '—'}</p>
          {s.existingUrl && s.state === 'conflict' && (
            <p className='mt-1 text-xs text-yellow-700 dark:text-yellow-400'>
              Existing entry points at <code>{s.existingUrl}</code>
            </p>
          )}
        </div>
        <div className='flex items-center gap-2'>
          {s.state === 'installed' ? (
            <Button
              variant='ghost'
              size='sm'
              disabled={uninstall.isPending}
              onClick={async () => {
                const plan = await invoke<AgentInstallPlan>('agent_install_preview', {
                  agent,
                });
                setConfirmMode('uninstall');
                setConfirmPlan(plan);
              }}
            >
              Uninstall…
            </Button>
          ) : null}
          <Button
            size='sm'
            disabled={install.isPending || isUnsupported}
            onClick={async () => {
              const plan = await invoke<AgentInstallPlan>('agent_install_preview', {
                agent,
              });
              setConfirmMode('install');
              setConfirmPlan(plan);
            }}
          >
            {s.state === 'installed' ? 'Reinstall…' : 'Install…'}
          </Button>
        </div>
      </div>

      <InstallConfirmDialog
        open={confirmPlan !== null}
        plan={confirmPlan}
        mode={confirmMode}
        onClose={() => setConfirmPlan(null)}
        onConfirm={async () => {
          if (!confirmPlan) return;
          try {
            if (confirmMode === 'install') {
              await install.mutateAsync(confirmPlan.agent);
              toast.success(`Installed for ${confirmPlan.displayName}`);
            } else {
              await uninstall.mutateAsync(confirmPlan.agent);
              toast.success(`Uninstalled from ${confirmPlan.displayName}`);
            }
          } catch (e) {
            toast.error(`${confirmMode === 'install' ? 'Install' : 'Uninstall'} failed: ${e}`);
          } finally {
            setConfirmPlan(null);
          }
        }}
      />
    </>
  );
}

function AgentStatePill({ state }: { state: AgentInstallPlan['agent'] extends never ? never : string }) {
  const map: Record<string, [string, string]> = {
    installed: ['Installed', 'bg-green-600 text-white'],
    notInstalled: ['Not installed', 'bg-muted text-muted-foreground'],
    conflict: ['Conflict', 'bg-yellow-500 text-black'],
    unsupported: ['Unsupported', 'bg-muted text-muted-foreground'],
  };
  const [label, cls] = map[state as string] ?? ['?', 'bg-muted text-muted-foreground'];
  return (
    <span className={cn('rounded-full px-2 py-0.5 text-[10px] font-medium uppercase', cls)}>
      {label}
    </span>
  );
}

function InstallConfirmDialog({
  open,
  plan,
  mode,
  onClose,
  onConfirm,
}: {
  open: boolean;
  plan: AgentInstallPlan | null;
  mode: 'install' | 'uninstall';
  onClose: () => void;
  onConfirm: () => void;
}) {
  if (!plan) return null;
  return (
    <Dialog open={open} onOpenChange={o => (!o ? onClose() : undefined)}>
      <DialogContent className='max-w-2xl'>
        <DialogHeader>
          <DialogTitle>
            {mode === 'install' ? 'Install into ' : 'Uninstall from '}
            {plan.displayName}
          </DialogTitle>
        </DialogHeader>
        <div className='space-y-3 text-sm'>
          <div>
            <div className='text-xs font-medium text-muted-foreground'>File</div>
            <code className='block truncate rounded bg-muted px-2 py-1'>{plan.configPath}</code>
          </div>
          <div>
            <div className='text-xs font-medium text-muted-foreground'>Section</div>
            <code className='block rounded bg-muted px-2 py-1'>{plan.section}</code>
          </div>
          {mode === 'install' && (
            <div>
              <div className='text-xs font-medium text-muted-foreground'>
                Will be written ({plan.language})
              </div>
              <pre className='max-h-64 overflow-auto rounded bg-muted p-3 text-xs'>
                {plan.snippet}
              </pre>
            </div>
          )}
          {plan.existingUrl && (
            <div className='rounded-md bg-yellow-50 p-2 text-xs text-yellow-900 dark:bg-yellow-900/20 dark:text-yellow-100'>
              An existing entry points at <code>{plan.existingUrl}</code>. It will be replaced.
            </div>
          )}
          {plan.willBackup && (
            <p className='text-xs text-muted-foreground'>
              A backup will be saved as <code>{`${plan.configPath}.bak-<timestamp>`}</code>.
            </p>
          )}
        </div>
        <DialogFooter>
          <Button variant='outline' onClick={onClose}>
            Cancel
          </Button>
          <Button
            onClick={onConfirm}
            variant={mode === 'uninstall' ? 'destructive' : 'default'}
          >
            {mode === 'install' ? 'Install' : 'Uninstall'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------- Log viewer ------------------------------------------------

function LogSection() {
  const log = useMcpLog(200);
  const clear = useMcpLogClear();
  const [selected, setSelected] = useState<McpLogEntry | null>(null);

  return (
    <section className='rounded-lg border p-5'>
      <div className='mb-3 flex items-center justify-between'>
        <div>
          <h2 className='text-lg font-semibold'>Requests</h2>
          <p className='text-xs text-muted-foreground'>
            Kept for {log.data?.retentionDays ?? 7} days.
          </p>
        </div>
        <div className='flex items-center gap-2'>
          <Button
            variant='ghost'
            size='sm'
            onClick={() => log.refetch()}
            disabled={log.isFetching}
          >
            <RefreshCw
              className={cn('mr-2 h-4 w-4', log.isFetching && 'animate-spin')}
            />
            Refresh
          </Button>
          <Button
            variant='ghost'
            size='sm'
            onClick={() => clear.mutate()}
            disabled={clear.isPending}
          >
            <Trash2 className='mr-2 h-4 w-4' />
            Clear
          </Button>
        </div>
      </div>
      <div className='overflow-hidden rounded-md border'>
        <table className='w-full text-sm'>
          <thead className='bg-muted'>
            <tr className='text-left'>
              <th className='px-3 py-2 font-medium'>Time</th>
              <th className='px-3 py-2 font-medium'>Tool</th>
              <th className='px-3 py-2 font-medium'>Status</th>
              <th className='px-3 py-2 font-medium'>Duration</th>
              <th className='px-3 py-2 font-medium'>Request</th>
            </tr>
          </thead>
          <tbody>
            {(log.data?.entries ?? []).map(entry => (
              <tr
                key={entry.id}
                className='cursor-pointer border-t hover:bg-accent'
                onClick={() => setSelected(entry)}
              >
                <td className='px-3 py-2 text-xs'>{new Date(entry.ts).toLocaleString()}</td>
                <td className='px-3 py-2'>
                  <code className='rounded bg-muted px-1 text-xs'>{entry.tool}</code>
                </td>
                <td className='px-3 py-2'>
                  <span
                    className={cn(
                      'rounded-full px-2 py-0.5 text-[10px] font-medium uppercase',
                      entry.status === 'ok'
                        ? 'bg-green-600 text-white'
                        : 'bg-destructive text-destructive-foreground',
                    )}
                  >
                    {entry.status}
                  </span>
                </td>
                <td className='px-3 py-2 text-xs'>{entry.durationMs} ms</td>
                <td className='px-3 py-2'>
                  <RequestSummary req={entry.request} />
                </td>
              </tr>
            ))}
            {log.data?.entries.length === 0 && (
              <tr>
                <td colSpan={5} className='px-3 py-4 text-center text-sm text-muted-foreground'>
                  No requests yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <Dialog open={selected !== null} onOpenChange={o => (!o ? setSelected(null) : undefined)}>
        <DialogContent className='max-w-5xl w-[calc(100vw-2rem)] max-h-[85vh] flex flex-col gap-4'>
          <DialogHeader>
            <DialogTitle>{selected?.tool}</DialogTitle>
          </DialogHeader>
          {selected && (
            <div className='flex-1 overflow-y-auto flex flex-col gap-4 -mx-6 px-6'>
              <div>
                <div className='mb-1 text-xs font-medium text-muted-foreground'>Request</div>
                <pre className='max-h-[40vh] overflow-auto rounded bg-muted p-3 text-xs whitespace-pre-wrap break-all'>
                  {JSON.stringify(selected.request, null, 2)}
                </pre>
              </div>
              <div>
                <div className='mb-1 text-xs font-medium text-muted-foreground'>Response</div>
                <pre className='max-h-[40vh] overflow-auto rounded bg-muted p-3 text-xs whitespace-pre-wrap break-all'>
                  {JSON.stringify(selected.response, null, 2)}
                </pre>
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant='outline' onClick={() => setSelected(null)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function RequestSummary({ req }: { req: unknown }) {
  const summary = useMemo(() => {
    try {
      const r = req as Record<string, unknown>;
      const pieces: string[] = [];
      if (typeof r?.repo === 'string') pieces.push(String(r.repo));
      if (typeof r?.for === 'string') pieces.push(`for=${r.for}`);
      if (typeof r?.responseMode === 'string') pieces.push(`mode=${r.responseMode}`);
      return pieces.join(' • ');
    } catch {
      return '';
    }
  }, [req]);
  return <span className='truncate text-xs'>{summary}</span>;
}
