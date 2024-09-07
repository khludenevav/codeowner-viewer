import { ExpandDirButton } from '@/components/custom/expand-dir-button';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import { Button } from '@/components/ui/button';
import { Tooltip } from '@/components/ui/tooltip';
import { DirectoryOwners } from '@/utils/all-owners';
import { dayjs } from '@/utils/dayjs';
import { FetchStatus } from '@tanstack/react-query';
import { useWindowVirtualizer } from '@tanstack/react-virtual';
import { UnfoldVertical, FoldVertical } from 'lucide-react';
import React, { useCallback, useMemo, useRef, useState } from 'react';

type Row = {
  isFile: boolean;
  /** Dir or file */
  name: string;
  fullName: string;
  /** List of owners */
  owner: string;
  /** 0, 1, 2 ... */
  indent: number;
  expanded: boolean; // for directory means expanded, for files always false
};

type Props = {
  root: DirectoryOwners;
  dataUpdatedAt: number;
  allCodeownersResponseFetchStatus: FetchStatus;
  updateAllCodeowners: () => void;
};

export const OwnersTree: React.FC<Props> = ({
  root,
  dataUpdatedAt,
  allCodeownersResponseFetchStatus,
  updateAllCodeowners,
}) => {
  const treeRef = useRef<HTMLDivElement>(null);
  const [expandedDirectoriesSet, setExpandedDirectoriesSet] = useState(new Set<string>());

  const rows: Row[] = useMemo(() => {
    const rows: Row[] = [];
    addRows(rows, root, -1, '', fillName => expandedDirectoriesSet.has(fillName));
    return rows;
  }, [expandedDirectoriesSet, root]);

  const virtualizer = useWindowVirtualizer({
    count: rows.length,
    estimateSize: () => 24,
    overscan: 0,
    scrollMargin: treeRef.current?.offsetTop ?? 0,
    getItemKey: index => rows[index].fullName,
  });
  const virtualOptions = virtualizer.getVirtualItems();
  const onExpandClick = (fullName: string, expanded: boolean) => {
    setExpandedDirectoriesSet(prev => {
      const isItemExpanded = prev.has(fullName);
      if (expanded && !isItemExpanded) {
        const newSet = new Set<string>(prev);
        newSet.add(fullName);
        return newSet;
      }
      if (!expanded && isItemExpanded) {
        const newSet = new Set<string>(prev);
        newSet.delete(fullName);
        return newSet;
      }
      return prev;
    });
  };

  const onCollapseAllClick = useCallback(() => {
    setExpandedDirectoriesSet(new Set());
  }, []);

  const onExpandAllClick = useCallback(() => {
    const rows: Row[] = [];
    addRows(rows, root, -1, '', () => true);

    const newExpandedDirectoriesSet = new Set<string>();
    rows.forEach(row => {
      if (!row.isFile) {
        newExpandedDirectoriesSet.add(row.fullName);
      }
    });
    setExpandedDirectoriesSet(newExpandedDirectoriesSet);
  }, [root]);

  return (
    <div className='flex flex-col'>
      <div className='flex gap-2 items-center'>
        <Button variant='ghost' size='icon' onClick={onCollapseAllClick}>
          <FoldVertical />
        </Button>
        <Button variant='ghost' size='icon' onClick={onExpandAllClick}>
          <UnfoldVertical />
        </Button>

        <div className='flex gap-2 items-start ml-auto'>
          <Tooltip content='Update codeowners tree'>
            <Button
              variant='ghost'
              size='icon'
              loading={allCodeownersResponseFetchStatus === 'fetching'}
              onClick={
                allCodeownersResponseFetchStatus === 'idle' ? updateAllCodeowners : undefined
              }
            >
              <RefreshIcon className='[animation-duration:2500ms]' />
            </Button>
          </Tooltip>
          <span className='text-sm'>
            Tree updated
            <br />
            at {dayjs(dataUpdatedAt).format('HH:mm:ss')}
          </span>
        </div>
      </div>

      <div
        ref={treeRef}
        className='mt-2 -mx-2'
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          // width: '100%',
          position: 'relative',
        }}
      >
        {virtualOptions.map(item => {
          const row = rows[item.index];
          return (
            <div
              key={item.key}
              className='flex gap-4 items-center dark:hover:bg-slate-800 hover:bg-zinc-200 rounded-md px-2'
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${item.size}px`,
                transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)`,
              }}
            >
              <ExpandDirButton
                expanded={expandedDirectoriesSet.has(row.fullName)}
                onChange={newExpanded => onExpandClick(row.fullName, newExpanded)}
                style={{ marginLeft: `${16 * row.indent}px` }}
                className={row.isFile ? 'invisible' : undefined}
              />
              <Tooltip content={row.fullName}>
                <span>{row.name}</span>
              </Tooltip>
              <div className='ml-auto flex gap-3'>
                <span>{row.owner}</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

/**
 * Note, rows mutable.
 * @returns string meaning all files of this directory have the same codeowners. Probably empty
 *   undefined in other case
 */
function addRows(
  rows: Row[],
  current: DirectoryOwners,
  indent: number,
  partialPath: string,
  isDirectoryExpanded: (fullName: string) => boolean,
): string | undefined {
  const isRoot = !current.name;
  const newPartialPath = partialPath ? `${partialPath}/${current.name}` : current.name;
  const isDirExpanded = isRoot ? true : isDirectoryExpanded(newPartialPath);
  let currentDirectoryItem: null | Row = null;
  if (!isRoot) {
    currentDirectoryItem = {
      isFile: false,
      name: current.name,
      fullName: newPartialPath,
      owner: current.owner ?? '',
      indent,
      expanded: isDirExpanded,
    };
    rows.push(currentDirectoryItem);
  }

  // Null means no owners set yet, undefined means multiple owners, string means all owners is same (probably empty)
  let owners: null | undefined | string = null;
  const updateOwners = (subOwners: string | undefined): string | undefined => {
    if (owners === undefined) {
      return undefined; // Multiple owners already. Leave as is
    }
    if (owners === null) {
      return subOwners;
    }
    if (subOwners === undefined) {
      return undefined; // some subfolder has multiple owners
    }
    return owners === subOwners ? owners : undefined;
  };
  current.directories.forEach(dir => {
    const subDirOwners = addRows(
      // In case isDirExpanded===false we don't need to put result to rows. We only need an subDirOwners
      isDirExpanded ? rows : [],
      dir,
      indent + 1,
      newPartialPath,
      isDirectoryExpanded,
    );
    owners = updateOwners(subDirOwners);
  });

  current.files.forEach(file => {
    if (isDirExpanded) {
      rows.push({
        isFile: true,
        name: file.name,
        fullName: `${newPartialPath}/${file.name}`,
        owner: file.owner,
        indent: indent + 1,
        expanded: false,
      });
    }
    owners = updateOwners(file.owner ? file.owner : undefined); // handle empty owner
  });

  if (currentDirectoryItem) {
    currentDirectoryItem.owner = owners ? owners : '';
  }
  return owners ? owners : undefined;
}
