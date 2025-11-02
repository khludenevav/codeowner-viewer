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
  allEntitiesSet: Set<string>;
  filteredEntities: Set<string> | null;
  setFilteredEntities: React.Dispatch<React.SetStateAction<Set<string> | null>>;
  entityName: string;
};

export const Filter: React.FC<Props> = ({
  allEntitiesSet,
  filteredEntities,
  setFilteredEntities,
  entityName,
}) => {
  const [localFilterValue, setLocalFilterValue] = useState('');
  const [localFilteredEntities, setLocalFilteredEntities] = useState<Set<string>>(
    new Set<string>(),
  );
  const allSortedEntities = useMemo(
    () => Array.from(allEntitiesSet.values()).sort(),
    [allEntitiesSet],
  );
  const initLocalState = useCallback(() => {
    setLocalFilteredEntities(new Set(filteredEntities ?? allSortedEntities));
    setLocalFilterValue('');
  }, [allSortedEntities, filteredEntities]);

  const applyChanges = useCallback(() => {
    setFilteredEntities(
      localFilteredEntities.size === 0 || localFilteredEntities.size === allSortedEntities.length
        ? null
        : new Set(localFilteredEntities),
    );
  }, [allSortedEntities.length, localFilteredEntities, setFilteredEntities]);

  const filteredEntityRows = useMemo(() => {
    const search = localFilterValue.toLowerCase();
    return allSortedEntities.filter(o => o.toLowerCase().includes(search));
  }, [allSortedEntities, localFilterValue]);
  const toggleSelection = useCallback((entity: string) => {
    setLocalFilteredEntities(prev => {
      const localFilteredEntities = new Set<string>(prev);
      if (!prev.has(entity)) {
        localFilteredEntities.add(entity);
      } else {
        localFilteredEntities.delete(entity);
      }
      return localFilteredEntities;
    });
  }, []);

  const onFilterChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setLocalFilterValue(e.target.value);
  }, []);

  function getMasterCheckBoxState(): CheckedState {
    if (localFilteredEntities.size > 0) {
      if (localFilteredEntities.size === allSortedEntities.length) {
        return true;
      }
      return 'indeterminate';
    }
    return false;
  }

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button
          disabled={!allSortedEntities.length}
          variant={
            filteredEntities === null || filteredEntities.size === 0 ? 'outline' : 'secondary'
          }
          onClick={initLocalState}
        >
          {filteredEntities === null || filteredEntities.size === 0
            ? `Filter by ${entityName}...`
            : `${filteredEntities.size}/${allSortedEntities.length} ${entityName} filtered`}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Filter repo tree by {entityName}</DialogTitle>
        </DialogHeader>
        <div className='h-[70vh] flex flex-col'>
          <div className='my-4 mx-1.5 flex gap-2'>
            <Input
              type='search'
              placeholder='Filter by extension...'
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
              <TableCaption>A list of repository file extensions</TableCaption>
              <TableHeader>
                <TableRow>
                  <TableHead className='w-[30px]'>
                    <Checkbox
                      checked={getMasterCheckBoxState()}
                      onCheckedChange={newState => {
                        if (newState) {
                          setLocalFilteredEntities(new Set(allSortedEntities));
                        } else {
                          setLocalFilteredEntities(new Set());
                        }
                      }}
                    />
                  </TableHead>
                  <TableHead>File extension</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredEntityRows.map(entity => {
                  return (
                    <TableRow key={entity} onClick={() => toggleSelection(entity)}>
                      <TableCell>
                        <Checkbox
                          onClick={e => {
                            // or else TableRow will be called after onCheckedChange and reset checkbox state back
                            e.stopPropagation();
                          }}
                          checked={localFilteredEntities.has(entity)}
                          onCheckedChange={() => toggleSelection(entity)}
                        />
                      </TableCell>
                      <TableCell>{entity}</TableCell>
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
