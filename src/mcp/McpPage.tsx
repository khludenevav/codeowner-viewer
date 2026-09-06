import { invoke } from '@tauri-apps/api/core';
import { homeDir } from '@tauri-apps/api/path';
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
  const portValid = Number.isInteger(parsedPort) && parsedPort >= 1024 && parsedPort <= 65535;
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
          <p className='text-xs text-destructive'>Port must be between 1024 and 65535.</p>
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
        The server exposes four tools: <code className='rounded bg-muted px-1'>get_codeowners</code>{' '}
        for inline reads (returns a plain-text DSL that encodes ownership as an inheritance tree),{' '}
        <code className='rounded bg-muted px-1'>export_codeowners</code> for scripting (dumps the
        full ownership map to a temp JSON file and returns just the path),{' '}
        <code className='rounded bg-muted px-1'>list_owners</code> for discovering the distinct
        owner handles present in the repo, and{' '}
        <code className='rounded bg-muted px-1'>owners_stats</code> for per-owner file counts + repo
        totals. Use <code>get_codeowners</code> when you want to look at data,{' '}
        <code>export_codeowners</code> when you want to run Python over it, and{' '}
        <code>list_owners</code> / <code>owners_stats</code> to pick a good filter first.
      </p>
      <h3 className='mt-5 text-base font-semibold'>get_codeowners</h3>
      <div className='mt-2 space-y-2 text-sm'>
        <p>
          <strong>Inputs:</strong> <code className='rounded bg-muted px-1'>repo</code> (absolute
          path),{' '}
          <code className='rounded bg-muted px-1'>
            for = &quot;branch&quot; | &quot;changed_files&quot;
          </code>
          , <code className='rounded bg-muted px-1'>branch?</code> (defaults to <code>HEAD</code>),{' '}
          <code className='rounded bg-muted px-1'>paths?</code> (repo-root-relative files,
          directories (recursive), or globs — mixed types OK),{' '}
          <code className='rounded bg-muted px-1'>
            responseMode = &quot;compact&quot; (default) | &quot;full&quot;
          </code>
          , <code className='rounded bg-muted px-1'>maxDepth?</code> (integer; when set, any subtree
          deeper than this many levels below each entry of <code>paths</code> is collapsed. Depth 0
          = the path itself; depth 1 = its immediate children. Ignored when <code>paths</code> is
          empty).
        </p>
        <p>
          When <code>for = &quot;changed_files&quot;</code> the <code>paths</code> list is additive:
          omit it for just the working-tree changed set, or include files/dirs/globs to also resolve
          ownership for those paths (against <code>branch</code>).
        </p>
        <p>
          <strong>Response modes:</strong> <em>compact</em> emits only exceptions —
          directories/files whose rule differs from the enclosing default; <em>full</em> lists every
          file with its rule id.
        </p>
        <p>
          <strong>Size guard.</strong> Responses over 50 KB are pruned by collapsing the heaviest
          subtree into <code>[id:count,…] TRUNCATED</code>; the untruncated body is dumped to an
          app-data file and referenced via <code>fullDumpPath:</code> in the header. Agents can then
          re-request that subtree with a larger <code>maxDepth</code> to see it inline.
        </p>
        <details className='rounded-md border bg-muted/40 px-3 py-2 text-xs'>
          <summary className='cursor-pointer font-medium'>DSL grammar</summary>
          <pre className='mt-2 whitespace-pre-wrap font-mono'>{`format: codeowners-dsl-v1
base: <common dir prefix stripped from every tree path>
default: <rule id that applies to anything not otherwise listed>
truncated: true|false
sizeBytes: <byte length of this body>
fullDumpPath: <path>   # only when truncated=true

rules:
  <line-number> <owner> [<owner> ...] [(<comment>)]
  # First "@..." is the primary owner; extra "@..." tokens on the same
  # line are co-owners on the same CODEOWNERS rule.
  # "(<comment>)" is a VERBATIM passthrough of the trailing "# ..."
  # comment on the CODEOWNERS line (leading "#" stripped). Its contents
  # are NOT part of the DSL grammar — they may hold project conventions
  # like "!required" markers or extra "@owner" notes. Do NOT parse
  # "@owner" tokens inside a comment as additional owners.
  # A bare "<line-number>" (no owners) means unowned.
  # ")" inside comments is escaped as "\\)".

<dir>/[ <rule-id> | [id:count,...] TRUNCATED]
  <file>[ <rule-id>]
  <subdir>/...

Rules:
- Trailing "/" marks a directory.
- A rule id on a directory line sets that subtree's default.
- A file/dir with no rule id inherits the nearest ancestor's default.
- "[id:count,...] TRUNCATED" replaces a subtree collapsed by maxDepth
  or the size guard. Each entry is <rule id>:<file count> for files
  that rule owns inside the collapsed subtree; sum(counts) = total.
  Sorted by descending count, tiebreak ascending id. Marker is only
  emitted for mixed (≥2 rules) subtrees — a single-rule collapse
  renders as "dir/ <rule>" because no info is lost.`}</pre>
        </details>
      </div>

      <h3 className='mt-6 text-base font-semibold'>export_codeowners</h3>
      <div className='mt-2 space-y-2 text-sm'>
        <p>
          Writes a JSON dump of the whole repo's ownership to a file and returns just{' '}
          <code>{'{ path, sizeBytes, fileCount, ruleCount, schema }'}</code>. The agent then reads
          the file with e.g. <code>json.load(open(path))</code> in Python — no third-party libraries
          required. When <code>path</code> is omitted, the dump lands under the OS temp dir and
          files matching <code>export-*.json</code> older than 7 days are pruned on each invocation.
        </p>
        <p>
          <strong>Inputs:</strong> <code className='rounded bg-muted px-1'>repo</code> (absolute
          path), <code className='rounded bg-muted px-1'>owners?</code> (<code>string[]</code>; keep
          files whose owners include any of these — OR within the list),{' '}
          <code className='rounded bg-muted px-1'>extensions?</code> (<code>string[]</code>;
          case-insensitive, leading dot ignored),{' '}
          <code className='rounded bg-muted px-1'>path?</code> (absolute path to write the dump to;
          overwrites any existing file; parent directories are created; defaults to a fresh file
          under the OS temp dir). Filters combine as AND. Always full-repo at HEAD.
        </p>
        <details className='rounded-md border bg-muted/40 px-3 py-2 text-xs'>
          <summary className='cursor-pointer font-medium'>
            Dump file schema (codeowners-export/v1)
          </summary>
          <pre className='mt-2 whitespace-pre-wrap font-mono'>{`{
  "schema": "codeowners-export/v1",
  "rules": {
    "42":  {"owners": ["@fivetran/kepler"], "comment": "!required"},
    "118": {"owners": ["@fivetran/bacon", "@fivetran/korolev"], "comment": null},
    "0":   {"owners": [], "comment": null}
  },
  "files": {
    "app/src/Main.java": 42,
    "app/misc/orphan.txt": 0
  },
  "stats": {
    "total_files": 41203,
    "owned_files": 41198,
    "unowned_files": 5,
    "rule_count": 187,
    "codeowners_file_path": ".github/CODEOWNERS",
    "filtered": false,
    "generated_at": "2026-09-06T18:15:28Z"
  }
}

# Rule keys are 1-based CODEOWNERS line numbers (as strings).
# Rule "0" is a reserved sentinel meaning "no matching CODEOWNERS rule".
#   Its "owners" is []; only present when at least one unowned file
#   survives the filter.
# "files" values are the rule id for that path. Paths are relative to
#   repo root — no base stripping.
# "comment" is a verbatim inline "# ..." comment from CODEOWNERS with
#   the leading "#" and whitespace stripped, or null. May contain
#   project markers like "!required".
# "stats.filtered" is true iff any input filter was non-empty. When
#   filtered, "rules" only holds rules actually referenced.
# "stats.generated_at" is ISO-8601 UTC, second precision, trailing Z.`}</pre>
        </details>
      </div>

      <h3 className='mt-6 text-base font-semibold'>list_owners</h3>
      <div className='mt-2 space-y-2 text-sm'>
        <p>
          Returns the distinct owner handles present in the repo at <code>HEAD</code>,
          alphabetically sorted. Use this to discover the space of valid owner strings before
          calling <code>export_codeowners</code> with an <code>owners[]</code> filter. Unowned files
          do NOT contribute an entry.
        </p>
        <p>
          <strong>Inputs:</strong> <code className='rounded bg-muted px-1'>repo</code> (absolute
          path).
        </p>
        <p>
          <strong>Output:</strong> <code>{'{ "owners": ["@team/a", "@team/b", ...] }'}</code>.
        </p>
      </div>

      <h3 className='mt-6 text-base font-semibold'>owners_stats</h3>
      <div className='mt-2 space-y-2 text-sm'>
        <p>
          Returns per-owner file counts + repo-wide totals at <code>HEAD</code>. Use to rank owners
          by footprint before picking a filter for <code>export_codeowners</code>. A file with N
          co-owners contributes +1 to each of those N owners, so the sum of per-owner{' '}
          <code>files</code> may exceed <code>totalFiles</code>. Unowned files are counted in{' '}
          <code>unownedFiles</code> but not attributed to any owner.
        </p>
        <p>
          <strong>Inputs:</strong> <code className='rounded bg-muted px-1'>repo</code> (absolute
          path).
        </p>
        <details className='rounded-md border bg-muted/40 px-3 py-2 text-xs'>
          <summary className='cursor-pointer font-medium'>Output shape</summary>
          <pre className='mt-2 whitespace-pre-wrap font-mono'>{`{
  "totalFiles": 41203,
  "unownedFiles": 5,
  "owners": {
    "@fivetran/kepler":  {"files": 12034},
    "@fivetran/bacon":   {"files":  8210},
    "@fivetran/korolev": {"files":   410}
  }
}

# Keys of "owners" are owner handles (alphabetically sorted).
# The count is wrapped in a {"files": N} object (not a raw number)
# so future fields can be added without a breaking schema change.`}</pre>
        </details>
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
        One-click install writes an <code>http</code> MCP entry into each agent's own config file. A
        backup is saved before any change.
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

