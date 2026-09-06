import { Dispatch, SetStateAction, useCallback, useState } from 'react';
import { useParams } from '@tanstack/react-router';

/**
 * In-memory, per-repository UI state.
 *
 * Layout: `store[repositoryId][stateKey] = value`.
 *
 * The store lives at module scope so it survives navigating between pages
 * within the app, but is intentionally not persisted to disk.
 */
const store = new Map<string, Map<string, unknown>>();

const NO_REPOSITORY_KEY = '__no-repository__';

function getRepositoryBag(repositoryId: string): Map<string, unknown> {
  let bag = store.get(repositoryId);
  if (!bag) {
    bag = new Map();
    store.set(repositoryId, bag);
  }
  return bag;
}

function isInitializer<T>(value: T | (() => T)): value is () => T {
  return typeof value === 'function';
}

/**
 * `useState` that is scoped to the repository currently selected in the URL.
 *
 * Reading a value that has never been set for the current repository falls
 * back to `initial`, so the first render on a repository matches the plain
 * `useState(initial)` behaviour. Every subsequent set is mirrored into the
 * module-level cache, so navigating away and coming back (as long as the
 * hosting component is remounted with `key={repositoryId}`) restores the
 * previous value.
 */
export function useRepositoryPageState<T>(
  stateKey: string,
  initial: T | (() => T),
): [T, Dispatch<SetStateAction<T>>] {
  const params = useParams({ strict: false }) as { repositoryId?: string };
  const repositoryId = params.repositoryId ?? NO_REPOSITORY_KEY;

  const [state, setStateInner] = useState<T>(() => {
    const bag = getRepositoryBag(repositoryId);
    if (bag.has(stateKey)) {
      return bag.get(stateKey) as T;
    }
    return isInitializer(initial) ? initial() : initial;
  });

  const setState = useCallback<Dispatch<SetStateAction<T>>>(
    value => {
      setStateInner(prev => {
        const next = typeof value === 'function' ? (value as (prev: T) => T)(prev) : value;
        getRepositoryBag(repositoryId).set(stateKey, next);
        return next;
      });
    },
    [repositoryId, stateKey],
  );

  return [state, setState];
}
