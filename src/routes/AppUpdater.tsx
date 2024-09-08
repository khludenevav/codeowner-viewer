import React, { useCallback, useEffect, useState } from 'react';
import { installUpdate, onUpdaterEvent, UpdateStatus } from '@tauri-apps/api/updater';
import { relaunch } from '@tauri-apps/api/process';
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

export const AppUpdater: React.FC = () => {
  const [events, setEvents] = useState<{ error: string | undefined; status: UpdateStatus }[]>([]);
  const [suggestUpdateDialogData, setSuggestUpdateDialogData] = useState<{
    newVersion: string | undefined;
    releaseNotes: string | undefined;
  } | null>(null);
  const [openUpdatingDialog, setOpenUpdatingDialog] = useState(false);
  const appUpdateResponse = useAppCheckUpdate();
  const onInstallConfirm = useCallback(async () => {
    setEvents([]);
    // Install the update. This will also restart the app on Windows.
    await installUpdate();
    // On macOS and Linux we need to restart the app manually.
    await relaunch();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined = undefined;
    const attachUpdater = async () => {
      unlisten = await onUpdaterEvent(({ error, status }) => {
        setEvents(prev => [...prev, { error, status }]);
      });
    };
    attachUpdater();
    // you need to call unlisten if your handler goes out of scope, for example if the component is unmounted.
    return unlisten;
  }, []);

  useEffect(() => {
    if (appUpdateResponse.status === 'success') {
      if (appUpdateResponse.data.shouldUpdate) {
        const tryUpdate = async () => {
          /**
           * How manifest data looks like:
           * version: 0.7.0
           * date: 2024-09-08 10:20:01.515 +00:00:00
           * body: Added the Repo owners tab to explore all repository owners.
           */
          const manifest = appUpdateResponse.data.manifest;
          setSuggestUpdateDialogData({
            newVersion: manifest?.version,
            releaseNotes: manifest?.body,
          });
        };
        tryUpdate();
      }
    }
  }, [
    appUpdateResponse.data?.manifest,
    appUpdateResponse.data?.shouldUpdate,
    appUpdateResponse.status,
  ]);

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
        <DialogContent>
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
          <div>
            {suggestUpdateDialogData?.releaseNotes && (
              <>
                <div>Release notes:</div>
                <div className='ml-2'>{suggestUpdateDialogData.releaseNotes}</div>
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
            {events.some(evt => !!evt.error) &&
              events.map((eventInfo, i) => (
                <div key={i}>
                  Status: {eventInfo.status}. {eventInfo.error && `Error: ${eventInfo.error}`}
                </div>
              ))}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
};
