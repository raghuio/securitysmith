import { useState, useCallback } from "react";

export type AppView =
  | { area: "dashboard" }
  | { area: "clients"; subview?: "list" | "detail"; clientId?: number }
  | {
      area: "engagements";
      subview?: "list" | "detail";
      engagementId?: number;
      clientId?: number;
    }
  | { area: "findings"; subview?: "list" | "detail"; engagementId?: number }
  | { area: "credentials"; engagementId?: number }
  | { area: "reports"; engagementId?: number }
  | { area: "documents"; clientId?: number }
  | { area: "invoices"; clientId?: number }
  | { area: "templates" }
  | { area: "calendar" }
  | { area: "news" }
  | { area: "activity-log" }
  | { area: "analytics" }
  | { area: "checklists" }
  | { area: "compliance" }
  | { area: "settings" };

const MAX_HISTORY = 20;

export function useNavigationHistory() {
  const [history, setHistory] = useState<AppView[]>([{ area: "dashboard" }]);
  const [index, setIndex] = useState(0);

  const navigate = useCallback(
    (view: AppView) => {
      setHistory((prev) => {
        const next = prev.slice(0, index + 1);
        next.push(view);
        if (next.length > MAX_HISTORY) {
          next.shift();
        }
        return next;
      });
      setIndex((prev) => Math.min(prev + 1, MAX_HISTORY - 1));
    },
    [index],
  );

  const goBack = useCallback(() => {
    setIndex((prev) => Math.max(0, prev - 1));
  }, []);

  const goForward = useCallback(() => {
    setIndex((prev) => Math.min(history.length - 1, prev + 1));
  }, [history.length]);

  const current = history[index] ?? { area: "dashboard" };
  const canGoBack = index > 0;
  const canGoForward = index < history.length - 1;

  return { current, navigate, goBack, goForward, canGoBack, canGoForward };
}
