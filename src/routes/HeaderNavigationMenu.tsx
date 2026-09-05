import { useCallback, useState } from 'react';
import { Repositories } from '@/app-config/app-config';
import { useAddRepository } from '@/app-config/useAddRepository';
import { useAppConfig, useUpdateAppConfig } from '@/app-config/useAppConfig';
import { getRepositoryLabel } from '@/app-config/useCurrentRepository';
import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { ThemeToggle } from '@/components/theme/ThemeToggle';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tooltip } from '@/components/ui/tooltip';
import { useMcpHeaderStatus } from '@/mcp/useMcp';
import { cn } from '@/utils/components-utils';
import { Link, useMatchRoute, useNavigate, useParams } from '@tanstack/react-router';
import { Plug, Plus, SettingsIcon, History, X } from 'lucide-react';

type SectionKey = 'codeowners' | 'file-owner' | 'all-owners';

const SECTIONS: { key: SectionKey; label: string; to: `/repositories/$repositoryId/${SectionKey}` }[] = [
  { key: 'codeowners', label: 'Branch changes', to: '/repositories/$repositoryId/codeowners' },
  { key: 'file-owner', label: 'File owners', to: '/repositories/$repositoryId/file-owner' },
  { key: 'all-owners', label: 'Repo owners', to: '/repositories/$repositoryId/all-owners' },
];

function useCurrentSection(): SectionKey {
  const matchRoute = useMatchRoute();
  for (const section of SECTIONS) {
    if (matchRoute({ to: section.to })) {
      return section.key;
    }
  }
  return 'codeowners';
}

function useCurrentRepositoryId(): string | null {
  const params = useParams({ strict: false }) as { repositoryId?: string };
  return params.repositoryId ?? null;
}

export const HeaderNavigationMenu = () => {
  const appConfigResponse = useAppConfig();
  const repositories = appConfigResponse.data?.repositories ?? [];

  return (
    <div className='flex flex-col'>
      <RepoTabsRow repositories={repositories} />
      {repositories.length > 0 && <SectionTabsRow />}
    </div>
  );
};

type RepoTabsRowProps = {
  repositories: Repositories[];
};

function RepoTabsRow({ repositories }: RepoTabsRowProps) {
  const currentSection = useCurrentSection();
  const currentRepositoryId = useCurrentRepositoryId();
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <nav className='border-b' aria-label='Repositories'>
      <div className='flex items-end gap-2 px-4 pt-1.5'>
        <div className='flex-1 min-w-0 flex items-end gap-0.5 overflow-x-auto whitespace-nowrap'>
          {repositories.map(repository => (
            <RepoTab
              key={repository.id}
              repository={repository}
              isActive={repository.id === currentRepositoryId}
              sectionKey={currentSection}
            />
          ))}
          <AddRepoButton />
        </div>

        <div className='flex items-center gap-0.5 flex-shrink-0 pb-1'>
          <McpHeaderButton />
          <ChangelogHeaderButton />
          <Tooltip content='Settings'>
            <Button
              type='button'
              variant='ghost'
              size='icon'
              aria-label='Settings'
              onClick={() => setSettingsOpen(true)}
              className={cn(
                'h-8 w-8 rounded-md text-foreground/70 hover:text-foreground',
                settingsOpen && 'bg-secondary text-foreground',
              )}
            >
              <SettingsIcon className='h-4 w-4' />
            </Button>
          </Tooltip>
          <ThemeToggle />
        </div>
      </div>
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </nav>
  );
}

type RepoTabProps = {
  repository: Repositories;
  isActive: boolean;
  sectionKey: SectionKey;
};

function RepoTab({ repository, isActive, sectionKey }: RepoTabProps) {
  const label = getRepositoryLabel(repository);
  const section = SECTIONS.find(s => s.key === sectionKey) ?? SECTIONS[0];
  const [confirmOpen, setConfirmOpen] = useState(false);

  return (
    <>
      <Tooltip content={repository.repoPath}>
        <div
          className={cn(
            'group relative inline-flex items-center h-8 rounded-t-md text-[13px] font-medium transition-colors',
            isActive
              ? 'bg-background text-foreground border border-b-0 border-border -mb-px'
              : 'text-foreground/60 hover:bg-accent/70 hover:text-foreground',
          )}
        >
          <Link
            to={section.to}
            params={{ repositoryId: repository.id }}
            className='inline-flex items-center h-full pl-3 pr-1.5 outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-t-md'
          >
            {label}
          </Link>
          <button
            type='button'
            aria-label={`Remove ${label}`}
            className={cn(
              'inline-flex items-center justify-center h-5 w-5 mr-1.5 rounded-sm',
              'opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity',
              'hover:bg-foreground/10 text-foreground/60 hover:text-foreground',
            )}
            onClick={event => {
              event.preventDefault();
              event.stopPropagation();
              setConfirmOpen(true);
            }}
          >
            <X className='h-3 w-3' />
          </button>
        </div>
      </Tooltip>

      <RemoveRepoConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        repository={repository}
      />
    </>
  );
}

