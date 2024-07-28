import { Command } from '@tauri-apps/api/shell';
import { getOwnerTeam, parseCodeowners } from './codeowners-utils';
import { Repositories } from '../app-config/app-config';
import { invoke } from '@tauri-apps/api';
import { useAppConfig } from '@/app-config/useAppConfig';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

async function getBranchDifference(
  repository: Repositories,
  branch: string,
): Promise<null | Map<string, string[]>> {
  const command = new Command(
    'run-git-command',
    ['--no-pager', 'diff', '--name-only', `origin/main...${branch}`],
    { cwd: repository.repoPath },
  );
  const output = await command.execute();
  if (output.code !== 0) {
    console.log(`Execution code: ${output.code}.\n${output.stderr}`);
    return null;
  }
  const lines = output.stdout.split('\n');

  const codeownersFileContent: string = await invoke('get_codeowners_content', {
    branch,
    absRepoPath: repository.repoPath,
  });

  const codeowners = parseCodeowners(codeownersFileContent);

  const owners = new Map<string, string[]>();
  for (const filePath of lines) {
    if (!filePath) {
      continue;
    }
    const ownerTeam = getOwnerTeam({ codeowners, filename: filePath });
    let teamFiles = owners.get(ownerTeam);
    if (!teamFiles) {
      teamFiles = [];
      owners.set(ownerTeam, teamFiles);
    }
    teamFiles.push(filePath);
  }
  return owners;
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
  const updateBranchCodeowners = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: getBranchCodeownersQueryKey(branch) });
  }, [branch, queryClient]);
  return updateBranchCodeowners;
}
