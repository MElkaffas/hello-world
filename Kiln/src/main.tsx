import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import { App } from "./App";
import { DropZone } from "./views/DropZone";
import { StoreProvider } from "./store";

const isDropZone = window.location.hash === "#dropzone";
if (isDropZone) document.body.classList.add("dropzone-body");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {isDropZone ? (
      <DropZone />
    ) : (
      <StoreProvider>
        <App />
      </StoreProvider>
    )}
  </React.StrictMode>,
);
