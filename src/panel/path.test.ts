import { expect, test } from 'vitest';
import { formatPath } from './path';

test('formats the row id in hex, then keys as dots and indices as brackets', () => {
  const at = { stream: 'doc:1', row: 3, id: 31, path: ['filterData', 'brands', 0, 'a-b'] };

  expect(formatPath(at)).toBe('1f.filterData.brands[0]["a-b"]');
});

test('a bare row has no path', () => {
  expect(formatPath({ stream: 'doc:1', row: 0, id: 0, path: [] })).toBe('0');
});
