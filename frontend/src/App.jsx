import { Routes, Route } from 'react-router-dom';
import Header from './components/layout/Header';
import Footer from './components/layout/Footer';
import LandingPage from './pages/LandingPage';

export default function App() {
  return (
    <div className='flex flex-col min-h-screen'> {/* Added min-h-screen for sticky footer */}
      <div className='grow'>
        <Header />
        <Routes>
          <Route path='/' element={<LandingPage />} />
        </Routes>
      </div>
      <Footer />
    </div>
  );
}