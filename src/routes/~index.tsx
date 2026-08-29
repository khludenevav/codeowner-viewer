import { createFileRoute, Navigate } from '@tanstack/react-router';
import { useAppConfig } from '../app-config/useAppConfig';

export const Route = createFileRoute('/')({
  component: Index,
});

function Index() {
  const appConfigResponse = useAppConfig();

  if (appConfigResponse.status !== 'success') {
    return <div>Loading app config...</div>;
  }

  const repositories = appConfigResponse.data.repositories;
  if (repositories.length > 0) {
    return (
      <Navigate
        to='/repositories/$repositoryId/codeowners'
        params={{ repositoryId: repositories[0].id }}
      />
    );
  }

  return (
    <div className='flex flex-col items-center justify-center gap-4 px-6 py-16 text-center'>
      <h1 className='text-2xl font-semibold'>No repositories yet</h1>
      <p className='max-w-md text-sm text-muted-foreground'>
        Add a local repository using the “Add repository” button above to start viewing its
        CODEOWNERS and branch changes.
      </p>
    </div>
  );
}
