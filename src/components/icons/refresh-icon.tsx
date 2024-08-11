import { cn } from '@/utils/components-utils';
import { RefreshCw } from 'lucide-react';
import React from 'react';

const RefreshIcon: React.FC<{ className?: string }> = ({ className }) => {
  return <RefreshCw className={cn('h-5 w-5 group-data-[loading]:animate-spin', className)} />;
};
RefreshIcon.displayName = 'RefreshIcon';

export { RefreshIcon };
