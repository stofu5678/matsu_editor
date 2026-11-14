// The jack Outline (TypeScript): a tree of the lines that start with "#". Node.js strips the types.
import { headingOf } from './lib/heading.ts';
import type { Heading } from './lib/heading.ts';

export default function (): void {
  const count: number = Editor.GetLineCount(0);
  Outline.SetListType(100);
  Outline.SetTitle('Headings (TypeScript sample)');
  for (let n = 1; n <= count; n++) {
    const heading: Heading | null = headingOf(Editor.GetLineStr(n));
    if (heading) {
      Outline.AddFuncInfo2(n, 1, heading.title, heading.level);
    }
  }
}
