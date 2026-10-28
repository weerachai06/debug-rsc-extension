import type { Loc } from '../bindings/Loc';

const identifier = /^[A-Za-z_$][\w$]*$/;

/** `1f.filterData.brands[0]`: the row id in hex, then the path into its value. */
export const formatPath = ({ id, path }: Loc): string =>
  id.toString(16) +
  path
    .map((seg) => (typeof seg === 'number' ? `[${seg}]` : identifier.test(seg) ? `.${seg}` : `[${JSON.stringify(seg)}]`))
    .join('');
