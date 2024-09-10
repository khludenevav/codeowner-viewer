import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useEffect, useMemo, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { ComboboxOption, VirtualizedCombobox } from '@/components/ui/virtual-combobox';
import { makeBranchOptions, useBranches, useUpdateBranches } from '@/utils/get-branches';
import { Button } from '@/components/ui/button';
import { dayjs } from '@/utils/dayjs';
import { Tooltip } from '@/components/ui/tooltip';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import {
  DirectoryOwners,
  FileOwners,
  useAllCodeowners,
  useUpdateAllCodeowners,
} from '@/utils/all-owners';
import { OwnersTree } from './OwnersTree';
import { UseQueryResult } from '@tanstack/react-query';
import { OwnersFilter } from './OwnersFilter';

export const Route = createFileRoute('/repositories/$repositoryId/all-owners')({
  component: Codeowners,
});

function splitToOwners(owners: string | null): string[] {
  return owners ? owners.replace(',', '').split(' ') : [];
}

function useFilteredRoot(
  allCodeownersResponse: UseQueryResult<DirectoryOwners | null, Error>,
  filteredOwners: Set<string> | null,
): DirectoryOwners | null {
  return useMemo(() => {
    if (allCodeownersResponse.status !== 'success' || !allCodeownersResponse.data) {
      return null;
    }
    const root = allCodeownersResponse.data;
    if (filteredOwners === null) {
      return root;
    }
    const filterFiles = (files: FileOwners[]): FileOwners[] => {
      return files.filter(f => splitToOwners(f.owner).some(o => filteredOwners.has(o)));
    };
    /** @return null when it is empty (no files and no dirs) */
    const filterDir = (dir: DirectoryOwners): DirectoryOwners | null => {
      const files = filterFiles(dir.files);
      const directories = filterDirs(dir.directories);
      return files.length || directories.length
        ? {
            ...dir,
            files,
            directories,
          }
        : null;
    };
    /** @return no empty directories */
    const filterDirs = (directories: DirectoryOwners[]): DirectoryOwners[] => {
      return directories.map(d => filterDir(d)).filter(d => !!d);
    };

    return { ...root, directories: filterDirs(root.directories), files: filterFiles(root.files) };
  }, [allCodeownersResponse.data, allCodeownersResponse.status, filteredOwners]);
}

function Codeowners() {
  const [branchOptions, setBranchOptions] = useState<ComboboxOption[]>([]);
  const [selectedBranchOption, setSelectedBranchOption] = useState<ComboboxOption | null>(null);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;
  const normalizedSelectedBranch = selectedBranchOption?.value ?? null;

  const allCodeownersResponse = useAllCodeowners(normalizedSelectedBranch);
  const updateAllCodeowners = useUpdateAllCodeowners(normalizedSelectedBranch);
  const branchesResponse = useBranches();

  const updateBranchesList = useUpdateBranches();
  /** null means all selected */
  const [filteredOwners, setFilteredOwners] = useState<Set<string> | null>(null);
  const allOwnersSet: Set<string> = useMemo(() => {
    const result: Set<string> = new Set();
    if (allCodeownersResponse.status === 'success' && allCodeownersResponse.data) {
      const addForDirectory = (dir: DirectoryOwners) => {
        splitToOwners(dir.owner).forEach(o => result.add(o));
        dir.directories.forEach(d => addForDirectory(d));
        dir.files.forEach(file => {
          splitToOwners(file.owner).forEach(o => result.add(o));
        });
      };
      addForDirectory(allCodeownersResponse.data);
    }
    return result;
  }, [allCodeownersResponse.data, allCodeownersResponse.status]);

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

  const filteredRoot = useFilteredRoot(allCodeownersResponse, filteredOwners);

  if (!appConfig) {
    return 'Loading app config...';
  }

  if (appConfig.repositories.length === 0) {
    return <Navigate to='/settings' />;
  }

  return (
    <div className='flex flex-col'>
      <span>Pick a branch and explore owners in tree view format for each file of the repo.</span>
      <div className='flex gap-2 justify-between mt-2 mb-2'>
        <div className='flex gap-2'>
          <VirtualizedCombobox
            options={branchOptions}
            selectedOption={selectedBranchOption}
            selectedChanged={setSelectedBranchOption}
            searchPlaceholder='Select branch ...'
            height='400px'
            disabled={branchesResponse.status !== 'success'}
          />

          {allOwnersSet.size > 0 && (
            <OwnersFilter
              allOwnersSet={allOwnersSet}
              filteredOwners={filteredOwners}
              setFilteredOwners={setFilteredOwners}
            />
          )}
        </div>

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
      <div className='mt-4'>
        {allCodeownersResponse.status === 'pending' && (
          <div>
            Calculating codeowners tree... There is lack of optimizations, so it can take a quite
            long time to calculate codeowners for all repository files. For my mac M1 it takes
            around 50 seconds per 100k files and codeowner file size 3k lines.
          </div>
        )}
        {allCodeownersResponse.status === 'error' && <div>Calculating codeowners tree error</div>}
        {allCodeownersResponse.status === 'success' && !allCodeownersResponse.data && (
          <div>Calculating codeowners tree done, but list is empty</div>
        )}
        {allCodeownersResponse.status === 'success' &&
          allCodeownersResponse.data &&
          filteredRoot && (
            <OwnersTree
              root={filteredRoot}
              dataUpdatedAt={allCodeownersResponse.dataUpdatedAt}
              updateAllCodeowners={updateAllCodeowners}
              allCodeownersResponseFetchStatus={allCodeownersResponse.fetchStatus}
            />
          )}
      </div>
    </div>
  );
}
