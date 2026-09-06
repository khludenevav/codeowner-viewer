import { Button } from '@/components/ui/button';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { writeTextFile, exists } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { toast } from 'sonner';

const FILENAME = 'codeowners';

type Props = {
  absRepoPath: string;
  branch: string;
  /** null = no owner filter applied. Set of owner handles otherwise. */
  filteredOwners: Set<string> | null;
  /** null = no extension filter applied. Set of extensions otherwise. */
  filteredExtensions: Set<string> | null;
  /** Whether the underlying data has finished loading. Used to disable the button while loading. */
  ready: boolean;
};

async function fetchExportJson(
  absRepoPath: string,
  branch: string,
  filteredOwners: Set<string> | null,
  filteredExtensions: Set<string> | null,
): Promise<string> {
  return invoke<string>('export_codeowners_for_branch', {
    absRepoPath,
    branch,
    owners: filteredOwners ? Array.from(filteredOwners) : null,
    extensions: filteredExtensions ? Array.from(filteredExtensions) : null,
  });
}

async function createFileInSelectedDir(
  absRepoPath: string,
  branch: string,
  filteredOwners: Set<string> | null,
  filteredExtensions: Set<string> | null,
) {
  const selectedDir = await open({
    directory: true,
    multiple: false,
    title: 'Select a directory to save the file',
  });

  if (!selectedDir || Array.isArray(selectedDir)) {
    return;
  }
  let path: string;
  let index = 0;
  do {
    path = await join(selectedDir, `${FILENAME}${index === 0 ? '' : index}.json`);
    index += 1;
  } while (await exists(path));

  let contents: string;
  try {
    contents = await fetchExportJson(absRepoPath, branch, filteredOwners, filteredExtensions);
  } catch (e) {
    toast.error(`Failed to build export payload: ${e}`);
    return;
  }

  try {
    await writeTextFile(path, contents);
    toast.success(`File saved successfully to ${path}`);
  } catch (e) {
    toast.error(`Failed to save file: ${e}`);
  }
}

export const ExportToFileButton: React.FC<Props> = ({
  absRepoPath,
  branch,
  filteredOwners,
  filteredExtensions,
  ready,
}) => {
  return (
    <Button
      variant='outline'
      disabled={!ready}
      onClick={() =>
        createFileInSelectedDir(absRepoPath, branch, filteredOwners, filteredExtensions)
      }
    >
      Export to json...
    </Button>
  );
};
