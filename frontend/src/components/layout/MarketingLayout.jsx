import { Outlet } from 'react-router-dom';
import Header from './Header';
import Footer from './Footer';

// Chrome for the public routes (/, /login, /register): the marketing header
// over a growing content area, with the footer pinned to the bottom.
export default function MarketingLayout() {
  return (
    <div className="flex min-h-screen flex-col bg-black">
      <div className="grow">
        <Header />
        <Outlet />
      </div>
      <Footer />
    </div>
  );
}
