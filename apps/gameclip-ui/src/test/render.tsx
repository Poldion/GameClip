// SPDX-License-Identifier: GPL-3.0-or-later
// Rendert eine Komponente mit Store gegen ipc.mock.ts.

import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { createMockIpc, type MockState, mockState } from "../ipc.mock";
import { Store, StoreContext } from "../store";

export async function renderWithStore(ui: ReactNode, over: Partial<MockState> = {}) {
  const ipc = createMockIpc({ ...mockState(), ...over });
  const store = new Store(ipc);
  await store.start();
  const result = render(<StoreContext.Provider value={store}>{ui}</StoreContext.Provider>);
  return { ...result, ipc, store, calls: ipc.state.calls };
}
