import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AppConfig, readAppConfig, writeAppConfig } from './app-config';

const appConfigQueryKey = ['app-config'];

export function useAppConfig() {
  const result = useQuery({ queryKey: appConfigQueryKey, queryFn: readAppConfig });
  return result;
}

type MutationVariable = {
  appConfig: AppConfig;
};

export function useUpdateAppConfig() {
  const queryClient = useQueryClient();
  const mutateAppConfig = useMutation({
    mutationKey: appConfigQueryKey,
    mutationFn: ({ appConfig }: MutationVariable) => writeAppConfig(appConfig),
    onSuccess: (_, { appConfig }) => queryClient.setQueryData(appConfigQueryKey, appConfig),
  });
  return mutateAppConfig;
}
