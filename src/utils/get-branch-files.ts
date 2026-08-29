import { Repositories } from '@/app-config/app-config';
import { useQuery } from '@tanstack/react-query';
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

function getQueryKeyForBranchFiles(repositoryId: string | null, branch: string | null) {
  return ['repo', repositoryId ?? '', 'branches', branch];
}

export function useBranchFiles(repository: Repositories | null, branch: string | null) {
  const result = useQuery({
    queryKey: getQueryKeyForBranchFiles(repository?.id ?? null, branch),
    queryFn: () => (repository ? getBranchFiles(repository, branch) : []),
    enabled: !!branch && !!repository,
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
