import React from 'react';
import '../styles/Sidebar.css';

interface SidebarProps {
  children: React.ReactNode;
}

export const Sidebar: React.FC<SidebarProps> = ({ children }) => {
  return (
    <aside className="sidebar">
      {children}
    </aside>
  );
};
