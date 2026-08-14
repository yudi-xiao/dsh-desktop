import React from "react";
import ReactDOM from "react-dom/client";
import { MarketApp } from "@dsh-desktop/plugin-market";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MarketApp />
  </React.StrictMode>,
);
