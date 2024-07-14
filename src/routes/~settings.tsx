import { createFileRoute, useNavigate } from '@tanstack/react-router';

import { useCallback } from 'react';

import { DEFAULT_APP_CONFIG } from '../app-config/app-config';
import { open } from '@tauri-apps/api/dialog';
import { path } from '@tauri-apps/api';
import { useAppConfig, useUpdateAppConfig } from '../app-config/useAppConfig';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Terminal } from 'lucide-react';

export const Route = createFileRoute('/settings')({
  component: () => <Settings />,
});

function Settings() {
  const navigate = useNavigate();
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
    appConfigUpdate.mutate(
      {
        appConfig: {
          ...appConfigResponse.data,
          repositories: [
            ...appConfigResponse.data.repositories,
            { repoPath: selectedDirectory, codeowners: 'CODEOWNERS' },
          ],
        },
      },
      {
        onSuccess: () => {
          // Redirect only first time
          if (appConfigResponse.data.repositories.length === 0) {
            navigate({
              to: '/repositories/$repositoryId/codeowners',
              params: { repositoryId: 'any' },
            });
          }
        },
      },
    );
  }, [appConfigResponse.data, appConfigResponse.status, appConfigUpdate, navigate]);

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
    <div className='flex flex-col gap-4'>
      <div>Application config:</div>
      <pre>{JSON.stringify(appConfigResponse.data, null, 2)}</pre>
      <div className='flex gap-2 flex-wrap md:flex-nowrap'>
        <Button onClick={addRepository} size='sm'>
          Add repository
        </Button>
        <Button onClick={removeRepository} variant='destructive' size='sm'>
          Remove last repository
        </Button>
        <Button onClick={resetEntireAppConfig} variant='destructive' size='sm'>
          Reset entire app config
        </Button>
      </div>
      {appConfigResponse.data.repositories.length > 1 && (
        <Alert>
          <Terminal className='h-4 w-4' />
          <AlertTitle>You added more than one repository</AlertTitle>
          <AlertDescription>
            For now using more than one repository is not supported. Program will always use the
            first one in the list.
          </AlertDescription>
        </Alert>
      )}
    </div>
  );
}
