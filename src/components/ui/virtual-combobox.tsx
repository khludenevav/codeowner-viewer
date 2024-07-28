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

export type ComboboxOption = {
  value: string;
  label: string;
};

interface VirtualizedCommandProps {
  options: ComboboxOption[];
  selectedOption: ComboboxOption | null;
  onSelectOption: (option: string) => void;
  placeholder: string;
  height: string;
}

const VirtualizedCommand = ({
  height,
  options,
  placeholder,
  selectedOption,
  onSelectOption,
}: VirtualizedCommandProps) => {
  const [filteredOptions, setFilteredOptions] = React.useState<ComboboxOption[]>(options);
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

  return (
    <Command shouldFilter={false}>
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
                    selectedOption === filteredOptions[virtualOption.index]
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
  options: ComboboxOption[];
  selectedOption: ComboboxOption | null;
  selectedChanged: (newSelectedOption: ComboboxOption) => void;
  searchPlaceholder?: string;
  disabled?: boolean;
  height?: string;
}
/** Button is a trigger which opens popup with a input filter and virtualized list of options */
export function VirtualizedCombobox({
  options,
  selectedOption,
  selectedChanged,
  searchPlaceholder = 'Search ...',
  disabled = false,
  height = '400px',
}: VirtualizedComboboxProps) {
  const [open, setOpen] = React.useState<boolean>(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant='outline'
          role='combobox'
          aria-expanded={open}
          disabled={disabled}
          className='justify-between'
        >
          {selectedOption ? selectedOption.label : searchPlaceholder}
          <ChevronsUpDown className='ml-2 h-4 w-4 shrink-0 opacity-50' />
        </Button>
      </PopoverTrigger>
      <PopoverContent className='min-w-[--radix-popover-trigger-width] p-0' align='start'>
        <VirtualizedCommand
          options={options}
          selectedOption={selectedOption}
          onSelectOption={currentValue => {
            selectedChanged(options.find(opt => opt.value === currentValue)!);
            setOpen(false);
          }}
          placeholder={searchPlaceholder}
          height={height}
        />
      </PopoverContent>
    </Popover>
  );
}
