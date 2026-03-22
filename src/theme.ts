export type Theme = "dark" | "light" | "system";

export interface AccentColor {
  value: string;
  hover: string;
  label: string;
}

export const ACCENT_COLORS: AccentColor[] = [
  { value: "#e94560", hover: "#ff6b81", label: "Red" },
  { value: "#f97316", hover: "#fb923c", label: "Orange" },
  { value: "#eab308", hover: "#facc15", label: "Yellow" },
  { value: "#22c55e", hover: "#4ade80", label: "Green" },
  { value: "#14b8a6", hover: "#2dd4bf", label: "Teal" },
  { value: "#3b82f6", hover: "#60a5fa", label: "Blue" },
  { value: "#8b5cf6", hover: "#a78bfa", label: "Purple" },
  { value: "#ec4899", hover: "#f472b6", label: "Pink" },
];

export function applyTheme(theme: Theme): void {
  const resolved =
    theme === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : theme;
  document.documentElement.setAttribute("data-theme", resolved);
}

export function getTheme(): Theme {
  return (localStorage.getItem("theme") as Theme) ?? "dark";
}

export function setTheme(theme: Theme): void {
  localStorage.setItem("theme", theme);
  applyTheme(theme);
}

export type FontSize = "small" | "medium" | "large";

const FONT_SIZE_PX: Record<FontSize, string> = {
  small: "14px",
  medium: "16px",
  large: "19px",
};

export function applyFontSize(size: FontSize): void {
  document.documentElement.style.fontSize = FONT_SIZE_PX[size];
}

export function getFontSize(): FontSize {
  return (localStorage.getItem("fontSize") as FontSize) ?? "medium";
}

export function setFontSize(size: FontSize): void {
  localStorage.setItem("fontSize", size);
  applyFontSize(size);
}

export type DefaultRulesDoc = "cr" | "mtr" | "ipg";

export function getDefaultRulesDoc(): DefaultRulesDoc {
  return (localStorage.getItem("defaultRulesDoc") as DefaultRulesDoc) ?? "cr";
}

export function setDefaultRulesDoc(doc: DefaultRulesDoc): void {
  localStorage.setItem("defaultRulesDoc", doc);
}

export function applyAccent(color: AccentColor): void {
  const el = document.documentElement;
  el.style.setProperty("--accent", color.value);
  el.style.setProperty("--accent-hover", color.hover);
}

export function getAccent(): AccentColor {
  const stored = localStorage.getItem("accent");
  return ACCENT_COLORS.find((c) => c.value === stored) ?? ACCENT_COLORS[0];
}

export function setAccent(color: AccentColor): void {
  localStorage.setItem("accent", color.value);
  applyAccent(color);
}
