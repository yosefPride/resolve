// Shared table chrome for the admin panels: horizontal-scroll wrapper,
// uppercase header row, hover-highlighted body rows. `columns` entries are
// strings, or { label, right: true } for a right-aligned column. Rows go
// through Tr, cells through Td (base padding; pass className for color).
export default function Table({ columns, children }) {
  return (
    <div className="overflow-x-auto rounded-lg border border-white/10">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-white/10 text-xs font-medium tracking-wide text-slate-400 uppercase">
            {columns.map((col) => {
              const { label, right } = typeof col === 'string' ? { label: col } : col;
              return (
                <th key={label} className={`px-4 py-3 ${right ? 'text-right' : ''}`}>
                  {label}
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}

export function Tr(props) {
  return <tr className="border-b border-white/5 last:border-0 hover:bg-white/5" {...props} />;
}

export function Td({ className = '', ...props }) {
  return <td className={`px-4 py-3 ${className}`} {...props} />;
}
