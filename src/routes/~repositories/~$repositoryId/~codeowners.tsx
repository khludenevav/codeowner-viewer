import { createFileRoute, Navigate } from '@tanstack/react-router';

import { useCallback, useRef, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { getBranchDifference } from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@radix-ui/react-scroll-area';

export const Route = createFileRoute('/repositories/$repositoryId/codeowners')({
  component: Codeowners,
});
function Codeowners() {
  const branchInputRef = useRef<HTMLInputElement>(null);
  const [isLoading, setIsLoading] = useState<boolean>();
  const [lastOwners, setLastOwners] = useState<Map<string, string[]> | null>(null);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;

  const onGetChangesCodeowners = useCallback(async () => {
    const branch = branchInputRef.current?.value?.trim();
    if (!branch) {
      return;
    }
    const repository = appConfig?.repositories[0];
    if (repository) {
      setLastOwners(null);
      setIsLoading(true);
      try {
        const owners = await getBranchDifference(repository, branch);
        setLastOwners(owners);
      } finally {
        setIsLoading(false);
      }
    } else {
      console.log('No repo');
      setLastOwners(null);
    }
  }, [appConfig?.repositories]);

  const onGetChangesCodeownersForCurrent = useCallback(() => {
    if (branchInputRef.current) {
      branchInputRef.current.value = 'HEAD';
    }
    onGetChangesCodeowners();
  }, [onGetChangesCodeowners]);

  if (!appConfig) {
    return 'Loading app config...';
  }

  if (appConfig.repositories.length === 0) {
    return <Navigate to='/settings' />;
  }

  return (
    <div className='flex flex-col gap-4'>
      <span>
        Enter a local or remote branch name to get the codeowners for changed files comparing with
        'main' branch. Type HEAD to compare current branch with main.
      </span>
      <div className='flex gap-2 flex-wrap md:flex-nowrap'>
        <Input
          autoFocus
          className='max-w-96 min-w-32'
          type='text'
          ref={branchInputRef}
          placeholder='[origin/][your_name/]branch_name.'
          onKeyUp={e => {
            if (e.key === 'Enter') {
              onGetChangesCodeowners();
            }
          }}
        />
        <Button onClick={onGetChangesCodeowners}>Get codeowners</Button>
        <Button onClick={onGetChangesCodeownersForCurrent} variant='secondary'>
          For current branch
        </Button>
      </div>

      {isLoading && <div>Finding codeowners...</div>}
      {lastOwners && !isLoading && (
        <div className='flex flex-col gap-2'>
          <span>Codeowners for changed files:</span>
          <ScrollArea className='w-full p-4 rounded-md border'>
            <pre className='text-sm text-neutral-900 dark:text-neutral-400'>
              {JSON.stringify(Object.fromEntries(lastOwners.entries()), null, 2)}
            </pre>
          </ScrollArea>
        </div>
      )}
    </div>
  );
}
