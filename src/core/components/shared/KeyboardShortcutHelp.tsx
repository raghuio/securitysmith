import { Modal, Table, Text } from "@mantine/core";

interface KeyboardShortcutHelpProps {
  opened: boolean;
  onClose: () => void;
}

const SHORTCUTS = [
  { key: "Ctrl + K", action: "Open Command Palette / Global Search" },
  { key: "Ctrl + ;", action: "Open AI Minibuffer" },
  { key: "?", action: "Show this help overlay" },
  { key: "Esc", action: "Close dialogs / palettes" },
];

export function KeyboardShortcutHelp({
  opened,
  onClose,
}: KeyboardShortcutHelpProps) {
  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title="Keyboard Shortcuts"
      size="sm"
    >
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Shortcut</Table.Th>
            <Table.Th>Action</Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>
          {SHORTCUTS.map((s) => (
            <Table.Tr key={s.key}>
              <Table.Td>
                <Text fw={600} size="sm">
                  {s.key}
                </Text>
              </Table.Td>
              <Table.Td>
                <Text size="sm">{s.action}</Text>
              </Table.Td>
            </Table.Tr>
          ))}
        </Table.Tbody>
      </Table>
    </Modal>
  );
}
