import { createFileRoute } from '@tanstack/react-router';

import { useCallback, useRef, useState } from 'react';

import { type AppConfig } from '../../../app-config/app-config';
import { getBranchDifference } from '../../../utils/codeowners-command';
import { useAppConfig } from '../../../app-config/useAppConfig';
import { Button } from '@/components/ui/button';

export const Route = createFileRoute('/repositories/$repositoryId/codeowners')({
  component: Codeowners,
});
function Codeowners() {
  console.log('co');
  const branchInputRef = useRef<HTMLInputElement>(null);
  const [isLoading, setIsLoading] = useState<boolean>();
  const [lastOwners, setLastOwners] = useState<Map<string, string[]> | null>(null);
  const appConfigResponse = useAppConfig();
  const appConfig: AppConfig | undefined = appConfigResponse.data;

  const onBranchDiff = useCallback(async () => {
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
    <>
      <input autoFocus style={{ minWidth: '300px' }} type='text' ref={branchInputRef}></input>
      <Button className='left-2' onClick={onBranchDiff}>
        Get branch diff
      </Button>

      <hr />
      {isLoading && <div>Finding codeowners...</div>}
      {lastOwners && !isLoading && (
        <pre>{JSON.stringify(Object.fromEntries(lastOwners.entries()), null, 2)}</pre>
      )}
    </>
  );
}
