import { useEffect, useState } from "react";
import {
  ActionIcon,
  Alert,
  Button,
  Drawer,
  Group,
  NumberInput,
  Select,
  Stack,
  Table,
  Text,
  Textarea,
  TextInput,
  Title,
} from "@mantine/core";
import { IconTrash, IconPlus } from "@tabler/icons-react";
import {
  createInvoice,
  updateInvoiceStatus,
  deleteInvoiceItem,
  generateInvoicePdf,
} from "../api/invoices";
import type { Invoice, InvoiceInput } from "../api/invoices";
import { listClients } from "../api/clients";
import type { Client } from "../api/clients";
import { listEngagements } from "../api/engagements";
import type { Engagement } from "../api/engagements";

interface Props {
  opened: boolean;
  onClose: () => void;
  invoice?: Invoice | null;
  onSaved: () => void;
}

interface ItemInput {
  id?: number;
  description: string;
  quantity: number;
  rate_cents: number;
}

export function InvoiceBuilder({ opened, onClose, invoice, onSaved }: Props) {
  const isEdit = !!invoice;
  const [invoiceNumber, setInvoiceNumber] = useState(
    invoice?.invoice_number || "",
  );
  const [clientId, setClientId] = useState<string>(
    invoice ? String(invoice.client_id) : "",
  );
  const [engagementId, setEngagementId] = useState<string>(
    invoice?.engagement_id ? String(invoice.engagement_id) : "",
  );
  const [documentType, setDocumentType] = useState(
    invoice?.document_type || "invoice",
  );
  const [currency, setCurrency] = useState(invoice?.currency || "USD");
  const [taxRateBps, setTaxRateBps] = useState<number>(
    invoice?.tax_rate_bps || 0,
  );
  const [discountCents, setDiscountCents] = useState<number>(
    invoice?.discount_cents || 0,
  );
  const [notes, setNotes] = useState(invoice?.notes || "");
  const [items, setItems] = useState<ItemInput[]>(
    invoice?.items.map((i) => ({
      id: i.id,
      description: i.description,
      quantity: i.quantity,
      rate_cents: i.rate_cents,
    })) || [{ description: "", quantity: 1, rate_cents: 0 }],
  );
  const [clients, setClients] = useState<Client[]>([]);
  const [engagements, setEngagements] = useState<Engagement[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (opened) {
      setInvoiceNumber(invoice?.invoice_number || "");
      setClientId(invoice ? String(invoice.client_id) : "");
      setEngagementId(
        invoice?.engagement_id ? String(invoice.engagement_id) : "",
      );
      setDocumentType(invoice?.document_type || "invoice");
      setCurrency(invoice?.currency || "USD");
      setTaxRateBps(invoice?.tax_rate_bps || 0);
      setDiscountCents(invoice?.discount_cents || 0);
      setNotes(invoice?.notes || "");
      setItems(
        invoice?.items.map((i) => ({
          id: i.id,
          description: i.description,
          quantity: i.quantity,
          rate_cents: i.rate_cents,
        })) || [{ description: "", quantity: 1, rate_cents: 0 }],
      );
      listClients().then(setClients).catch(console.error);
      listEngagements().then(setEngagements).catch(console.error);
    }
  }, [opened, invoice]);

  const handleSave = async () => {
    if (!invoiceNumber.trim() || !clientId) {
      setError("Invoice number and client are required.");
      return;
    }
    setError(null);
    setLoading(true);
    try {
      const input: InvoiceInput = {
        client_id: Number(clientId),
        engagement_id: engagementId ? Number(engagementId) : undefined,
        document_type: documentType,
        invoice_number: invoiceNumber.trim(),
        tax_rate_bps: taxRateBps,
        discount_cents: discountCents,
        discount_pct_bps: 0,
        currency,
        notes: notes || undefined,
        items: items.map((i) => ({
          description: i.description,
          quantity: i.quantity,
          rate_cents: i.rate_cents,
        })),
      };
      if (isEdit && invoice) {
        await updateInvoiceStatus(invoice.id, invoice.status);
        // For existing items, we can't easily sync via this simple builder.
        // Re-add them simply by clearing old and re-adding (simplified).
        for (const old of invoice.items) {
          await deleteInvoiceItem(old.id);
        }
        await createInvoice(input);
        // Close and let parent refresh
        onSaved();
        onClose();
      } else {
        await createInvoice(input);
        onSaved();
        onClose();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const addItem = () => {
    setItems([...items, { description: "", quantity: 1, rate_cents: 0 }]);
  };

  const removeItem = (index: number) => {
    setItems(items.filter((_, i) => i !== index));
  };

  const updateItemField = (
    index: number,
    field: keyof ItemInput,
    value: string | number,
  ) => {
    const next = [...items];
    next[index] = { ...next[index], [field]: value };
    setItems(next);
  };

  const formatCents = (cents: number) => `$${(cents / 100).toFixed(2)}`;

  const subtotal = items.reduce((sum, i) => sum + i.quantity * i.rate_cents, 0);
  const tax = Math.round((subtotal * taxRateBps) / 10000);
  const total = subtotal + tax - discountCents;

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={isEdit ? "Edit Invoice" : "New Invoice"}
      position="right"
      size="xl"
    >
      <Stack gap="md">
        <TextInput
          label="Invoice Number"
          value={invoiceNumber}
          onChange={(e) => setInvoiceNumber(e.currentTarget.value)}
        />
        {!isEdit && (
          <Select
            label="Type"
            value={documentType}
            onChange={(v) => setDocumentType(v || "invoice")}
            data={[
              { value: "invoice", label: "Invoice" },
              { value: "quote", label: "Quote" },
            ]}
          />
        )}
        <Select
          label="Client"
          value={clientId}
          onChange={(v) => setClientId(v || "")}
          data={clients.map((c) => ({
            value: String(c.id),
            label: c.name,
          }))}
          searchable
        />
        <Select
          label="Engagement (optional)"
          value={engagementId}
          onChange={(v) => setEngagementId(v || "")}
          data={[
            { value: "", label: "None" },
            ...engagements.map((e) => ({
              value: String(e.id),
              label: e.name,
            })),
          ]}
          searchable
        />
        <TextInput
          label="Currency"
          value={currency}
          onChange={(e) => setCurrency(e.currentTarget.value)}
        />
        <Textarea
          label="Notes"
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
          autosize
          minRows={3}
          maxRows={10}
        />

        <Title order={5}>Line Items</Title>
        <Table>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Description</Table.Th>
              <Table.Th>Qty</Table.Th>
              <Table.Th>Rate</Table.Th>
              <Table.Th>Total</Table.Th>
              <Table.Th></Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {items.map((item, i) => (
              <Table.Tr key={i}>
                <Table.Td>
                  <TextInput
                    value={item.description}
                    onChange={(e) =>
                      updateItemField(i, "description", e.currentTarget.value)
                    }
                  />
                </Table.Td>
                <Table.Td>
                  <NumberInput
                    value={item.quantity}
                    onChange={(v) =>
                      updateItemField(i, "quantity", Number(v) || 0)
                    }
                    min={1}
                    hideControls
                    style={{ width: 60 }}
                  />
                </Table.Td>
                <Table.Td>
                  <NumberInput
                    value={item.rate_cents / 100}
                    onChange={(v) =>
                      updateItemField(
                        i,
                        "rate_cents",
                        Math.round((Number(v) || 0) * 100),
                      )
                    }
                    decimalScale={2}
                    hideControls
                    style={{ width: 100 }}
                  />
                </Table.Td>
                <Table.Td>
                  {formatCents(item.quantity * item.rate_cents)}
                </Table.Td>
                <Table.Td>
                  <ActionIcon color="red" onClick={() => removeItem(i)}>
                    <IconTrash size={16} />
                  </ActionIcon>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
        <Button
          leftSection={<IconPlus size={16} />}
          variant="light"
          onClick={addItem}
        >
          Add Item
        </Button>

        <Group justify="space-between">
          <TextInput
            label="Tax rate (%)"
            value={String(taxRateBps / 100)}
            onChange={(e) =>
              setTaxRateBps(
                Math.round(Number(e.currentTarget.value) * 100) || 0,
              )
            }
            style={{ width: 120 }}
          />
          <NumberInput
            label="Discount ($)"
            value={discountCents / 100}
            onChange={(v) =>
              setDiscountCents(Math.round((Number(v) || 0) * 100))
            }
            decimalScale={2}
            hideControls
            style={{ width: 120 }}
          />
        </Group>

        <Group justify="flex-end">
          <Stack gap={0} ta="right">
            <Text size="sm">Subtotal: {formatCents(subtotal)}</Text>
            <Text size="sm">Tax: {formatCents(tax)}</Text>
            <Text size="sm">Discount: -{formatCents(discountCents)}</Text>
            <Text fw={700}>Total: {formatCents(total)}</Text>
          </Stack>
        </Group>

        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          {isEdit && invoice && (
            <Button
              variant="light"
              loading={loading}
              onClick={async () => {
                setLoading(true);
                try {
                  const path = await generateInvoicePdf(
                    invoice.id,
                    `/tmp/invoice-${invoice.invoice_number}.pdf`,
                  );
                  alert(`PDF saved to ${path}`);
                } catch (e) {
                  setError(String(e));
                } finally {
                  setLoading(false);
                }
              }}
            >
              Generate PDF
            </Button>
          )}
          <Button onClick={handleSave} loading={loading}>
            Save
          </Button>
        </Group>
      </Stack>
    </Drawer>
  );
}
