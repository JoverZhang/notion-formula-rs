type ThemeName = "light" | "dark";

const THEME_STORAGE_KEY = "nf-theme";

function isThemeName(value: string | null): value is ThemeName {
  return value === "light" || value === "dark";
}

function getInitialTheme(): ThemeName {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (isThemeName(stored)) return stored;
  if (window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches) {
    return "dark";
  }
  return "light";
}

function applyTheme(theme: ThemeName) {
  document.documentElement.dataset.theme = theme;
  localStorage.setItem(THEME_STORAGE_KEY, theme);
}

function syncToggleLabel(button: HTMLButtonElement, theme: ThemeName) {
  const next = theme === "dark" ? "Light" : "Dark";
  button.textContent = `${theme === "dark" ? "Dark" : "Light"} mode`;
  button.setAttribute("aria-pressed", theme === "dark" ? "true" : "false");
  button.setAttribute("aria-label", `Switch to ${next.toLowerCase()} mode`);
}

export function initThemeToggle(button: HTMLButtonElement) {
  let current = getInitialTheme();
  applyTheme(current);
  syncToggleLabel(button, current);

  button.addEventListener("click", () => {
    current = current === "dark" ? "light" : "dark";
    applyTheme(current);
    syncToggleLabel(button, current);
  });
}
