import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import Dashboard from "./pages/Dashboard";
import "./App.css";

function App() {
  const [currentView, setCurrentView] = useState<"dashboard" | "deploy" | "chat">("dashboard");

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="logo">
          <h1>Deployotron</h1>
        </div>
        <ul className="nav-menu">
          <li 
            className={currentView === "dashboard" ? "active" : ""}
            onClick={() => setCurrentView("dashboard")}
          >
            Dashboard
          </li>
          <li 
            className={currentView === "deploy" ? "active" : ""}
            onClick={() => setCurrentView("deploy")}
          >
            Deployments
          </li>
          <li 
            className={currentView === "chat" ? "active" : ""}
            onClick={() => setCurrentView("chat")}
          >
            AI Assistant
          </li>
        </ul>
      </nav>
      <main className="content">
        {currentView === "dashboard" && <Dashboard />}
        {currentView === "deploy" && <div>Deployment View - Coming soon</div>}
        {currentView === "chat" && <div>Chat Interface - Coming soon</div>}
      </main>
    </div>
  );
}

export default App;
