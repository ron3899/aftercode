export function fmtTime(sec: number | null | undefined): string {
  if (sec == null || isNaN(sec)) return "0:00";
  const s = Math.max(0, Math.floor(sec));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${r.toString().padStart(2, "0")}`;
}

export function fmtDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

export function langLabel(code: string): string {
  return code === "he" ? "Hebrew" : code === "en" ? "English" : code;
}
