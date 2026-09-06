import { createFileRoute, Navigate } from '@tanstack/react-router';

import { Fragment, useEffect, useMemo, useState } from 'react';

import {
  type BranchFile,
  useBranchCodeowners,
  useUpdateBranchCodeowners,
} from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { useCurrentRepository } from '../../../app-config/useCurrentRepository';
import { Repositories } from '../../../app-config/app-config';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import { makeBranchOptions, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { dayjs } from '@/utils/dayjs';
import { Tooltip } from '@/components/ui/tooltip';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import { X } from 'lucide-react';
import { useRepositoryPageState } from '@/utils/hooks/useRepositoryPageState';

export const Route = createFileRoute('/repositories/$repositoryId/codeowners')({
  component: CodeownersRoute,
});

function CodeownersOutput({
  data,
  showComments,
}: {
  data: Map<string, BranchFile[]>;
  showComments: boolean;
}) {
  const entries = Array.from(data.entries());
  return (
    <pre className='text-sm text-neutral-900 dark:text-neutral-400'>
      {'{\n'}
      {entries.map(([owner, files], ownerIdx) => (
        <Fragment key={owner || '__unowned__'}>
          {`  "${owner}": [\n`}
          {files.map((file, fileIdx) => (
            <span key={file.path}>
              {file.uncommitted ? (
                <span
                  className='select-none text-amber-700 dark:text-[#e2c08d]'
                  title='Uncommitted (working-tree) change'
                >
                  {'    • '}
                </span>
              ) : (
                '      '
              )}
              {`"${file.path}"${fileIdx < files.length - 1 ? ',' : ''}`}
              {showComments && file.comment && (
                <span className='select-none text-amber-600 dark:text-amber-400'>
                  {'  '}
                  {file.comment}
                </span>
              )}
              {'\n'}
            </span>
          ))}
          {`  ]${ownerIdx < entries.length - 1 ? ',' : ''}\n`}
        </Fragment>
      ))}
      {'}'}
    </pre>
  );
}

function CodeownersRoute() {
  const appConfigResponse = useAppConfig();
  const currentRepository = useCurrentRepository();

  if (!appConfigResponse.data) {
    return 'Loading app config...';
  }

  if (currentRepository.status === 'no-repositories') {
    return <Navigate to='/' />;
  }

  if (currentRepository.status === 'not-found') {
    return (
      <Navigate
        to='/repositories/$repositoryId/codeowners'
        params={{ repositoryId: appConfigResponse.data.repositories[0].id }}
      />
    );
  }

  if (currentRepository.status !== 'ready') {
    return null;
  }

  return (
    <Codeowners key={currentRepository.repository.id} repository={currentRepository.repository} />
  );
}

function Codeowners({ repository }: { repository: Repositories }) {
  const [persistedBranchOption, setSelectedBranchOption] =
    useRepositoryPageState<ComboboxOption | null>('codeowners.selectedBranchOption', null);
  const [ownerFilter, setOwnerFilter] = useRepositoryPageState<string>(
    'codeowners.ownerFilter',
    '',
  );
  const [fileFilter, setFileFilter] = useRepositoryPageState<string>('codeowners.fileFilter', '');
  const [ownerFilterDebounced, setOwnerFilterDebounced] = useState(() =>
    ownerFilter.length >= 2 ? ownerFilter : '',
  );
  const [fileFilterDebounced, setFileFilterDebounced] = useState(() =>
    fileFilter.length >= 2 ? fileFilter : '',
  );
  const [showComments, setShowComments] = useRepositoryPageState<boolean>(
    'codeowners.showComments',
    false,
  );
  const [onlyWithComments, setOnlyWithComments] = useRepositoryPageState<boolean>(
    'codeowners.onlyWithComments',
    false,
  );
  const [includeUncommittedPref, setIncludeUncommittedPref] = useRepositoryPageState<boolean>(
    'codeowners.includeUncommitted',
    true,
  );

  const branchesResponse = useBranches(repository);
  const updateBranchesList = useUpdateBranches(repository);

  const { branches: branchOptions, headOption } = useMemo(
    () =>
      branchesResponse.status === 'success'
        ? makeBranchOptions(branchesResponse.data)
        : { branches: [] as ComboboxOption[], headOption: null as ComboboxOption | null },
    [branchesResponse.data, branchesResponse.status],
  );
  const selectedBranchOption = persistedBranchOption ?? headOption;

  const normalizedSelectedBranch = selectedBranchOption?.value ?? null;
  const currentBranch = branchesResponse.data?.current ?? null;
  const isCurrentBranchSelected =
    !!currentBranch && !!normalizedSelectedBranch && currentBranch === normalizedSelectedBranch;
  const effectiveIncludeUncommitted = isCurrentBranchSelected && includeUncommittedPref;
  const branchCodeownersResponse = useBranchCodeowners(
    repository,
    normalizedSelectedBranch,
    effectiveIncludeUncommitted,
  );
  const updateBranchCodeowners = useUpdateBranchCodeowners(
    repository,
    normalizedSelectedBranch,
    effectiveIncludeUncommitted,
  );
  const branchCodeownersResponseData = branchCodeownersResponse.data;
  const filteredData = useMemo(() => {
    if (!branchCodeownersResponseData) {
      return branchCodeownersResponseData;
    }

    const ownerRe = ownerFilterDebounced ? new RegExp(`.*${ownerFilterDebounced}.*`, 'i') : null;
    const fileRe = fileFilterDebounced ? new RegExp(`.*${fileFilterDebounced}.*`, 'i') : null;

    const result = new Map(
      Array.from(branchCodeownersResponseData.entries())
        .filter(([owner]) => !ownerRe || ownerRe.test(owner))
        .map(([owner, files]) => [
          owner,
          files
            .filter(f => !fileRe || fileRe.test(f.path))
            .filter(f => !onlyWithComments || !!f.comment),
        ]),
    );
    return result;
  }, [branchCodeownersResponseData, ownerFilterDebounced, fileFilterDebounced, onlyWithComments]);

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

            <div className='flex flex-col justify-center gap-1'>
              <label className='flex items-center gap-1 text-xs cursor-pointer select-none'>
                <Checkbox
                  className='h-3.5 w-3.5'
                  checked={showComments}
                  onCheckedChange={v => {
                    const next = v === true;
                    setShowComments(next);
                    if (!next) setOnlyWithComments(false);
                  }}
                />
                Show comments
              </label>
              <label className='flex items-center gap-1 text-xs cursor-pointer select-none'>
                <Checkbox
                  className='h-3.5 w-3.5'
                  checked={onlyWithComments}
                  disabled={!showComments}
                  onCheckedChange={v => setOnlyWithComments(v === true)}
                />
                Only with comments
              </label>
            </div>

            <div className='flex flex-col justify-start gap-1 pt-1'>
              <label
                className={`flex items-center gap-1 text-xs select-none ${
                  isCurrentBranchSelected ? 'cursor-pointer' : 'cursor-not-allowed opacity-50'
                }`}
                title={
                  isCurrentBranchSelected
                    ? undefined
                    : 'Available only when the current (HEAD) branch is selected.'
                }
              >
                <Checkbox
                  className='h-3.5 w-3.5'
                  checked={includeUncommittedPref}
                  disabled={!isCurrentBranchSelected}
                  onCheckedChange={v => setIncludeUncommittedPref(v === true)}
                />
                Uncommitted changes
              </label>
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
            <CodeownersOutput data={filteredData} showComments={showComments} />
          </div>
        </div>
      )}
    </div>
  );
}
