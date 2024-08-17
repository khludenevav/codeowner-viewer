import { Repositories } from '../app-config/app-config';
import { invoke } from '@tauri-apps/api';
import { useAppConfig } from '@/app-config/useAppConfig';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

async function getBranchDifference(
  repository: Repositories,
  branch: string,
): Promise<null | Map<string, string[]>> {
  const owners = (await invoke('get_changed_codeowners_for_branch', {
    branch,
    absRepoPath: repository.repoPath,
  })) as string;
  // We pass it as list in order to get always the same data in the same order.
  const parsedOwners = JSON.parse(owners) as { owners: string; files: string[] }[];

  return parsedOwners.reduce((acc, item) => {
    acc.set(item.owners, item.files);
    return acc;
  }, new Map<string, string[]>());
}

function getBranchCodeownersQueryKey(branch: string | null) {
  return ['branch', branch ?? '', 'codeowners'];
}

export function useBranchCodeowners(branch: string | null) {
  const appConfigResponse = useAppConfig();

  const result = useQuery({
    queryKey: getBranchCodeownersQueryKey(branch),
    queryFn: () =>
      appConfigResponse.status === 'success'
        ? getBranchDifference(appConfigResponse.data.repositories[0], branch!)
        : null,
    enabled: !!branch && appConfigResponse.status === 'success',
    refetchInterval: 1_000 * 60 * 5, // every 5 min
  });
  return result;
}

export function useUpdateBranchCodeowners(branch: string | null) {
  const queryClient = useQueryClient();
  return useCallback(() => {
    queryClient.invalidateQueries({ queryKey: getBranchCodeownersQueryKey(branch) });
  }, [branch, queryClient]);
}
