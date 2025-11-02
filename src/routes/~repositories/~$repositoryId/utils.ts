export function getFileExtension(fileName: string): string {
  const lastDotIndex = fileName.indexOf('.');
  if (lastDotIndex === -1) {
    return '';
  }
  return fileName.substring(lastDotIndex);
}
