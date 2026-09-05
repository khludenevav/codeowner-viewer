import { readTextFile, writeTextFile, exists, BaseDirectory, mkdir } from '@tauri-apps/plugin-fs';
import { emit } from '@tauri-apps/api/event';

export type Repositories = {
  /** Stable auto-generated identifier used in URLs and query keys */
  id: string;
  /** Absolute path to repository */
  repoPath: string;
  /** Relative path to codeowners from repoPath */
  codeowners: string;
};

export function generateRepositoryId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `repo-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export type ColorTheme = 'dark' | 'light' | 'system';

export type ThemeSettings = {
  colorTheme: ColorTheme;
};

export type McpSettings = {
  /** MCP server is enabled by default. Users can toggle it off from /mcp. */
  enabled: boolean;
  /** Loopback port the MCP server binds to. */
  port: number;
};

export type AppConfig = {
  theme: ThemeSettings;
  repositories: Repositories[];
  mcp: McpSettings;
};

const CONFIG_FILE_NAME = 'config.json';
export const DEFAULT_COLOR_THEME: ColorTheme = 'system';
export const DEFAULT_THEME: ThemeSettings = {
  colorTheme: DEFAULT_COLOR_THEME,
};
export const DEFAULT_MCP_PORT = 47821;
export const DEFAULT_MCP_SETTINGS: McpSettings = {
  enabled: true,
  port: DEFAULT_MCP_PORT,
};
export const DEFAULT_APP_CONFIG: AppConfig = {
  repositories: [],
  theme: DEFAULT_THEME,
  mcp: DEFAULT_MCP_SETTINGS,
};

function fillConfigForOlderVersions(appConfig: AppConfig): boolean {
  let migrated = false;
  if (!appConfig.theme) {
    appConfig.theme = DEFAULT_THEME;
    migrated = true;
  }
  if (!appConfig.mcp) {
    appConfig.mcp = { ...DEFAULT_MCP_SETTINGS };
    migrated = true;
  } else {
    if (typeof appConfig.mcp.enabled !== 'boolean') {
      appConfig.mcp.enabled = DEFAULT_MCP_SETTINGS.enabled;
      migrated = true;
    }
    if (typeof appConfig.mcp.port !== 'number' || !Number.isFinite(appConfig.mcp.port)) {
      appConfig.mcp.port = DEFAULT_MCP_SETTINGS.port;
      migrated = true;
    }
  }
  if (Array.isArray(appConfig.repositories)) {
    for (const repo of appConfig.repositories) {
      if (!repo.id) {
        repo.id = generateRepositoryId();
        migrated = true;
      }
    }
  }
  return migrated;
}

export async function readAppConfig(): Promise<AppConfig> {
  let config: AppConfig;
  if (await exists(CONFIG_FILE_NAME, { baseDir: BaseDirectory.AppConfig })) {
    const configAsJson = await readTextFile(CONFIG_FILE_NAME, { baseDir: BaseDirectory.AppConfig });
    config = JSON.parse(configAsJson);
    const migrated = fillConfigForOlderVersions(config);
    if (migrated) {
      await writeAppConfig(config);
    }
  } else {
    config = DEFAULT_APP_CONFIG;
    await mkdir('', { baseDir: BaseDirectory.AppConfig, recursive: true });
    await writeAppConfig(config);
  }

  return config;
}

export async function writeAppConfig(config: AppConfig) {
  await writeTextFile(CONFIG_FILE_NAME, JSON.stringify(config, null, 2), {
    baseDir: BaseDirectory.AppConfig,
  });
  // Rust side reloads AppConfig (repos + MCP settings) on this event so it
  // doesn't have to poll config.json.
  try {
    await emit('app-config-updated');
  } catch {
    // Best-effort — swallow if the runtime isn't ready (tests, teardown).
  }
}
