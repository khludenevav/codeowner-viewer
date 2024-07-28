import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useCallback, useEffect, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { getBranchDifference } from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { ScrollArea } from '@radix-ui/react-scroll-area';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import useRefState from '@/utils/hooks/useRefState';
import { Branches, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { dayjs } from '@/utils/dayjs';
import { RefreshCw } from 'lucide-react';

export const Route = createFileRoute('/repositories/$repositoryId/codeowners')({
  component: Codeowners,
});

function makeBranchOptions(branchedData: Branches) {
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

function Codeowners() {
  const [isLoading, setIsLoading] = useState<boolean>();
  const [lastOwners, setLastOwners] = useState<Map<string, string[]> | null>(null);
  const [selectedBranchOption, selectedBranchOptionRef, setSelectedBranchOption] =
    useRefState<ComboboxOption | null>(null);
  const branchesResponse = useBranches();
  const updateBranchesList = useUpdateBranches();
  const [branchOptions, setBranchOptions] = useState<ComboboxOption[]>([]);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;

  useEffect(() => {
    if (branchesResponse.status === 'success') {
      const { branches, headOption } = makeBranchOptions(branchesResponse.data);
      setBranchOptions(branches);
      if (!selectedBranchOptionRef.current) {
        setSelectedBranchOption(headOption);
      }
    }
  }, [
    branchesResponse.data,
    branchesResponse.status,
    selectedBranchOptionRef,
    setSelectedBranchOption,
  ]);

  const onGetChangesCodeowners = useCallback(async () => {
    const branch = selectedBranchOptionRef.current?.value;
    if (!branch) {
      return;
    }
    const repository = appConfig?.repositories[0];
    if (repository) {
      setLastOwners(null);
      setIsLoading(true);
      try {
        const owners = await getBranchDifference(repository, branch);
        setLastOwners(owners);
      } finally {
        setIsLoading(false);
      }
    } else {
      console.log('No repo');
      setLastOwners(null);
    }
  }, [appConfig?.repositories, selectedBranchOptionRef]);

  const selectedBranchChanged = useCallback(
    (newBranchOption: ComboboxOption) => {
      setSelectedBranchOption(newBranchOption);
      onGetChangesCodeowners();
    },
    [onGetChangesCodeowners, setSelectedBranchOption],
  );

  if (!appConfig) {
    return 'Loading app config...';
  }

  if (appConfig.repositories.length === 0) {
    return <Navigate to='/settings' />;
  }

  return (
    <div className='flex flex-col gap-4'>
      <span>
        Pick a branch name to get the codeowners for changed files comparing with 'main' branch.
      </span>
      <div className='flex gap-2 justify-between'>
        <VirtualizedCombobox
          options={branchOptions}
          selectedOption={selectedBranchOption}
          selectedChanged={selectedBranchChanged}
          searchPlaceholder='Select branch ...'
          height='400px'
          disabled={branchesResponse.status !== 'success'}
        />

        <div className='flex gap-2 items-center'>
          <Button
            variant='ghost'
            size='icon'
            onClick={branchesResponse.fetchStatus === 'idle' ? updateBranchesList : undefined}
          >
            <RefreshCw className='h-5 w-5' />
          </Button>
          <span className='text-sm'>
            Branch list updated
            <br />
            at {dayjs(branchesResponse.dataUpdatedAt).format('HH:mm:ss')}
          </span>
        </div>
      </div>

      {isLoading && <div>Calculating codeowners...</div>}
      {lastOwners && !isLoading && (
        <div className='flex flex-col gap-2'>
          <span>Codeowners for changed files:</span>
          <ScrollArea className='w-full p-4 rounded-md border'>
            <pre className='text-sm text-neutral-900 dark:text-neutral-400'>
              {JSON.stringify(Object.fromEntries(lastOwners.entries()), null, 2)}
            </pre>
          </ScrollArea>
        </div>
      )}
    </div>
  );
}
