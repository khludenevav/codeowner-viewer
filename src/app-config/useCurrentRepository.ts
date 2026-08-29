import { useParams } from '@tanstack/react-router';
import { useAppConfig } from './useAppConfig';
import { Repositories } from './app-config';

type CurrentRepositoryResult =
  | { status: 'loading'; repository: null }
  | { status: 'no-repositories'; repository: null }
  | { status: 'not-found'; repository: null }
  | { status: 'ready'; repository: Repositories };

/**
 * Resolves the `$repositoryId` route param to a repository from the app config.
 */
export function useCurrentRepository(): CurrentRepositoryResult {
  const appConfigResponse = useAppConfig();
  const params = useParams({ strict: false }) as { repositoryId?: string };
  const repositoryId = params.repositoryId;

  if (appConfigResponse.status !== 'success') {
    return { status: 'loading', repository: null };
  }

  const repositories = appConfigResponse.data.repositories;
  if (repositories.length === 0) {
    return { status: 'no-repositories', repository: null };
  }

  const repository = repositories.find(repo => repo.id === repositoryId);
  if (!repository) {
    return { status: 'not-found', repository: null };
  }

  return { status: 'ready', repository };
}

/** Human-friendly label = last path segment of the repoPath. */
export function getRepositoryLabel(repository: Repositories): string {
  const path = repository.repoPath.replace(/[\\/]+$/, '');
  const parts = path.split(/[\\/]/);
  const last = parts[parts.length - 1];
  return last || repository.repoPath;
}
