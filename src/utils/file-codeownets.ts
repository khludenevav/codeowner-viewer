import { Repositories } from '@/app-config/app-config';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api';
import { useCallback } from 'react';

function getBranchFileCodeownersQueryKey(
  repositoryId: string | null,
  branch: string | null,
  file: string | null,
) {
  return ['repo', repositoryId ?? '', 'branch', branch ?? '', file, 'codeowners'];
}

export function useFileCodeowners(
  repository: Repositories | null,
  branch: string | null,
  file: string | null,
) {
  const result = useQuery({
    queryKey: getBranchFileCodeownersQueryKey(repository?.id ?? null, branch, file),
    queryFn: async () => {
      if (!repository) {
        return null;
      }
      return (await invoke('get_codeowners_for_branch_file', {
        branch,
        absRepoPath: repository.repoPath,
        file,
      })) as string;
    },
    enabled: !!branch && !!file && !!repository,
    refetchInterval: 1_000 * 60 * 5, // every 5 min
  });
  return result;
}

export function useUpdateFileCodeowners(
  repository: Repositories | null,
  branch: string | null,
  file: string | null,
) {
  const queryClient = useQueryClient();
  return useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: getBranchFileCodeownersQueryKey(repository?.id ?? null, branch, file),
    });
  }, [branch, file, queryClient, repository?.id]);
}
