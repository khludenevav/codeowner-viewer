import { Repositories } from '@/app-config/app-config';
import { Command } from '@tauri-apps/api/shell';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useAppConfig } from '@/app-config/useAppConfig';
import { useCallback } from 'react';
import { ComboboxOption } from '@/components/ui/virtual-combobox';

const CURRENT_PREFIX = '* ';
const REMOTE_PREFIX = 'remotes/';

export type Branches = {
  current: string;
  locals: string[];
  remotes: string[];
};

const NO_BRANCHES: Branches = {
  current: '',
  locals: [],
  remotes: [],
};

async function getBranches(repository: Repositories): Promise<Branches> {
  const command = new Command('run-git-command', ['--no-pager', 'branch', '-a'], {
    cwd: repository.repoPath,
  });
  const output = await command.execute();
  if (output.code !== 0) {
    console.log(`Execution code: ${output.code}.\n${output.stderr}`);
    return NO_BRANCHES;
  }
  const lines = output.stdout.split('\n');
  let current: string = '';
  const locals: string[] = [];
  const remotes: string[] = [];
  for (const line of lines) {
    if (line.length === 0) {
      // Last line of output is empty
      continue;
    }
    if (line.startsWith(CURRENT_PREFIX)) {
      current = line.trim().substring(CURRENT_PREFIX.length);
      continue;
    }
    const name = line.trim();
    if (name.startsWith(REMOTE_PREFIX)) {
      if (/ -> /.test(name)) {
        // Filter out external head.
        // represented as line 'origin/HEAD -> origin/main'
        continue;
      }
      remotes.push(name.substring(REMOTE_PREFIX.length));
      continue;
    }
    locals.push(name);
  }

  return {
    current,
    locals,
    remotes,
  };
}

const branchesQueryKey = ['branches'];

export function useBranches() {
  const appConfigResponse = useAppConfig();

  const result = useQuery({
    queryKey: branchesQueryKey,
    queryFn: () =>
      appConfigResponse.status === 'success'
        ? getBranches(appConfigResponse.data.repositories[0])
        : NO_BRANCHES,
    enabled: appConfigResponse.status === 'success',
    refetchInterval: 1_000 * 60 * 5, // every 5 min
  });
  return result;
}

export function useUpdateBranches() {
  const queryClient = useQueryClient();
  const updateBranches = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: branchesQueryKey });
  }, [queryClient]);
  return updateBranches;
}

export function makeBranchOptions(branchedData: Branches) {
  const headOption = {
    value: branchedData.current,
    label: `(HEAD) ${branchedData.current}`,
  };
  const branches: ComboboxOption[] = [headOption];
  for (const branch of branchedData.locals) {
    branches.push({ value: branch, label: branch });
  }
  for (const branch of branchedData.remotes) {
    branches.push({ value: branch, label: branch });
  }
  return { branches, headOption };
}
