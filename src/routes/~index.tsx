import { createFileRoute, Navigate } from '@tanstack/react-router';
import { useAppConfig } from '../app-config/useAppConfig';

export const Route = createFileRoute('/')({
  component: Index,
});

function Index() {
  const appConfigResponse = useAppConfig();
  if (appConfigResponse.status !== 'success') {
    return <div>Loading config...</div>;
  }

  if (appConfigResponse.data.repositories.length > 0) {
    return (
      <Navigate to='/repositories/$repositoryId/codeowners' params={{ repositoryId: 'any' }} />
    );
  } else {
    <Navigate to='/settings' />;
  }
}
