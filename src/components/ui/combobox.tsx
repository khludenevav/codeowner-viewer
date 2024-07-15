import { useVirtualizer } from '@tanstack/react-virtual';
import * as React from 'react';
import { Check, ChevronsUpDown } from 'lucide-react';
import { cn } from '@/utils/components-utils';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

const frameworks = Array.from({ length: 6000 }, (v, i) => ({
  value: `item_${i}`,
  label: `item_${i}`,
}));

export function ComboboxDemo() {
  const parentRef = React.useRef<HTMLDivElement>(null);
  const [open, setOpen] = React.useState(false);
  const [value, setValue] = React.useState('');

  const rowVirtualizer = useVirtualizer({
    count: frameworks.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 35,
  });

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant='outline'
          role='combobox'
          aria-expanded={open}
          className='w-[200px] justify-between'
        >
          {value
            ? frameworks.find(framework => framework.value === value)?.label
            : 'Select framework...'}
          <ChevronsUpDown className='ml-2 h-4 w-4 shrink-0 opacity-50' />
        </Button>
      </PopoverTrigger>
      <PopoverContent className='w-[200px] p-0'>
        <Command
        //   ref={parentRef}
        //   style={{
        //     height: `400px`,
        //     overflow: 'auto', // Make it scroll!
        //   }}
        >
          <CommandInput placeholder='Search framework...' />
          <CommandList>
            <CommandEmpty>No framework found.</CommandEmpty>
            <CommandGroup>
              {frameworks.map(framework => (
                <CommandItem
                  key={framework.value}
                  value={framework.value}
                  onSelect={currentValue => {
                    setValue(currentValue === value ? '' : currentValue);
                    setOpen(false);
                  }}
                >
                  <Check
                    className={cn(
                      'mr-2 h-4 w-4',
                      value === framework.value ? 'opacity-100' : 'opacity-0',
                    )}
                  />
                  {framework.label}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
