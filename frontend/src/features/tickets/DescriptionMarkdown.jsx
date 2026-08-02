import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

// Styled overrides for the hand-rolled dark theme — the app doesn't use the
// @tailwindcss/typography plugin, so element styling lives here instead of
// a `prose` class.
const components = {
  h1: (props) => <h1 className="mb-2 mt-4 text-lg font-semibold text-white first:mt-0" {...props} />,
  h2: (props) => <h2 className="mb-2 mt-4 text-base font-semibold text-white first:mt-0" {...props} />,
  h3: (props) => <h3 className="mb-1 mt-3 text-sm font-semibold text-white first:mt-0" {...props} />,
  p: (props) => <p className="mb-3 text-sm text-slate-200 last:mb-0" {...props} />,
  ul: (props) => <ul className="mb-3 list-disc space-y-1 pl-5 text-sm text-slate-200" {...props} />,
  ol: (props) => <ol className="mb-3 list-decimal space-y-1 pl-5 text-sm text-slate-200" {...props} />,
  li: (props) => <li className="text-sm text-slate-200" {...props} />,
  a: (props) => (
    <a className="text-sky-400 underline-offset-2 hover:underline" target="_blank" rel="noopener noreferrer" {...props} />
  ),
  strong: (props) => <strong className="font-semibold text-white" {...props} />,
  code: (props) => <code className="rounded bg-white/10 px-1 py-0.5 text-xs text-slate-100" {...props} />,
  pre: (props) => <pre className="mb-3 overflow-x-auto rounded-lg bg-black/40 p-3 text-xs text-slate-100" {...props} />,
  blockquote: (props) => (
    <blockquote className="mb-3 border-l-2 border-white/20 pl-3 text-sm text-slate-400" {...props} />
  ),
  hr: () => <hr className="my-4 border-white/10" />,
  table: (props) => (
    <div className="mb-3 overflow-x-auto">
      <table className="w-full border-collapse text-sm text-slate-200" {...props} />
    </div>
  ),
  th: (props) => <th className="border border-white/10 px-2 py-1 text-left font-medium text-white" {...props} />,
  td: (props) => <td className="border border-white/10 px-2 py-1" {...props} />,
};

export default function DescriptionMarkdown({ children }) {
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
      {children}
    </ReactMarkdown>
  );
}
