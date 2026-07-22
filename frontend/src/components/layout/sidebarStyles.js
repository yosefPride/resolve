// Row styling shared by the app sidebar (Sidebar.jsx) and the landing-page
// demo sidebar (components/marketing/demo/DemoSidebar.jsx). It lives in its own
// module so the two can't drift apart: the demo exists to look exactly like the
// product, and a hover colour changed in one place should reach both.
//
// Only the presentation lives here. Collapse state, auth, navigation and data
// fetching stay in whichever component owns them — the demo has none of those.

export const ROW =
  'flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors';
export const IDLE = 'border-transparent text-slate-400 hover:bg-white/5 hover:text-white';
export const ACTIVE = 'border-sky-400 bg-white/10 text-white';

export function rowClasses(collapsed, isActive) {
  return `${ROW} border-l-2 ${collapsed ? 'justify-center px-2' : ''} ${isActive ? ACTIVE : IDLE}`;
}
