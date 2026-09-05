import { useCallback } from 'react';

import { DEFAULT_APP_CONFIG } from '@/app-config/app-config';
import { useAppConfig, useUpdateAppConfig } from '@/app-config/useAppConfig';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

type SettingsDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const appConfigResponse = useAppConfig();
  const appConfigUpdate = useUpdateAppConfig();

  const resetEntireAppConfig = useCallback(async () => {
    if (appConfigResponse.status !== 'success') {
      return;
    }
    appConfigUpdate.mutate({
      appConfig: DEFAULT_APP_CONFIG,
    });
  }, [appConfigResponse.status, appConfigUpdate]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-3xl w-[calc(100vw-2rem)] max-h-[85vh] flex flex-col gap-4'>
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
        </DialogHeader>

        <div className='flex-1 overflow-y-auto -mx-6 px-6'>
          {appConfigResponse.status === 'error' && (
            <div className='text-destructive'>Error: {String(appConfigResponse.error)}</div>
          )}

          {appConfigResponse.status === 'pending' && <div>Loading app config...</div>}

          {appConfigResponse.status === 'success' && (
            <div className='flex flex-col gap-6'>
              <section className='flex flex-col gap-2'>
                <div className='text-sm font-medium'>Application config</div>
                <pre className='text-xs bg-muted rounded-md p-3 overflow-x-auto'>
                  {JSON.stringify(appConfigResponse.data, null, 2)}
                </pre>
                <div className='flex gap-2 flex-wrap md:flex-nowrap'>
                  <Button onClick={resetEntireAppConfig} variant='destructive' size='sm'>
                    Reset entire app config
                  </Button>
                </div>
              </section>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
