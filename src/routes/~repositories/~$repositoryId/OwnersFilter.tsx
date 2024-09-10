import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { CheckedState } from '@radix-ui/react-checkbox';
import React, { useCallback, useMemo, useState } from 'react';

type Props = {
  allOwnersSet: Set<string>;
  filteredOwners: Set<string> | null;
  setFilteredOwners: React.Dispatch<React.SetStateAction<Set<string> | null>>;
};

export const OwnersFilter: React.FC<Props> = ({
  allOwnersSet,
  filteredOwners,
  setFilteredOwners,
}) => {
  const [localFilterValue, setLocalFilterValue] = useState('');
  const [localFilteredOwners, setLocalFilteredOwners] = useState<Set<string>>(new Set<string>());
  const allSortedOwners = useMemo(() => Array.from(allOwnersSet.values()).sort(), [allOwnersSet]);
  const initLocalState = useCallback(() => {
    setLocalFilteredOwners(new Set(filteredOwners ?? allSortedOwners));
    setLocalFilterValue('');
  }, [allSortedOwners, filteredOwners]);

  const applyChanges = useCallback(() => {
    setFilteredOwners(
      localFilteredOwners.size === 0 || localFilteredOwners.size === allSortedOwners.length
        ? null
        : new Set(localFilteredOwners),
    );
  }, [allSortedOwners.length, localFilteredOwners, setFilteredOwners]);

  const filteredOwnerRows = useMemo(() => {
    const search = localFilterValue.toLowerCase();
    return allSortedOwners.filter(o => o.toLowerCase().includes(search));
  }, [allSortedOwners, localFilterValue]);
  const toggleOwnerState = useCallback((owner: string) => {
    setLocalFilteredOwners(prev => {
      const localFilteredOwners = new Set<string>(prev);
      if (!prev.has(owner)) {
        localFilteredOwners.add(owner);
      } else {
        localFilteredOwners.delete(owner);
      }
      return localFilteredOwners;
    });
  }, []);

  const onFilterChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setLocalFilterValue(e.target.value);
  }, []);

  function getMasterCheckBoxState(): CheckedState {
    if (localFilteredOwners.size > 0) {
      if (localFilteredOwners.size === allSortedOwners.length) {
        return true;
      }
      return 'indeterminate';
    }
    return false;
  }

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button disabled={!allSortedOwners.length} variant='outline' onClick={initLocalState}>
          {filteredOwners === null || filteredOwners.size === 0
            ? 'Filter by owner...'
            : `${filteredOwners.size}/${allSortedOwners.length} owners filtered`}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Filter repo tree by owners</DialogTitle>
        </DialogHeader>
        <div className='h-[70vh] flex flex-col'>
          <div className='my-4 mx-1.5 flex gap-2'>
            <Input
              type='search'
              placeholder='Filter by owner...'
              value={localFilterValue}
              onChange={onFilterChange}
              autoFocus
              autoComplete='off'
              spellCheck={false}
            />
            <Button variant='outline' onClick={() => setLocalFilterValue('')}>
              Clear filter
            </Button>
          </div>
          <div className='flex-1 overflow-y-auto'>
            <Table>
              <TableCaption>A list of repository codeowners.</TableCaption>
              <TableHeader>
                <TableRow>
                  <TableHead className='w-[30px]'>
                    <Checkbox
                      checked={getMasterCheckBoxState()}
                      onCheckedChange={newState => {
                        if (newState) {
                          setLocalFilteredOwners(new Set(allSortedOwners));
                        } else {
                          setLocalFilteredOwners(new Set());
                        }
                      }}
                    />
                  </TableHead>
                  <TableHead>Owner</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredOwnerRows.map(owner => {
                  return (
                    <TableRow key={owner} onClick={() => toggleOwnerState(owner)}>
                      <TableCell>
                        <Checkbox
                          onClick={e => {
                            // or else TableRow will be called after onCheckedChange and reset checkbox state back
                            e.stopPropagation();
                          }}
                          checked={localFilteredOwners.has(owner)}
                          onCheckedChange={() => toggleOwnerState(owner)}
                        />
                      </TableCell>
                      <TableCell>{owner}</TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button type='button' variant='secondary'>
              Cancel
            </Button>
          </DialogClose>

          <DialogClose asChild>
            <Button type='submit' onClick={applyChanges}>
              Apply
            </Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
