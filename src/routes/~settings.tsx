import { createFileRoute } from '@tanstack/react-router';

import { useCallback } from 'react';

import { DEFAULT_APP_CONFIG } from '../app-config/app-config';
import { open } from '@tauri-apps/api/dialog';
import { path } from '@tauri-apps/api';
import { useAppConfig, useUpdateAppConfig } from '../app-config/useAppConfig';

export const Route = createFileRoute('/settings')({
  component: () => <Settings />,
});

function Settings() {
  const appConfigResponse = useAppConfig();
  const appConfigUpdate = useUpdateAppConfig();

  const addRepository = useCallback(async () => {
    if (appConfigResponse.status !== 'success') {
      return;
    }

    const selectedDirectory = await open({
      title: 'Select repository directory',
      defaultPath: await path.homeDir(),
      directory: true,
      recursive: true,
    });
    if (!selectedDirectory || Array.isArray(selectedDirectory)) {
      return;
    }
    appConfigUpdate.mutate({
      appConfig: {
        ...appConfigResponse.data,
        repositories: [
          ...appConfigResponse.data.repositories,
          { repoPath: selectedDirectory, codeowners: 'CODEOWNERS' },
        ],
      },
    });
  }, [appConfigResponse.data, appConfigResponse.status, appConfigUpdate]);

  const removeRepository = useCallback(async () => {
    if (appConfigResponse.status !== 'success') {
      return;
    }
    const repositories = [...appConfigResponse.data.repositories];
    repositories.pop();
    appConfigUpdate.mutate({
      appConfig: {
        ...appConfigResponse.data,
        repositories,
      },
    });
  }, [appConfigResponse.data, appConfigResponse.status, appConfigUpdate]);

  const resetEntireAppConfig = useCallback(async () => {
    if (appConfigResponse.status !== 'success') {
      return;
    }
    appConfigUpdate.mutate({
      appConfig: DEFAULT_APP_CONFIG,
    });
  }, [appConfigResponse.status, appConfigUpdate]);

  if (appConfigResponse.status === 'error') {
    return `Error: ${appConfigResponse.error}`;
  }

  if (appConfigResponse.status === 'pending') {
    return 'Loading app config...';
  }

  return (
    <>
      <div>Config:</div>
      <pre>{JSON.stringify(appConfigResponse.data, null, 2)}</pre>
      <div>
        <button onClick={addRepository}>Add repository</button>
        <button onClick={removeRepository}>Remove repository</button>
        <button onClick={resetEntireAppConfig}>Reset entire app config</button>
      </div>
    </>
  );
}
