import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider } from "@mantine/core";
import "@mantine/core/styles.css";
import { ErrorBoundary } from "./components/ErrorBoundary";
import App from "./App";
import "./App.css";

const root = document.getElementById("root") as HTMLElement;

// Remove the static fallback once React mounts
const fallback = document.getElementById("fallback");
if (fallback) fallback.remove();

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <ErrorBoundary>
      <MantineProvider defaultColorScheme="light">
        <App />
      </MantineProvider>
    </ErrorBoundary>
  </React.StrictMode>,
);
