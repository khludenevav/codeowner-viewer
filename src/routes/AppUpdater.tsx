import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { relaunch } from '@tauri-apps/plugin-process';
import type { DownloadEvent, Update } from '@tauri-apps/plugin-updater';
import { useAppCheckUpdate } from '@/utils/useCheckUpdates';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';

type ProgressEvent = { kind: 'progress'; downloaded: number; total?: number };
type ErrorEvent = { kind: 'error'; message: string };
type FinishedEvent = { kind: 'finished' };
type UpdateEvent = ProgressEvent | ErrorEvent | FinishedEvent;

export const AppUpdater: React.FC = () => {
  const [events, setEvents] = useState<UpdateEvent[]>([]);
  const [suggestUpdateDialogData, setSuggestUpdateDialogData] = useState<{
    newVersion: string | undefined;
    releaseNotes: string | undefined;
  } | null>(null);
  const [openUpdatingDialog, setOpenUpdatingDialog] = useState(false);
  const appUpdateResponse = useAppCheckUpdate();
  const availableUpdate: Update | null = appUpdateResponse.data ?? null;

  const onInstallConfirm = useCallback(async () => {
    if (!availableUpdate) {
      return;
    }
    setEvents([]);
    let contentLength: number | undefined;
    let downloaded = 0;
    try {
      await availableUpdate.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === 'Started') {
          contentLength = event.data.contentLength;
          downloaded = 0;
          setEvents(prev => [...prev, { kind: 'progress', downloaded, total: contentLength }]);
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          setEvents(prev => [...prev, { kind: 'progress', downloaded, total: contentLength }]);
        } else if (event.event === 'Finished') {
          setEvents(prev => [...prev, { kind: 'finished' }]);
        }
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setEvents(prev => [...prev, { kind: 'error', message }]);
      return;
    }
    // On macOS and Linux we need to restart the app manually. `relaunch`
    // is a no-op on Windows when the updater has already replaced the
    // running executable.
    try {
      await relaunch();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setEvents(prev => [
        ...prev,
        {
          kind: 'error',
          message: `Error restarting the app automatically (${message}). Restart manually.`,
        },
      ]);
    }
  }, [availableUpdate]);

  useEffect(() => {
    if (appUpdateResponse.status !== 'success') {
      return;
    }
    if (!availableUpdate) {
      return;
    }
    setSuggestUpdateDialogData({
      newVersion: availableUpdate.version,
      releaseNotes: availableUpdate.body,
    });
  }, [appUpdateResponse.status, availableUpdate]);

  const errorEvents = useMemo(
    () => events.filter((e): e is ErrorEvent => e.kind === 'error'),
    [events],
  );

  return (
    <>
      <Dialog
        open={!!suggestUpdateDialogData}
        onOpenChange={isOpen => {
          if (!isOpen) {
            setSuggestUpdateDialogData(null);
          }
        }}
      >
        <DialogContent className='max-h-[90vh] grid-rows-[auto_minmax(0,1fr)_auto]'>
          <DialogHeader>
            <DialogTitle>Update app</DialogTitle>
            <DialogDescription>
              {suggestUpdateDialogData?.newVersion ? (
                <>The new version {suggestUpdateDialogData.newVersion ?? ''} available.</>
              ) : (
                <>A new version available.</>
              )}
            </DialogDescription>
          </DialogHeader>
          <div className='min-h-0 overflow-y-auto'>
            {suggestUpdateDialogData?.releaseNotes && (
              <>
                <div>Release notes:</div>
                <div className='ml-2'>
                  {suggestUpdateDialogData.releaseNotes.split(/\r?\n/).map((line, i) => (
                    <div key={i}>{line.length > 0 ? line : '\u00A0'}</div>
                  ))}
                </div>
              </>
            )}
            <div className='mt-4'>Would you like to install it now?</div>
          </div>
          <DialogFooter>
            <Button variant='secondary' onClick={() => setSuggestUpdateDialogData(null)}>
              Skip for now
            </Button>
            <Button
              autoFocus
              onClick={() => {
                onInstallConfirm();
                setSuggestUpdateDialogData(null);
                setOpenUpdatingDialog(true);
              }}
            >
              Install now
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog
        open={openUpdatingDialog}
        onOpenChange={() => {
          /** Do nothing. This modal can't be dismissed once update started */
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Installing new version</DialogTitle>
          </DialogHeader>
          <div className='max-h-96 overflow-y-auto'>
            <p>Downloading and installing...</p>
            {errorEvents.length > 0 &&
              errorEvents.map((eventInfo, i) => <div key={i}>Error: {eventInfo.message}</div>)}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
};
