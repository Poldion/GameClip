// SPDX-License-Identifier: GPL-3.0-or-later
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ipc } from "./ipc";
import { Store, StoreContext } from "./store";
import "./styles/tokens.css";

void ipc().then((i) => {
  const store = new Store(i);
  void store.start().catch((e) => store.notify(String((e as Error).message ?? e), "error"));
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <StoreContext.Provider value={store}>
        <App />
      </StoreContext.Provider>
    </StrictMode>,
  );
});
