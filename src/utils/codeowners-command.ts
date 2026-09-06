import { Repositories } from '../app-config/app-config';
import { invoke } from '@tauri-apps/api/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

export type BranchFile = { path: string; comment: string | null; uncommitted?: boolean };

async function getBranchDifference(
  repository: Repositories,
  branch: string,
  includeUncommitted: boolean,
): Promise<null | Map<string, BranchFile[]>> {
  const owners = (await invoke('get_changed_codeowners_for_branch', {
    branch,
    absRepoPath: repository.repoPath,
    includeUncommitted,
  })) as string;
  // We pass it as list in order to get always the same data in the same order.
  const parsedOwners = JSON.parse(owners) as { owners: string; files: BranchFile[] }[];

  return parsedOwners.reduce((acc, item) => {
    acc.set(item.owners, item.files);
    return acc;
  }, new Map<string, BranchFile[]>());
}

function getBranchCodeownersQueryKey(
  repositoryId: string | null,
  branch: string | null,
  includeUncommitted: boolean,
) {
  return [
    'repo',
    repositoryId ?? '',
    'branch',
    branch ?? '',
    'codeowners',
    { includeUncommitted },
  ];
}

export function useBranchCodeowners(
  repository: Repositories | null,
  branch: string | null,
  includeUncommitted: boolean,
) {
  const result = useQuery({
    queryKey: getBranchCodeownersQueryKey(repository?.id ?? null, branch, includeUncommitted),
    queryFn: () =>
      repository ? getBranchDifference(repository, branch!, includeUncommitted) : null,
    enabled: !!branch && !!repository,
    refetchInterval: 1_000 * 60 * 5, // every 5 min
  });
  return result;
}

export function useUpdateBranchCodeowners(
  repository: Repositories | null,
  branch: string | null,
  includeUncommitted: boolean,
) {
  const queryClient = useQueryClient();
  return useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: getBranchCodeownersQueryKey(repository?.id ?? null, branch, includeUncommitted),
    });
  }, [branch, includeUncommitted, queryClient, repository?.id]);
}
