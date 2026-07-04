import { Routes, Route } from 'react-router-dom';
import Header from './components/layout/Header';
import Footer from './components/layout/Footer';

export default function App() {
  return (
    <div className='flex flex-col min-h-screen'> {/* Added min-h-screen for sticky footer */}
      <div className='grow'>
        <Header />
        <Routes>
        </Routes>
      </div>
      <Footer />
    </div>
  );
}