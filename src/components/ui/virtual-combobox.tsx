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
import { cn } from '@/utils/components-utils';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Check, ChevronsUpDown } from 'lucide-react';
import * as React from 'react';

function generateRandomStrings() {
  const items = new Set<string>();
  while (items.size < 20000) {
    // const randomString = Math.random().toString(36).substr(2, 10);
    // const times = Math.max(5, Math.floor(Math.random() * 8));
    // items.add(randomString.repeat(times));
    items.add(items.size.toString());
  }
  return Array.from(items);
}

const initialOptions: Option[] = generateRandomStrings().map(option => ({
  value: option,
  label: option,
}));

type Option = {
  value: string;
  label: string;
};

interface VirtualizedCommandProps {
  height: string;
  options: Option[];
  placeholder: string;
  selectedOption: string;
  onSelectOption?: (option: string) => void;
}

const VirtualizedCommand = ({
  height,
  options,
  placeholder,
  selectedOption,
  onSelectOption,
}: VirtualizedCommandProps) => {
  const [filteredOptions, setFilteredOptions] = React.useState<Option[]>(options);
  const parentRef = React.useRef(null);

  const virtualizer = useVirtualizer({
    count: filteredOptions.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 32,
    overscan: 0,
  });

  const virtualOptions = virtualizer.getVirtualItems();

  const handleSearch = (search: string) => {
    setFilteredOptions(
      options.filter(option => option.value.toLowerCase().includes(search.toLowerCase() ?? [])),
    );
  };

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      // event.preventDefault();
    }
  };

  return (
    <Command shouldFilter={false} onKeyDown={handleKeyDown}>
      <CommandInput onValueChange={handleSearch} placeholder={placeholder} />
      <CommandList>
        <CommandEmpty>No item found.</CommandEmpty>
        <CommandGroup
          ref={parentRef}
          style={{
            height: height,
            width: '100%',
            overflow: 'auto',
          }}
        >
          <div
            style={{
              minHeight: `${virtualizer.getTotalSize()}px`,
            }}
          >
            {virtualOptions.length > 0 && (
              // allow place items using non-absolute position
              <div
                style={{
                  width: '1px',
                  height: `${virtualOptions[0].start}px`,
                }}
              />
            )}

            {virtualOptions.map(virtualOption => (
              <CommandItem
                key={filteredOptions[virtualOption.index].value}
                value={filteredOptions[virtualOption.index].value}
                onSelect={onSelectOption}
              >
                <Check
                  className={cn(
                    'mr-2 h-4 min-w-6',
                    selectedOption === filteredOptions[virtualOption.index].value
                      ? 'opacity-100'
                      : 'opacity-0',
                  )}
                />
                {filteredOptions[virtualOption.index].label}
              </CommandItem>
            ))}
          </div>
        </CommandGroup>
      </CommandList>
    </Command>
  );
};

interface VirtualizedComboboxProps {
  options?: Option[];
  searchPlaceholder?: string;
  width?: string;
  height?: string;
}
/** Button is a trigger which opens popup with a input filter and virtualized list of options */
export function VirtualizedCombobox({
  options = initialOptions,
  searchPlaceholder = 'Search items...',
  width = '400px',
  height = '400px',
}: VirtualizedComboboxProps) {
  const [open, setOpen] = React.useState<boolean>(false);
  const [selectedOption, setSelectedOption] = React.useState<string>('');

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant='outline'
          role='combobox'
          aria-expanded={open}
          className='justify-between'
          style={{
            width: width,
          }}
        >
          {selectedOption
            ? options.find(option => option.value === selectedOption)?.label ?? ''
            : searchPlaceholder}
          <ChevronsUpDown className='ml-2 h-4 w-4 shrink-0 opacity-50' />
        </Button>
      </PopoverTrigger>
      <PopoverContent className='min-w-[--radix-popover-trigger-width] p-0'>
        <VirtualizedCommand
          height={height}
          options={options}
          placeholder={searchPlaceholder}
          selectedOption={selectedOption}
          onSelectOption={currentValue => {
            setSelectedOption(currentValue === selectedOption ? '' : currentValue);
            setOpen(false);
          }}
        />
      </PopoverContent>
    </Popover>
  );
}
