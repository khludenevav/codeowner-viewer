import { cn } from '@/utils/components-utils';

// Also take a look at https://mui.com/material-ui/react-skeleton/
function Skeleton({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn('animate-pulse rounded-md bg-slate-100', className)} {...props} />;
}

export { Skeleton };
