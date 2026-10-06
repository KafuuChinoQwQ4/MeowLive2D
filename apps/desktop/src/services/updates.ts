import type { UpdateStatus } from "@meowlive/contracts";
import { invokeDesktop } from "./desktop/transport";
export type { UpdateStatus } from "@meowlive/contracts";
export interface UpdateClient {
  status(): Promise<UpdateStatus>;
  check(): Promise<void>;
  prepare(): Promise<void>;
  cancel(): Promise<void>;
  install(): Promise<void>;
}
async function invoke<T>(command: string): Promise<T> {
  return invokeDesktop<T>(command);
}
export const updateClient: UpdateClient = {
  status: () => invoke("update_status"),
  check: () => invoke("update_check"),
  prepare: () => invoke("update_prepare"),
  cancel: () => invoke("update_cancel"),
  install: () => invoke("update_install"),
};
