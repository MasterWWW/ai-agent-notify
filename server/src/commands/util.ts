/** Read the value following `name` in argv (e.g. --port 3210). */
export function argValue(argv: string[], name: string): string | undefined {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
}

/** Mask a secret for display. */
export function mask(s: string | undefined): string {
  if (!s) return "(未设置)";
  return s.length > 12 ? s.slice(0, 4) + "…" + s.slice(-4) : "***";
}
