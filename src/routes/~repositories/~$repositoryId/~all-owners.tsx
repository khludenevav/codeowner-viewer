import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useEffect, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import { makeBranchOptions, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { dayjs } from '@/utils/dayjs';
import { Tooltip } from '@/components/ui/tooltip';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import { useAllCodeowners } from '@/utils/all-owners';

export const Route = createFileRoute('/repositories/$repositoryId/all-owners')({
  component: Codeowners,
});

function Codeowners() {
  const [branchOptions, setBranchOptions] = useState<ComboboxOption[]>([]);
  const [selectedBranchOption, setSelectedBranchOption] = useState<ComboboxOption | null>(null);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;
  const normalizedSelectedBranch = selectedBranchOption?.value ?? null;

  const allCodeownersResponse = useAllCodeowners(normalizedSelectedBranch);
  const branchesResponse = useBranches();

  const updateBranchesList = useUpdateBranches();

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
      <span>Pick a branch and below you will see an owners for each file in the repo.</span>
      <div className='flex gap-2 justify-between mt-2 mb-2'>
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
      <div>allCodeownersResponse: {allCodeownersResponse.status},</div>
      <pre>{JSON.stringify(allCodeownersResponse.data, null, 2)}</pre>
    </div>
  );
}