function AgentStatePill({
  state,
}: {
  state: AgentInstallPlan['agent'] extends never ? never : string;
}) {
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
          <Button onClick={onConfirm} variant={mode === 'uninstall' ? 'destructive' : 'default'}>
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
          <Button variant='ghost' size='sm' onClick={() => log.refetch()} disabled={log.isFetching}>
            <RefreshCw className={cn('mr-2 h-4 w-4', log.isFetching && 'animate-spin')} />
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
              <th className='px-3 py-2 font-medium'>Response size</th>
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
                <td className='px-3 py-2 text-xs'>{formatBytes(entry.responseSizeBytes)}</td>
                <td className='px-3 py-2'>
                  <RequestSummary req={entry.request} />
                </td>
              </tr>
            ))}
            {log.data?.entries.length === 0 && (
              <tr>
                <td colSpan={6} className='px-3 py-4 text-center text-sm text-muted-foreground'>
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
                <div className='mb-1 flex items-center justify-between text-xs font-medium text-muted-foreground'>
                  <span>Response</span>
                  {selected.responseSizeBytes !== undefined && (
                    <span>{formatBytes(selected.responseSizeBytes)}</span>
                  )}
                </div>
                <pre className='max-h-[40vh] overflow-auto rounded bg-muted p-3 text-xs whitespace-pre font-mono'>
                  {renderResponse(selected.response)}
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
  const home = useHomeDir();
  const summary = useMemo(() => {
    try {
      const r = req as Record<string, unknown>;
      const pieces: string[] = [];
      if (typeof r?.repo === 'string') pieces.push(shortenHome(r.repo, home));
      if (typeof r?.for === 'string') pieces.push(`for=${r.for}`);
      if (typeof r?.responseMode === 'string') pieces.push(`mode=${r.responseMode}`);
      return pieces.join(' • ');
    } catch {
      return '';
    }
  }, [req, home]);
  return <span className='truncate text-xs'>{summary}</span>;
}

/**
 * One-shot fetch of the user's home directory, cached at module scope so
 * every consumer shares the same promise. Returns `null` until resolved
 * and `null` if the host has no home dir. Used to render `~/foo` instead
 * of a full `/Users/…/foo` path in the Requests table.
 */
let homeDirPromise: Promise<string | null> | null = null;
function useHomeDir(): string | null {
  const [value, setValue] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    if (!homeDirPromise) {
      homeDirPromise = homeDir()
        .then(v => (typeof v === 'string' && v.length > 0 ? stripTrailingSep(v) : null))
        .catch(() => null);
    }
    homeDirPromise.then(v => {
      if (!cancelled) setValue(v);
    });
    return () => {
      cancelled = true;
    };
  }, []);
  return value;
}

function stripTrailingSep(p: string): string {
  return p.replace(/[\\/]+$/, '');
}

/**
 * If `path` lives under the user's home directory, replace that prefix
 * with `~`. Handles both `/` and `\` separators. Returns `path`
 * unchanged when the home dir isn't known yet or when there's no
 * match.
 */
function shortenHome(path: string, home: string | null): string {
  if (!home) return path;
  if (path === home) return '~';
  if (path.startsWith(home + '/')) return '~' + path.slice(home.length);
  if (path.startsWith(home + '\\')) return '~' + path.slice(home.length);
  return path;
}

function formatBytes(n: number | undefined): string {
  if (n === undefined || n === null) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

/**
 * The tool response is a DSL string (or an error object). Show strings
 * verbatim so the ownership tree renders like a file listing; fall back
 * to pretty JSON for anything else.
 */
function renderResponse(response: unknown): string {
  if (typeof response === 'string') return response;
  return JSON.stringify(response, null, 2);
}
