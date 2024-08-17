import { Repositories } from '@/app-config/app-config';
import { Command } from '@tauri-apps/api/shell';
import { useQuery } from '@tanstack/react-query';
import { useAppConfig } from '@/app-config/useAppConfig';
import { ComboboxOption } from '@/components/ui/virtual-combobox';

/** @return list of files in repository for specified branch  */
async function getBranchFiles(repository: Repositories, branch: string | null): Promise<string[]> {
  const command = new Command('run-git-command', ['ls-tree', '-r', branch ?? '', '--name-only'], {
    cwd: repository.repoPath,
  });

  const output = await command.execute();
  if (output.code !== 0) {
    console.log(`Execution code: ${output.code}.\n${output.stderr}`);
    return [];
  }
  return output.stdout.split('\n').filter(l => !!l);
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
