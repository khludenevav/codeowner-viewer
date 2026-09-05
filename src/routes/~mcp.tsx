import { createFileRoute } from '@tanstack/react-router';
import { McpPage } from '@/mcp/McpPage';

export const Route = createFileRoute('/mcp')({
  component: McpPage,
});
