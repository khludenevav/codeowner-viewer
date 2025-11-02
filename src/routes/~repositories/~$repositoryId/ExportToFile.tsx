import { Button } from '@/components/ui/button';
import { DirectoryOwners, FileOwners } from '@/utils/all-owners';
import { open } from '@tauri-apps/api/dialog';
import { writeFile, exists } from '@tauri-apps/api/fs';
import { join } from '@tauri-apps/api/path';
import { toast } from 'sonner';

const FILENAME = 'codeowners';

async function joinDir(parentDir: string, name: string): Promise<string> {
  // handling empty string separately or else join adds an extra slash at the start
  return parentDir === '' ? name : await join(parentDir, name);
}

async function getContent(filteredRoot: DirectoryOwners): Promise<string> {
  /** Key os owner, value - list of paths */
  const ownersMap = new Map<string, string[]>();

  const addFiles = async (parentDirPath: string, files: FileOwners[]) => {
    for (const file of files) {
      let existingRecord = ownersMap.get(file.owner);
      if (!existingRecord) {
        existingRecord = [];
        ownersMap.set(file.owner, existingRecord);
      }
      existingRecord.push(await joinDir(parentDirPath, file.name));
    }
  };

  const addDirectory = async (parentDirPath: string, dir: DirectoryOwners) => {
    const currentDirPath = await joinDir(parentDirPath, dir.name);
    await addFiles(currentDirPath, dir.files);
    await addDirectories(currentDirPath, dir.directories);
  };

  const addDirectories = async (parentDirPath: string, dirs: DirectoryOwners[]) => {
    for (const dir of dirs) {
      await addDirectory(parentDirPath, dir);
    }
  };

  await addFiles('', filteredRoot.files);
  await addDirectories('', filteredRoot.directories);
  return JSON.stringify(Object.fromEntries(ownersMap.entries()), null, 2);
}

async function createFileInSelectedDir(filteredRoot: DirectoryOwners) {
  // Ask the user to pick a directory
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
  const contents = await getContent(filteredRoot);
  try {
    await writeFile({ path, contents });
    toast.success(`File saved successfully to ${path}`);
  } catch (e) {
    toast.error(`Failed to save file: ${e}`);
  }
}

type Props = {
  filteredRoot: DirectoryOwners;
};

export const ExportToFileButton: React.FC<Props> = ({ filteredRoot }) => {
  return (
    <Button variant='outline' onClick={() => createFileInSelectedDir(filteredRoot)}>
      Export to json...
    </Button>
  );
};
