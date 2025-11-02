import { Repositories } from '../app-config/app-config';
import { invoke } from '@tauri-apps/api';
import { useAppConfig } from '@/app-config/useAppConfig';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useWebViewSessionId } from './WebViewSessionIdProvider';

export type FileOwners = {
  /** File name */
  name: string;
  owner: string;
};

export type DirectoryOwners = {
  /** Directory name. For root folder it is empty */
  name: string;
  directories: DirectoryOwners[];
  files: FileOwners[];
  /**
   * @return string which contains all owners in case every files/directories inside have their own owners.
   *   null in other case (for root directory also null)
   */
  owner: string | null;
};

async function getAllOwners(
  repository: Repositories,
  branch: string,
  sessionId: string,
): Promise<DirectoryOwners> {
  const owners = (await invoke('get_all_codeowners_for_branch', {
    branch,
    absRepoPath: repository.repoPath,
    sessionId,
  })) as string;
  return JSON.parse(owners) as DirectoryOwners;
}

function getAllCodeownersQueryKey(branch: string | null) {
  return ['branch', branch ?? '', 'all-codeowners'];
}

function getAllCodeownersProgressQueryKey(branch: string | null) {
  return ['branch', branch ?? '', 'all-codeowners-progress'];
}

type AllCodeownersProgressPayload = {
  session_id: string;
  files_handled: number;
  files_total: number;
};

export function useAllCodeowners(branch: string | null) {
  const appConfigResponse = useAppConfig();
  const sessionId = useWebViewSessionId();

  const result = useQuery({
    queryKey: getAllCodeownersQueryKey(branch),
    queryFn: () =>
      appConfigResponse.status === 'success'
        ? getAllOwners(appConfigResponse.data.repositories[0], branch!, sessionId)
        : null,
    enabled: !!branch && appConfigResponse.status === 'success',
    staleTime: 1_000 * 60 * 60, // every 60 min. Reloads request when you switch tabs, for example
    refetchInterval: 1_000 * 60 * 30, // every 30 min
    refetchOnWindowFocus: false, // for now request is too heavy
  });
  return result;
}

export function useUpdateAllCodeowners(branch: string | null) {
  const queryClient = useQueryClient();
  return useCallback(() => {
    queryClient.invalidateQueries({ queryKey: getAllCodeownersQueryKey(branch) });
  }, [branch, queryClient]);
}

export function useAllCodeownersProgress(branch: string | null) {
  const appConfigResponse = useAppConfig();
  const queryClient = useQueryClient();
  const sessionId = useWebViewSessionId();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      unlisten = await listen<AllCodeownersProgressPayload>('all-codeowners-progress', event => {
        if (event.payload.session_id === sessionId) {
          queryClient.setQueryData<AllCodeownersProgressPayload>(
            getAllCodeownersProgressQueryKey(branch),
            event.payload,
          );
        }
      });
    };
    setupListener();
    return () => {
      unlisten?.();
    };
  }, [branch, queryClient, sessionId]);

  return useQuery<AllCodeownersProgressPayload>({
    queryKey: getAllCodeownersProgressQueryKey(branch),
    enabled: !!branch && appConfigResponse.status === 'success',
    staleTime: Infinity,
    refetchInterval: Infinity,
    refetchOnWindowFocus: false,
    initialData: {
      session_id: sessionId,
      files_handled: 0,
      files_total: 0,
    },
  });
}