function McpHeaderButton() {
  const matchRoute = useMatchRoute();
  const state = useMcpHeaderStatus();
  const isActive = !!matchRoute({ to: '/mcp' });
  const [dotColor, tooltip] = (() => {
    switch (state) {
      case 'running':
        return ['bg-green-500', 'MCP running — click to open'];
      case 'error':
        return ['bg-destructive', 'MCP error — click to open'];
      case 'disabled':
        return ['bg-muted-foreground', 'MCP disabled — click to open'];
      case 'stopped':
      default:
        return ['bg-muted-foreground', 'MCP stopped — click to open'];
    }
  })();
  return (
    <Tooltip content={tooltip}>
      <Link to='/mcp' className='block'>
        <Button
          type='button'
          variant='ghost'
          size='sm'
          aria-label='MCP server'
          className={cn(
            'relative h-8 gap-1.5 px-2 text-foreground/70 hover:text-foreground',
            isActive && 'bg-secondary text-foreground',
          )}
        >
          <Plug className='h-4 w-4' />
          <span className='text-xs font-medium'>MCP</span>
          <span
            className={cn(
              'ml-0.5 h-1.5 w-1.5 rounded-full',
              dotColor,
            )}
            aria-hidden='true'
          />
        </Button>
      </Link>
    </Tooltip>
  );
}

function ChangelogHeaderButton() {
  const matchRoute = useMatchRoute();
  const isActive = !!matchRoute({ to: '/changelog' });
  return (
    <Tooltip content='Changelog'>
      <Link to='/changelog' className='block'>
        <Button
          type='button'
          variant='ghost'
          size='sm'
          aria-label='Changelog'
          className={cn(
            'h-8 gap-1.5 px-2 text-foreground/70 hover:text-foreground',
            isActive && 'bg-secondary text-foreground',
          )}
        >
          <History className='h-4 w-4' />
          <span className='text-xs font-medium'>Changelog</span>
        </Button>
      </Link>
    </Tooltip>
  );
}

function AddRepoButton() {
  const appConfigResponse = useAppConfig();
  const handleClick = useAddRepository();

  const noRepositories = appConfigResponse.data?.repositories.length === 0;

  if (noRepositories) {
    return (
      <Tooltip content='Add repository'>
        <Button
          type='button'
          variant='default'
          onClick={handleClick}
          aria-label='Add repository'
          className='flex-shrink-0 mb-1'
        >
          <Plus className='h-4 w-4 mr-1' />
          Add repository
        </Button>
      </Tooltip>
    );
  }

  return (
    <Tooltip content='Add repository'>
      <Button
        type='button'
        variant='ghost'
        size='icon'
        onClick={handleClick}
        aria-label='Add repository'
        className='flex-shrink-0 h-8 w-8 text-foreground/70 hover:text-foreground'
      >
        <Plus className='h-4 w-4' />
      </Button>
    </Tooltip>
  );
}

type RemoveRepoConfirmDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  repository: Repositories;
};

function RemoveRepoConfirmDialog({
  open,
  onOpenChange,
  repository,
}: RemoveRepoConfirmDialogProps) {
  const navigate = useNavigate();
  const appConfigResponse = useAppConfig();
  const appConfigUpdate = useUpdateAppConfig();
  const currentRepositoryId = useCurrentRepositoryId();
  const currentSection = useCurrentSection();
  const label = getRepositoryLabel(repository);

  const handleConfirm = useCallback(() => {
    if (appConfigResponse.status !== 'success') {
      return;
    }
    const repositories = appConfigResponse.data.repositories.filter(
      repo => repo.id !== repository.id,
    );
    appConfigUpdate.mutate(
      {
        appConfig: {
          ...appConfigResponse.data,
          repositories,
        },
      },
      {
        onSuccess: () => {
          onOpenChange(false);
          if (currentRepositoryId === repository.id) {
            if (repositories.length === 0) {
              navigate({ to: '/' });
            } else {
              const section = SECTIONS.find(s => s.key === currentSection) ?? SECTIONS[0];
              navigate({
                to: section.to,
                params: { repositoryId: repositories[0].id },
              });
            }
          }
        },
      },
    );
  }, [
    appConfigResponse.data,
    appConfigResponse.status,
    appConfigUpdate,
    currentRepositoryId,
    currentSection,
    navigate,
    onOpenChange,
    repository.id,
  ]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Remove repository "{label}"?</DialogTitle>
          <DialogDescription>
            {repository.repoPath}
            <br />
            The repository won't be deleted from disk; only removed from this app's list.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant='outline' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant='destructive' onClick={handleConfirm}>
            Remove
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function SectionTabsRow() {
  const matchRoute = useMatchRoute();
  const appConfigResponse = useAppConfig();
  const currentRepositoryId = useCurrentRepositoryId();
  const repositories = appConfigResponse.data?.repositories ?? [];
  const noRepositories = repositories.length === 0;

  const effectiveRepositoryId =
    currentRepositoryId && repositories.some(r => r.id === currentRepositoryId)
      ? currentRepositoryId
      : (repositories[0]?.id ?? null);

  if (!effectiveRepositoryId) {
    return null;
  }

  return (
    <div className='px-4 py-2' aria-label='Sections'>
      <Tooltip content={noRepositories ? 'Add at least one repository' : null}>
        <div
          role='tablist'
          className='inline-flex items-center gap-0.5 rounded-md bg-secondary border border-border p-1'
        >
          {SECTIONS.map(section => {
            const isActive = !!matchRoute({ to: section.to });
            return (
              <Link
                key={section.key}
                to={section.to}
                params={{ repositoryId: effectiveRepositoryId }}
                role='tab'
                aria-selected={isActive}
                className={cn(
                  'inline-flex items-center h-7 px-3 rounded text-[13px] transition-colors',
                  'outline-none focus-visible:ring-2 focus-visible:ring-ring',
                  isActive
                    ? 'bg-background text-foreground font-medium shadow ring-1 ring-border/60'
                    : 'text-foreground/60 hover:text-foreground',
                )}
              >
                {section.label}
              </Link>
            );
          })}
        </div>
      </Tooltip>
    </div>
  );
}
