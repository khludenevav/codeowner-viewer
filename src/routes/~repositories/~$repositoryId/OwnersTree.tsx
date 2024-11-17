import { ExpandDirButton } from '@/components/custom/expand-dir-button';
import { CollapseExpandIcon } from '@/components/custom/collapse-expand-icon';
import { RefreshIcon } from '@/components/icons/refresh-icon';
import { Button } from '@/components/ui/button';
import { Tooltip } from '@/components/ui/tooltip';
import { DirectoryOwners } from '@/utils/all-owners';
import { dayjs } from '@/utils/dayjs';
import { FetchStatus } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { UnfoldVertical, FoldVertical, ArrowLeftRight, BadgeCheck } from 'lucide-react';
import React, { useCallback, useMemo, useRef, useState } from 'react';
import { cn } from '@/utils/components-utils';
import { getVSIFileIcon, getVSIFolderIcon } from 'file-extension-icon-js';

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

  const scrollContainer = document.getElementById('main'); // not very efficient. Ideally replace to ref
  const scrollMargin =
    treeRef.current != null && scrollContainer != null
      ? treeRef.current.offsetTop - scrollContainer.offsetTop
      : 0;
  const virtualizer = useVirtualizer({
    count: rows.length,
    estimateSize: () => 24,
    overscan: 7,
    getScrollElement: () => scrollContainer,
    scrollMargin,
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

  /** Collapse directory and it's child */
  const onCollapseAllDirectory = useCallback((fullName: string) => {
    setExpandedDirectoriesSet(prev => {
      const newExpandedDirectoriesSet = new Set<string>(prev);
      for (const currentFullName of prev.values()) {
        if (currentFullName.startsWith(fullName)) {
          newExpandedDirectoriesSet.delete(currentFullName);
        }
      }
      return newExpandedDirectoriesSet;
    });
  }, []);

  /** Collapse directory and all it's children */
  const onExpandAllDirectory = useCallback(
    (targetFullName: string) => {
      setExpandedDirectoriesSet(prev => {
        const newExpandedDirectoriesSet = new Set<string>(prev);
        addRows([], root, -1, '', fName => {
          let expanded = newExpandedDirectoriesSet.has(fName);
          if (!expanded) {
            if (fName.startsWith(targetFullName)) {
              newExpandedDirectoriesSet.add(fName);
              expanded = true;
            }
          }
          return expanded;
        });
        return newExpandedDirectoriesSet;
      });
    },
    [root],
  );

  return (
    <div className='flex flex-col'>
      <div className='flex gap-2 items-center'>
        <Tooltip content='Collapse all'>
          <Button variant='ghost' size='icon' onClick={onCollapseAllClick}>
            <FoldVertical />
          </Button>
        </Tooltip>
        <Tooltip content='Expand all'>
          <Button variant='ghost' size='icon' onClick={onExpandAllClick}>
            <UnfoldVertical />
          </Button>
        </Tooltip>

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
          position: 'relative',
        }}
      >
        {virtualOptions.map(item => {
          const row = rows[item.index];
          const expanded = expandedDirectoriesSet.has(row.fullName);
          const fileNameForIcon = row.fullName.split('/').at(-1)!;
          return (
            <div
              key={item.key}
              className='grid items-center group gap-6 grid-cols-2 hover:bg-zinc-200 dark:hover:bg-slate-800  rounded-md px-2'
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${item.size}px`,
                transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)`,
              }}
            >
              <div className='flex gap-3 items-center justify-end'>
                {!row.isFile && !row.owner ? (
                  <Tooltip
                    delayDuration={0}
                    content={
                      <>
                        Owners do not defined or are different only for some children of this
                        directory.
                        <br />
                        This does not mean an error. It just makes sense to look inside the folder
                        <br /> to check if the owners are defined everywhere.
                      </>
                    }
                  >
                    <ArrowLeftRight className='h-5 w-5 text-gray-300 dark:text-gray-600' />
                  </Tooltip>
                ) : (
                  <>
                    <span className='gap-3'>{row.owner}</span>
                    <Tooltip
                      delayDuration={0}
                      content={
                        row.isFile
                          ? ''
                          : 'These are owners for all children of this directory. It also means all sub-owners the same.'
                      }
                    >
                      <BadgeCheck
                        className={cn(
                          'h-5 w-5 text-green-300 dark:text-green-800',
                          row.isFile && 'invisible',
                        )}
                      />
                    </Tooltip>
                  </>
                )}
              </div>

              <div className='flex items-center'>
                {/* Just to adjust space for files */}
                {row.isFile && <div className='h-5 w-5 invisible' />}
                {!row.isFile &&
                  (expanded ? (
                    <Tooltip content="Expand directory and all it's children">
                      <CollapseExpandIcon
                        mode='collapse'
                        onClick={() => onCollapseAllDirectory(row.fullName)}
                      />
                    </Tooltip>
                  ) : (
                    <Tooltip content="Collapse directory and it's children">
                      <CollapseExpandIcon
                        mode='expand'
                        onClick={() => onExpandAllDirectory(row.fullName)}
                      />
                    </Tooltip>
                  ))}

                <ExpandDirButton
                  className={row.isFile ? 'invisible' : undefined}
                  expanded={expanded}
                  onChange={newExpanded => onExpandClick(row.fullName, newExpanded)}
                  style={{ marginLeft: `${6 + 16 * row.indent}px` }}
                />
                <img
                  className='ml-0.5 hover:cursor-pointer'
                  src={
                    // I prefer identical icons over different ones for directories
                    row.isFile ? getVSIFileIcon(fileNameForIcon) : getVSIFolderIcon('f', expanded)
                  }
                  onClick={
                    row.isFile ? undefined : () => onExpandClick(row.fullName, !row.expanded)
                  }
                  width='20px'
                />
                <Tooltip content={row.fullName} side='top' align='start'>
                  <span className='flex-grow ml-1'>{row.name}</span>
                </Tooltip>
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
