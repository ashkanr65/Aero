import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';

const colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
const applyTheme = () => document.documentElement.classList.toggle('dark', colorScheme.matches);
applyTheme();
colorScheme.addEventListener('change', applyTheme);

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
