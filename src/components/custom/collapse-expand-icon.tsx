import { cn } from '@/utils/components-utils';
import React from 'react';
import { UnfoldVertical, FoldVertical } from 'lucide-react';

type Props = {
  mode: 'collapse' | 'expand';
  onClick: () => void;
  className?: string;
};

export const CollapseExpandIcon: React.FC<Props> = ({ mode, onClick, className }) => {
  const Icon = mode === 'collapse' ? FoldVertical : UnfoldVertical;
  return (
    <Icon
      onClick={onClick}
      className={cn(
        'h-5 w-5 invisible group-hover:visible hover:cursor-pointer text-zinc-500 hover:text-zinc-800 dark:text-slate-500 dark:hover:text-slate-50',
        className,
      )}
    />
  );
};
