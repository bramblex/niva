export type ThemePreference = "system" | "light" | "dark";

const storageKey = "niva-devtools-theme";

export function readThemePreference(): ThemePreference {
  try {
    const stored = window.localStorage.getItem(storageKey);
    if (stored === "light" || stored === "dark") {
      return stored;
    }
  } catch {
    // The system setting remains usable if WebView storage is unavailable.
  }
  return "system";
}

export function saveThemePreference(preference: ThemePreference): void {
  try {
    if (preference === "system") {
      window.localStorage.removeItem(storageKey);
    } else {
      window.localStorage.setItem(storageKey, preference);
    }
  } catch {
    // The selected theme still applies for this session.
  }
}

export function applyThemePreference(
  preference: ThemePreference,
  systemTheme?: "light" | "dark"
): void {
  const resolved = preference === "system"
    ? systemTheme ?? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
    : preference;
  document.documentElement.dataset.theme = resolved;
}
