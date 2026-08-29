import { readTextFile, writeTextFile, exists, BaseDirectory, createDir } from '@tauri-apps/api/fs';

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

export type AppConfig = {
  theme: ThemeSettings;
  repositories: Repositories[];
};

const CONFIG_FILE_NAME = 'config.json';
export const DEFAULT_COLOR_THEME: ColorTheme = 'system';
export const DEFAULT_THEME: ThemeSettings = {
  colorTheme: DEFAULT_COLOR_THEME,
};
export const DEFAULT_APP_CONFIG: AppConfig = {
  repositories: [],
  theme: DEFAULT_THEME,
};

function fillConfigForOlderVersions(appConfig: AppConfig) {
  if (!appConfig.theme) {
    appConfig.theme = DEFAULT_THEME;
  }
  if (Array.isArray(appConfig.repositories)) {
    for (const repo of appConfig.repositories) {
      if (!repo.id) {
        repo.id = generateRepositoryId();
      }
    }
  }
}

export async function readAppConfig(): Promise<AppConfig> {
  let config: AppConfig;
  if (await exists(CONFIG_FILE_NAME, { dir: BaseDirectory.AppConfig })) {
    const configAsJson = await readTextFile(CONFIG_FILE_NAME, { dir: BaseDirectory.AppConfig });
    config = JSON.parse(configAsJson);
    fillConfigForOlderVersions(config);
  } else {
    config = DEFAULT_APP_CONFIG;
    await createDir('', { dir: BaseDirectory.AppConfig, recursive: true });
    await writeTextFile(CONFIG_FILE_NAME, JSON.stringify(config, null, 2), {
      dir: BaseDirectory.AppConfig,
    });
  }

  return config;
}

export async function writeAppConfig(config: AppConfig) {
  await writeTextFile(CONFIG_FILE_NAME, JSON.stringify(config, null, 2), {
    dir: BaseDirectory.AppConfig,
  });
}
