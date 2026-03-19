import React from 'react';
import { createRoot } from 'react-dom/client';

import { KanbanApp } from './App';
import './styles.css';

const container = document.getElementById('root');
if (!container) {
  throw new Error('Missing Macro Kanban root element');
}

createRoot(container).render(
  <React.StrictMode>
    <KanbanApp />
  </React.StrictMode>,
);
