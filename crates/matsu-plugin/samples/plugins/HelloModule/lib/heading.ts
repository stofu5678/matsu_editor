// A TypeScript file of the plugin folder that outline.ts imports (with its extension).
export interface Heading {
  title: string;
  level: number;
}

export function headingOf(line: string): Heading | null {
  const m = /^(#+)\s*(.*)$/.exec(line.replace(/[\r\n]+$/, ''));
  return m ? { title: m[2], level: m[1].length - 1 } : null;
}
