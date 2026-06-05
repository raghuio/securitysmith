import { ReactNode } from "react";
import { Stack } from "@mantine/core";
import { PageLayout } from "./PageLayout";

interface ListPageProps {
  title: string;
  breadcrumbs?: string[];
  searchToolbar?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  emptyState?: ReactNode;
  isEmpty?: boolean;
}

export function ListPage({
  title,
  breadcrumbs,
  searchToolbar,
  actions,
  children,
  emptyState,
  isEmpty,
}: ListPageProps) {
  return (
    <PageLayout title={title} breadcrumbs={breadcrumbs} actions={actions}>
      {searchToolbar && <div>{searchToolbar}</div>}
      {isEmpty && emptyState ? (
        <Stack align="center" py="xl">
          {emptyState}
        </Stack>
      ) : (
        <Stack gap="sm">{children}</Stack>
      )}
    </PageLayout>
  );
}
