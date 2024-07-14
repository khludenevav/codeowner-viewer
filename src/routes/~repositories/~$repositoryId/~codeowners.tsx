import { createFileRoute } from '@tanstack/react-router';

import { useCallback, useRef, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { getBranchDifference } from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

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

  if (!appConfig) {
    return 'Loading app config...';
  }

  return (
    <div className='flex flex-col gap-4'>
      <span>
        Enter the local or remote branch name to get the codeowners for changed files comparing with
        'main' branch.
      </span>
      <div>
        <Input
          autoFocus
          className='max-w-96'
          type='text'
          ref={branchInputRef}
          placeholder='origin/your_name/b_name or your_name/branch_name'
          onKeyUp={e => {
            if (e.key === 'Enter') {
              onGetChangesCodeowners();
            }
          }}
        />
        <Button className='ml-2' onClick={onGetChangesCodeowners}>
          Get codeowners
        </Button>
      </div>

      {isLoading && <div>Finding codeowners...</div>}
      {lastOwners && !isLoading && (
        <div className='flex flex-col gap4'>
          <span>Codeowners for changed files:</span>
          <pre>{JSON.stringify(Object.fromEntries(lastOwners.entries()), null, 2)}</pre>
        </div>
      )}
    </div>
  );
}
