import { useAllCodeownersProgress } from '@/utils/all-owners';

type Props = {
  branch: string | null;
};
export const AllCodeownersFetchProgress: React.FC<Props> = ({ branch }) => {
  const { data, status } = useAllCodeownersProgress(branch);
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
