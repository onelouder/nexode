import React from 'react';
import { createRoot } from 'react-dom/client';

import { SynapseGridApp } from './App';
import './styles.css';

const container = document.getElementById('root');
if (!container) {
  throw new Error('Missing Synapse Grid root element');
}

const surface = container.getAttribute('data-surface') === 'synapse-sidebar' ? 'synapse-sidebar' : 'synapse-grid';

createRoot(container).render(
  <React.StrictMode>
    <SynapseGridApp surface={surface} />
  </React.StrictMode>,
);
