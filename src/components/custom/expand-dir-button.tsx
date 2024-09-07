import { cn } from '@/utils/components-utils';
import { LucideChevronRight } from 'lucide-react';
import React, { useCallback } from 'react';

type Props = {
  expanded: boolean;
  onChange: (value: boolean) => void;
  style?: React.CSSProperties | undefined;
  className?: string;
};

export const ExpandDirButton: React.FC<Props> = ({ expanded, onChange, className, style }) => {
  const onClick = useCallback(() => {
    onChange(!expanded);
  }, [expanded, onChange]);

  return (
    <LucideChevronRight
      style={style}
      onClick={onClick}
      className={cn('h-5 w-5 duration-300', expanded && 'rotate-90', className)}
    />
  );
};
