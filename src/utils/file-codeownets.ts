import { useAppConfig } from '@/app-config/useAppConfig';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api';
import { useCallback } from 'react';

function getBranchFileCodeownersQueryKey(branch: string | null, file: string | null) {
  return ['branch', branch ?? '', file, 'codeowners'];
}

export function useFileCodeowners(branch: string | null, file: string | null) {
  const appConfigResponse = useAppConfig();

  const result = useQuery({
    queryKey: getBranchFileCodeownersQueryKey(branch, file),
    queryFn: async () => {
      if (appConfigResponse.status !== 'success') {
        return null;
      }
      return (await invoke('get_codeowners_for_branch_file', {
        branch,
        absRepoPath: appConfigResponse.data.repositories[0].repoPath,
        file,
      })) as string;
    },
    enabled: !!branch && !!file && appConfigResponse.status === 'success',
    refetchInterval: 1_000 * 60 * 5, // every 5 min
  });
  return result;
}

export function useUpdateFileCodeowners(branch: string | null, file: string | null) {
  const queryClient = useQueryClient();
  return useCallback(() => {
    queryClient.invalidateQueries({ queryKey: getBranchFileCodeownersQueryKey(branch, file) });
  }, [branch, file, queryClient]);
}
