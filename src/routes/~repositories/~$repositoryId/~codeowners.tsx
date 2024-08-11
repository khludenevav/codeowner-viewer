import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useEffect, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { useBranchCodeowners, useUpdateBranchCodeowners } from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import { Branches, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { dayjs } from '@/utils/dayjs';
import { Tooltip } from '@/components/ui/tooltip';
import { RefreshIcon } from '@/components/icons/refresh-icon';

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
  const [branchOptions, setBranchOptions] = useState<ComboboxOption[]>([]);
  const [selectedBranchOption, setSelectedBranchOption] = useState<ComboboxOption | null>(null);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;

  const branchesResponse = useBranches();
  const updateBranchesList = useUpdateBranches();

  const normalizedSelectedBranch = selectedBranchOption?.value ?? null;
  const branchCodeownersResponse = useBranchCodeowners(normalizedSelectedBranch);
  const updateBranchCodeowners = useUpdateBranchCodeowners(normalizedSelectedBranch);

  useEffect(() => {
    if (branchesResponse.status === 'success') {
      const { branches, headOption } = makeBranchOptions(branchesResponse.data);
      setBranchOptions(branches);
      if (!selectedBranchOption) {
        setSelectedBranchOption(headOption);
      }
    }
  }, [
    branchesResponse.data,
    branchesResponse.status,
    selectedBranchOption,
    setSelectedBranchOption,
  ]);

  if (!appConfig) {
    return 'Loading app config...';
  }

  if (appConfig.repositories.length === 0) {
    return <Navigate to='/settings' />;
  }

  return (
    <div className='flex flex-col'>
      <span>
        Pick a branch name to get the codeowners for changed files comparing with 'main' branch.
      </span>

      <div className='flex gap-2 justify-between mt-2 mb-6'>
        <VirtualizedCombobox
          options={branchOptions}
          selectedOption={selectedBranchOption}
          selectedChanged={setSelectedBranchOption}
          searchPlaceholder='Select branch ...'
          height='400px'
          disabled={branchesResponse.status !== 'success'}
        />

        <div className='flex gap-2 items-center'>
          <Tooltip content='Update branches list'>
            <Button
              variant='ghost'
              size='icon'
              loading={branchesResponse.fetchStatus === 'fetching'}
              onClick={branchesResponse.fetchStatus === 'idle' ? updateBranchesList : undefined}
            >
              <RefreshIcon className='[animation-duration:2500ms]' />
            </Button>
          </Tooltip>
          <span className='text-sm'>
            Branch list updated
            <br />
            at {dayjs(branchesResponse.dataUpdatedAt).format('HH:mm:ss')}
          </span>
        </div>
      </div>

      {branchCodeownersResponse.status === 'pending' && <div>Calculating codeowners...</div>}
      {branchCodeownersResponse.status === 'error' && <div>Calculating codeowners error</div>}
      {branchCodeownersResponse.data && (
        <div className='flex flex-col gap-2'>
          <div className='flex gap-2 justify-between items-center'>
            <span>Codeowners for changed files:</span>{' '}
            <span className='text-sm'>
              Codeowners updated at{' '}
              {dayjs(branchCodeownersResponse.dataUpdatedAt).format('HH:mm:ss')}
            </span>
          </div>
          <div className='w-full p-4 rounded-md border overflow-auto relative'>
            <Tooltip content='Update codeowners'>
              <Button
                className='absolute top-2 right-2'
                variant='ghost'
                size='icon'
                loading={branchCodeownersResponse.fetchStatus === 'fetching'}
                onClick={
                  branchCodeownersResponse.fetchStatus === 'idle'
                    ? updateBranchCodeowners
                    : undefined
                }
              >
                <RefreshIcon className='[animation-duration:2500ms]' />
              </Button>
            </Tooltip>
            <pre className='text-sm text-neutral-900 dark:text-neutral-400'>
              {JSON.stringify(Object.fromEntries(branchCodeownersResponse.data.entries()), null, 2)}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
