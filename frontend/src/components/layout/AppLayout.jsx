import { Outlet } from 'react-router-dom';
import Header from './Header';
import Footer from './Footer';

// Chrome for the authenticated app routes. Deliberately identical to
// MarketingLayout for now so extracting the layouts is a pure refactor with no
// visual change; the next stage replaces the Header/Footer here with the
// persistent left Sidebar.
export default function AppLayout() {
  return (
    <div className="flex min-h-screen flex-col">
      <div className="grow">
        <Header />
        <Outlet />
      </div>
      <Footer />
    </div>
  );
}
