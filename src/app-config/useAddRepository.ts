import { useCallback } from 'react';
import { homeDir } from '@tauri-apps/api/path';
import { open } from '@tauri-apps/plugin-dialog';
import { useNavigate } from '@tanstack/react-router';

import { generateRepositoryId, Repositories } from './app-config';
import { useAppConfig, useUpdateAppConfig } from './useAppConfig';

/**
 * Opens a native directory picker and appends the chosen directory to the
 * app config as a new repository, then navigates to it. Shared between the
 * header "+" button and the welcome / empty-state screen.
 */
export function useAddRepository() {
  const navigate = useNavigate();
  const appConfigResponse = useAppConfig();
  const appConfigUpdate = useUpdateAppConfig();

  return useCallback(async () => {
    if (appConfigResponse.status !== 'success') {
      return;
    }
    const selectedDirectory = await open({
      title: 'Select repository directory',
      defaultPath: await homeDir(),
      directory: true,
    });
    if (!selectedDirectory || Array.isArray(selectedDirectory)) {
      return;
    }
    const newRepository: Repositories = {
      id: generateRepositoryId(),
      repoPath: selectedDirectory,
      codeowners: 'CODEOWNERS',
    };
    appConfigUpdate.mutate(
      {
        appConfig: {
          ...appConfigResponse.data,
          repositories: [...appConfigResponse.data.repositories, newRepository],
        },
      },
      {
        onSuccess: () => {
          navigate({
            to: '/repositories/$repositoryId/codeowners',
            params: { repositoryId: newRepository.id },
          });
        },
      },
    );
  }, [appConfigResponse.data, appConfigResponse.status, appConfigUpdate, navigate]);
}
