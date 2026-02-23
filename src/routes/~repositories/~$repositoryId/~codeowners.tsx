import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useEffect, useMemo, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { useBranchCodeowners, useUpdateBranchCodeowners } from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import { makeBranchOptions, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { dayjs } from '@/utils/dayjs';
import { Tooltip } from '@/components/ui/tooltip';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import { X } from 'lucide-react';

export const Route = createFileRoute('/repositories/$repositoryId/codeowners')({
  component: Codeowners,
});

function Codeowners() {
  const [branchOptions, setBranchOptions] = useState<ComboboxOption[]>([]);
  const [selectedBranchOption, setSelectedBranchOption] = useState<ComboboxOption | null>(null);
  const [ownerFilter, setOwnerFilter] = useState('');
  const [fileFilter, setFileFilter] = useState('');
  const [ownerFilterDebounced, setOwnerFilterDebounced] = useState('');
  const [fileFilterDebounced, setFileFilterDebounced] = useState('');
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;

  const branchesResponse = useBranches();
  const updateBranchesList = useUpdateBranches();

  const normalizedSelectedBranch = selectedBranchOption?.value ?? null;
  const branchCodeownersResponse = useBranchCodeowners(normalizedSelectedBranch);
  const updateBranchCodeowners = useUpdateBranchCodeowners(normalizedSelectedBranch);
  const branchCodeownersResponseData = branchCodeownersResponse.data;
  const filteredData = useMemo(() => {
    if (!branchCodeownersResponseData) {
      return branchCodeownersResponseData;
    }

    const ownerRe = ownerFilterDebounced ? new RegExp(`.*${ownerFilterDebounced}.*`, 'i') : null;
    const fileRe = fileFilterDebounced ? new RegExp(`.*${fileFilterDebounced}.*`, 'i') : null;

    const result = new Map<string, string[]>();
    for (const [owner, files] of branchCodeownersResponseData.entries()) {
      if (ownerRe && !ownerRe.test(owner)) {
        continue;
      }
      const filteredFiles = fileRe ? files.filter(f => fileRe.test(f)) : files;
      result.set(owner, filteredFiles);
    }
    return result;
  }, [branchCodeownersResponseData, ownerFilterDebounced, fileFilterDebounced]);

  useEffect(() => {
    const timer = setTimeout(() => {
      setOwnerFilterDebounced(ownerFilter.length >= 2 ? ownerFilter : '');
    }, 300);
    return () => clearTimeout(timer);
  }, [ownerFilter]);

  useEffect(() => {
    const timer = setTimeout(() => {
      setFileFilterDebounced(fileFilter.length >= 2 ? fileFilter : '');
    }, 300);
    return () => clearTimeout(timer);
  }, [fileFilter]);

  useEffect(() => {
    if (branchesResponse.status === 'success') {
      const { branches, headOption } = makeBranchOptions(branchesResponse.data);
      setBranchOptions(branches);
      setOwnerFilter('');
      setFileFilter('');
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
    <div className='flex flex-col mx-6 mb-6 max-h-full'>
      <div className='sticky top-0 z-[1] bg-background pt-6'>
        <span>
          Pick a branch name to get the codeowners for changed files comparing with 'main' branch.
        </span>

        <div className='flex gap-2 justify-between mt-2 mb-6'>
          <div className='flex gap-2'>
            <VirtualizedCombobox
              options={branchOptions}
              selectedOption={selectedBranchOption}
              selectedChanged={setSelectedBranchOption}
              searchPlaceholder='Select branch ...'
              height='400px'
              disabled={branchesResponse.status !== 'success'}
            />

            <div className='relative w-48'>
              <Input
                placeholder='Filter owners...'
                value={ownerFilter}
                onChange={e => setOwnerFilter(e.target.value)}
                className={ownerFilter ? 'pr-7' : ''}
                autoComplete='off'
              />
              {ownerFilter && (
                <button
                  className='absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground'
                  onClick={() => setOwnerFilter('')}
                >
                  <X size={14} />
                </button>
              )}
            </div>

            <div className='relative w-48'>
              <Input
                placeholder='Filter files...'
                value={fileFilter}
                onChange={e => setFileFilter(e.target.value)}
                className={fileFilter ? 'pr-7' : ''}
                autoComplete='off'
              />
              {fileFilter && (
                <button
                  className='absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground'
                  onClick={() => setFileFilter('')}
                >
                  <X size={14} />
                </button>
              )}
            </div>
          </div>
          <div className='flex gap-2 items-center flex-shrink-0'>
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
      </div>
      {branchCodeownersResponse.status === 'pending' && <div>Calculating codeowners...</div>}
      {branchCodeownersResponse.status === 'error' && <div>Calculating codeowners error</div>}
      {filteredData && (
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
              {JSON.stringify(Object.fromEntries(filteredData.entries()), null, 2)}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
