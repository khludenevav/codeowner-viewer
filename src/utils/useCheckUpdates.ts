import { useQuery, useQueryClient } from '@tanstack/react-query';
import { checkUpdate } from '@tauri-apps/api/updater';
import { useCallback } from 'react';

const appUpdateQueryKey = ['app-update'];
const UPDATE_CHECK_DURATION = 1_000 * 60 * 60 * 24; // every day in ms

export function useAppCheckUpdate() {
  const result = useQuery({
    queryKey: appUpdateQueryKey,
    queryFn: () => checkUpdate(),
    refetchInterval: UPDATE_CHECK_DURATION,
    staleTime: UPDATE_CHECK_DURATION, // It will trigger update once view
    refetchOnWindowFocus: false,
  });
  return result;
}

export function useInvalidateAppCheckUpdate() {
  const queryClient = useQueryClient();
  const invalidateAppCheckUpdate = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: appUpdateQueryKey });
  }, [queryClient]);
  return invalidateAppCheckUpdate;
}
