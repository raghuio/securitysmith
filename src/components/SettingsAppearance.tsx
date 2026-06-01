import { useState } from "react";
import {
  Button,
  ColorPicker,
  Group,
  Select,
  Stack,
  Switch,
  Text,
} from "@mantine/core";

const PREDEFINED_COLORS = [
  { name: "Blue", hex: "#228be6" },
  { name: "Teal", hex: "#12b886" },
  { name: "Green", hex: "#40c057" },
  { name: "Orange", hex: "#fd7e14" },
  { name: "Red", hex: "#fa5252" },
  { name: "Purple", hex: "#7950f2" },
  { name: "Pink", hex: "#e64980" },
  { name: "Gray", hex: "#868e96" },
];

const DEFAULT_WIDGETS = {
  clients: true,
  engagements: true,
  findings: true,
  credentials: true,
  news: true,
  calendar: true,
  recent_activity: true,
};

const DEFAULT_NAV = {
  dashboard: true,
  clients: true,
  engagements: true,
  templates: true,
  documents: true,
  calendar: true,
  news: true,
  activity_log: true,
  settings: true,
};

interface Props {
  theme: "light" | "dark";
  setTheme: (t: "light" | "dark") => void;
  accentColor: string;
  setAccentColor: (c: string) => void;
}

export function SettingsAppearance({
  theme,
  setTheme,
  accentColor,
  setAccentColor,
}: Props) {
  const [widgets, setWidgets] = useState(DEFAULT_WIDGETS);
  const [navItems, setNavItems] = useState(DEFAULT_NAV);

  return (
    <Stack gap="lg">
      <Select
        label="Color Scheme"
        description="Choose your preferred theme."
        data={[
          { value: "light", label: "Light" },
          { value: "dark", label: "Dark" },
        ]}
        value={theme}
        onChange={(value) => {
          if (value === "light" || value === "dark") {
            setTheme(value);
          }
        }}
      />

      <Stack gap="xs">
        <Text fw={600}>Accent Color</Text>
        <Group gap="xs">
          {PREDEFINED_COLORS.map((c) => (
            <Button
              key={c.hex}
              size="xs"
              style={{
                backgroundColor: c.hex,
                color: "#fff",
                border:
                  accentColor === c.hex
                    ? "2px solid #000"
                    : "2px solid transparent",
              }}
              onClick={() => setAccentColor(c.hex)}
            >
              {c.name}
            </Button>
          ))}
        </Group>
        <ColorPicker
          format="hex"
          value={accentColor}
          onChange={setAccentColor}
        />
      </Stack>

      <Stack gap="xs">
        <Text fw={600}>Dashboard Widgets</Text>
        {Object.entries(widgets).map(([key, value]) => (
          <Switch
            key={key}
            label={key
              .replace(/_/g, " ")
              .replace(/\b\w/g, (l) => l.toUpperCase())}
            checked={value}
            onChange={(event) => {
              const next = {
                ...widgets,
                [key]: event.currentTarget.checked,
              };
              setWidgets(next);
            }}
          />
        ))}
      </Stack>

      <Stack gap="xs">
        <Text fw={600}>Navigation Items</Text>
        {Object.entries(navItems).map(([key, value]) => (
          <Switch
            key={key}
            label={key
              .replace(/_/g, " ")
              .replace(/\b\w/g, (l) => l.toUpperCase())}
            checked={value}
            disabled={key === "dashboard" || key === "settings"}
            onChange={(event) => {
              const next = {
                ...navItems,
                [key]: event.currentTarget.checked,
              };
              setNavItems(next);
            }}
          />
        ))}
      </Stack>
    </Stack>
  );
}
