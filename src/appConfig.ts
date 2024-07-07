import { readTextFile, writeTextFile, exists, BaseDirectory, createDir } from '@tauri-apps/api/fs';

export type Repositories = {
  /** Absolute path to repository */
  repoPart: string;
  /** Relative path to codeowners from repoPath */
  codeowners: string;
};

export type AppConfig = {
  repositories: Repositories[];
};

const CONFIG_FILE_NAME = 'config.json';

export async function readAppConfig(): Promise<AppConfig> {
  let config: AppConfig;
  if (await exists(CONFIG_FILE_NAME, { dir: BaseDirectory.AppConfig })) {
    const configAsJson = await readTextFile(CONFIG_FILE_NAME, { dir: BaseDirectory.AppConfig });
    config = JSON.parse(configAsJson);
  } else {
    config = {
      repositories: [],
    };
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
