import React from "react";
import ReactDOM from "react-dom/client";
import { BoardApp } from "./BoardApp";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BoardApp />
  </React.StrictMode>,
);
