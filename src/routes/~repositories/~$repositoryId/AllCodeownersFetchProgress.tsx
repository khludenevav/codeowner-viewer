import { Repositories } from '@/app-config/app-config';
import { useAllCodeownersProgress } from '@/utils/all-owners';

type Props = {
  repository: Repositories | null;
  branch: string | null;
};
export const AllCodeownersFetchProgress: React.FC<Props> = ({ repository, branch }) => {
  const { data, status } = useAllCodeownersProgress(repository, branch);
  if (status === 'error') {
    return <div>Error loading codeowners fetch progress</div>;
  }

  return (
    <div>
      Calculating codeowners tree: {data.files_handled.toLocaleString()} /{' '}
      {data.files_total.toLocaleString()} files handled.
    </div>
  );
};
