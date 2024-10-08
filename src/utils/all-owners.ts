import { Repositories } from '../app-config/app-config';
import { invoke } from '@tauri-apps/api';
import { useAppConfig } from '@/app-config/useAppConfig';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

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

async function getAllOwners(repository: Repositories, branch: string): Promise<DirectoryOwners> {
  const owners = (await invoke('get_all_codeowners_for_branch', {
    branch,
    absRepoPath: repository.repoPath,
  })) as string;
  return JSON.parse(owners) as DirectoryOwners;
}

function getAllCodeownersQueryKey(branch: string | null) {
  return ['branch', branch ?? '', 'all-codeowners'];
}

export function useAllCodeowners(branch: string | null) {
  const appConfigResponse = useAppConfig();

  const result = useQuery({
    queryKey: getAllCodeownersQueryKey(branch),
    queryFn: () =>
      appConfigResponse.status === 'success'
        ? getAllOwners(appConfigResponse.data.repositories[0], branch!)
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
