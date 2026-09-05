import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

export type McpStatus = {
  running: boolean;
  port: number;
  url: string | null;
  error: string | null;
};

export type AgentKind =
  | 'claude_code'
  | 'codex_cli'
  | 'copilot_vscode'
  | 'cursor'
  | 'windsurf';

export const AGENT_ORDER: AgentKind[] = [
  'claude_code',
  'codex_cli',
  'copilot_vscode',
  'cursor',
  'windsurf',
];

export type InstallState = 'installed' | 'notInstalled' | 'conflict' | 'unsupported';

export type AgentStatus = {
  agent: AgentKind;
  displayName: string;
  state: InstallState;
  configPath: string;
  existingUrl?: string;
};

export type AgentInstallPlan = {
  agent: AgentKind;
  displayName: string;
  configPath: string;
  section: string;
  language: 'json' | 'toml';
  snippet: string;
  existingUrl?: string;
  backupPath?: string;
  willBackup: boolean;
};

export type McpLogStatus = 'ok' | 'error';

export type McpLogEntry = {
  id: string;
  ts: string;
  tool: string;
  request: unknown;
  response: unknown;
  durationMs: number;
  status: McpLogStatus;
};

const STATUS_KEY = ['mcp', 'status'];
const LOG_KEY = ['mcp', 'log'];
const AGENT_STATUS_KEY = (agent: AgentKind) => ['mcp', 'agent', agent];

export function useMcpStatus() {
  const queryClient = useQueryClient();
  const query = useQuery<McpStatus>({
    queryKey: STATUS_KEY,
    queryFn: () => invoke<McpStatus>('mcp_get_status'),
    // MCP status updates are pushed via `mcp-status-changed`, but poll
    // as a fallback in case the event system misses one.
    refetchInterval: 15_000,
  });

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen<McpStatus>('mcp-status-changed', event => {
        queryClient.setQueryData(STATUS_KEY, event.payload);
        // Agent install statuses depend on the current URL, so
        // whenever the server URL changes we recheck them.
        queryClient.invalidateQueries({ queryKey: ['mcp', 'agent'] });
      });
    })();
    return () => unlisten?.();
  }, [queryClient]);

  return query;
}

export function useMcpStart() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (port: number) => invoke('mcp_start', { port }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: STATUS_KEY }),
  });
}

export function useMcpStop() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => invoke('mcp_stop'),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: STATUS_KEY }),
  });
}

export function useMcpRestart() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (port: number) => invoke('mcp_restart', { port }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: STATUS_KEY }),
  });
}

export type LogListResponse = { entries: McpLogEntry[]; retentionDays: number };

export function useMcpLog(limit = 200) {
  const queryClient = useQueryClient();
  const query = useQuery<LogListResponse>({
    queryKey: [...LOG_KEY, limit],
    queryFn: () => invoke<LogListResponse>('mcp_log_list', { args: { limit } }),
  });

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    (async () => {
      unlisten = await listen('mcp-log-appended', () => {
        queryClient.invalidateQueries({ queryKey: LOG_KEY });
      });
    })();
    return () => unlisten?.();
  }, [queryClient]);

  return query;
}

export function useMcpLogClear() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => invoke('mcp_log_clear'),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: LOG_KEY }),
  });
}

export function useAgentStatus(agent: AgentKind) {
  return useQuery<AgentStatus>({
    queryKey: AGENT_STATUS_KEY(agent),
    queryFn: () => invoke<AgentStatus>('agent_status', { agent }),
  });
}

export function useAgentInstallPreview() {
  return useCallback(
    (agent: AgentKind) => invoke<AgentInstallPlan>('agent_install_preview', { agent }),
    [],
  );
}

export function useAgentInstall() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (agent: AgentKind) => invoke<AgentInstallPlan>('agent_install', { agent }),
    onSuccess: (_data, agent) =>
      queryClient.invalidateQueries({ queryKey: AGENT_STATUS_KEY(agent) }),
  });
}

export function useAgentUninstall() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (agent: AgentKind) => invoke('agent_uninstall', { agent }),
    onSuccess: (_data, agent) =>
      queryClient.invalidateQueries({ queryKey: AGENT_STATUS_KEY(agent) }),
  });
}

/**
 * Simple always-alive listener that reflects the running/stopped state
 * in one boolean — used by the header status dot without the caller
 * having to consume the whole query.
 */
export function useMcpHeaderStatus(): 'running' | 'stopped' | 'error' | 'disabled' {
  const status = useMcpStatus();
  return useMemo(() => {
    if (!status.data) return 'stopped';
    if (status.data.error) return 'error';
    if (status.data.running) return 'running';
    return 'disabled';
  }, [status.data]);
}

/** Convenience: force-refetch MCP status. */
export function useForceStatusRefresh() {
  const queryClient = useQueryClient();
  const [inFlight, setInFlight] = useState(false);
  return {
    inFlight,
    refresh: async () => {
      setInFlight(true);
      await queryClient.invalidateQueries({ queryKey: STATUS_KEY });
      setInFlight(false);
    },
  };
}
