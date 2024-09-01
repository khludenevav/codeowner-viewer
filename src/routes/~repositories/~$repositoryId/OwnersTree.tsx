import { RefreshIcon } from '@/components/icons/refresh-icon';
import { Button } from '@/components/ui/button';
import { Tooltip } from '@/components/ui/tooltip';
import { DirectoryOwners } from '@/utils/all-owners';
import { dayjs } from '@/utils/dayjs';
import { FetchStatus } from '@tanstack/react-query';
import { useWindowVirtualizer } from '@tanstack/react-virtual';
import React, { useMemo, useRef } from 'react';

type Row = {
  isFile: boolean;
  /** Dir or file */
  name: string;
  fullName: string;
  /** List of owners */
  owner: string;
  /** 0, 1, 2 ... */
  indent: number;
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

  const rows: Row[] = useMemo(() => {
    const rows: Row[] = [];
    addRows(rows, root, -1, '');
    return rows;
  }, [root]);

  const virtualizer = useWindowVirtualizer({
    count: rows.length,
    estimateSize: () => 32,
    overscan: 0,
    scrollMargin: treeRef.current?.offsetTop ?? 0,
  });
  const virtualOptions = virtualizer.getVirtualItems();

  return (
    <div className='flex flex-col'>
      <div className='flex gap-2 items-center self-end'>
        <Tooltip content='Update codeowners tree'>
          <Button
            variant='ghost'
            size='icon'
            loading={allCodeownersResponseFetchStatus === 'fetching'}
            onClick={allCodeownersResponseFetchStatus === 'idle' ? updateAllCodeowners : undefined}
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

      <div
        ref={treeRef}
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualOptions.map(item => {
          const row = rows[item.index];
          return (
            <div
              key={item.key}
              className='flex justify-between'
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${item.size}px`,
                transform: `translateY(${item.start - virtualizer.options.scrollMargin}px)`,
              }}
            >
              <Tooltip content={row.fullName}>
                <div style={{ marginLeft: `${16 * row.indent}px` }}>{row.name}</div>
              </Tooltip>
              <div>{row.owner}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

/** Note, rows mutable */
function addRows(rows: Row[], current: DirectoryOwners, indent: number, partialPath: string): void {
  const isRoot = !current.name;
  const newPartialPath = partialPath ? `${partialPath}/${current.name}` : current.name;
  if (!isRoot) {
    rows.push({
      isFile: false,
      name: current.name,
      fullName: newPartialPath,
      owner: current.owner ?? '',
      indent,
    });
  }

  current.directories.forEach(dir => {
    addRows(rows, dir, indent + 1, newPartialPath);
  });

  current.files.forEach(file => {
    rows.push({
      isFile: true,
      name: file.name,
      fullName: newPartialPath,
      owner: file.owner,
      indent: indent + 1,
    });
  });
}
