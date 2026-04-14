import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppProvider } from "./context";
import { Layout } from "./Layout";
import { Workbench } from "./pages/Workbench";
import { Analyze } from "./pages/Analyze";
import { Settings } from "./pages/Settings";
import "./styles.css";

export default function App() {
  return (
    <BrowserRouter>
      <AppProvider>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<Workbench />} />
            <Route path="analyze" element={<Analyze />} />
            <Route path="settings" element={<Settings />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </AppProvider>
    </BrowserRouter>
  );
}
