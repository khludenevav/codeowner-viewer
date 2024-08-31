import { Repositories } from '@/app-config/app-config';
import { useQuery } from '@tanstack/react-query';
import { useAppConfig } from '@/app-config/useAppConfig';
import { ComboboxOption } from '@/components/ui/virtual-combobox';
import { invoke } from '@tauri-apps/api';

/** @return list of files in repository for specified branch  */
async function getBranchFiles(repository: Repositories, branch: string | null): Promise<string[]> {
  const owners = (await invoke('get_branch_files', {
    branch,
    absRepoPath: repository.repoPath,
  })) as string;
  return JSON.parse(owners) as string[];
}

function getQueryKeyForBranchFiles(branch: string | null) {
  return ['branches', branch];
}

export function useBranchFiles(branch: string | null) {
  const appConfigResponse = useAppConfig();

  const result = useQuery({
    queryKey: getQueryKeyForBranchFiles(branch),
    queryFn: () =>
      appConfigResponse.status === 'success'
        ? getBranchFiles(appConfigResponse.data.repositories[0], branch)
        : [],
    enabled: !!branch && appConfigResponse.status === 'success',
    refetchInterval: 1_000 * 60 * 5, // every 5 min
  });
  return result;
}

export function makeBranchFilesOptions(files: string[]): ComboboxOption[] {
  return files.map(f => ({
    value: f,
    label: f,
  }));
}
