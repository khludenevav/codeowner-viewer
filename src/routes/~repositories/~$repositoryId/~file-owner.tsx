import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useEffect, useMemo, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { useUpdateFileCodeowners } from '../../../utils/file-codeownets';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import { makeBranchOptions, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { dayjs } from '@/utils/dayjs';
import { Tooltip } from '@/components/ui/tooltip';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import { useFileCodeowners } from '@/utils/file-codeownets';
import { makeBranchFilesOptions, useBranchFiles } from '@/utils/get-branch-files';

export const Route = createFileRoute('/repositories/$repositoryId/file-owner')({
  component: Codeowners,
});

function Codeowners() {
  const [branchOptions, setBranchOptions] = useState<ComboboxOption[]>([]);
  const [selectedBranchOption, setSelectedBranchOption] = useState<ComboboxOption | null>(null);
  const [selectedFileOption, setSelectedFileOption] = useState<ComboboxOption | null>(null);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;
  const normalizedSelectedBranch = selectedBranchOption?.value ?? null;
  const normalizedSelectedFile = selectedFileOption?.value ?? null;

  const branchesResponse = useBranches();
  const filesResponse = useBranchFiles(normalizedSelectedBranch);
  const branchFilesOptions = useMemo(
    () => (filesResponse.status === 'success' ? makeBranchFilesOptions(filesResponse.data) : []),
    [filesResponse.data, filesResponse.status],
  );
  const updateBranchesList = useUpdateBranches();

  const fileCodeownersResponse = useFileCodeowners(
    normalizedSelectedBranch,
    normalizedSelectedFile,
  );
  const updateFileCodeowners = useUpdateFileCodeowners(
    normalizedSelectedBranch,
    normalizedSelectedFile,
  );

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

  useEffect(() => {
    if (
      filesResponse.status === 'success' &&
      !selectedFileOption &&
      branchFilesOptions.length > 0
    ) {
      setSelectedFileOption(branchFilesOptions[0]);
    }
  }, [branchFilesOptions, filesResponse.status, selectedFileOption]);

  if (!appConfig) {
    return 'Loading app config...';
  }

  if (appConfig.repositories.length === 0) {
    return <Navigate to='/settings' />;
  }

  return (
    <div className='flex flex-col mx-6 mb-6 max-h-full'>
      <div className='sticky top-0 z-[1] bg-background pt-6'>
        <span>Pick a branch and a file in that branch to get this file codeowners.</span>

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

        <div className='flex gap-2 justify-between mt-2 mb-6'>
          <VirtualizedCombobox
            options={branchFilesOptions}
            selectedOption={selectedFileOption}
            selectedChanged={setSelectedFileOption}
            searchPlaceholder='Select file ...'
            disabled={filesResponse.status !== 'success'}
            height='600px'
            className='min-w-[600px]'
          />
        </div>
      </div>

      {fileCodeownersResponse.status === 'pending' && <div>Calculating codeowners...</div>}
      {fileCodeownersResponse.status === 'error' && <div>Calculating codeowners error</div>}
      {fileCodeownersResponse.data && (
        <div className='flex flex-col gap-2'>
          <div className='flex gap-2 justify-between items-center'>
            <span>Codeowners file:</span>{' '}
            <span className='text-sm'>
              Codeowners updated at {dayjs(fileCodeownersResponse.dataUpdatedAt).format('HH:mm:ss')}
            </span>
          </div>
          <div className='w-full p-4 rounded-md border overflow-auto relative'>
            <Tooltip content='Update codeowners'>
              <Button
                className='absolute top-2 right-2'
                variant='ghost'
                size='icon'
                loading={fileCodeownersResponse.fetchStatus === 'fetching'}
                onClick={
                  fileCodeownersResponse.fetchStatus === 'idle' ? updateFileCodeowners : undefined
                }
              >
                <RefreshIcon className='[animation-duration:2500ms]' />
              </Button>
            </Tooltip>
            <pre className='text-sm text-neutral-900 dark:text-neutral-400'>
              {fileCodeownersResponse.data}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
