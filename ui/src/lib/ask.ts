import { ask as tauriAsk } from "@tauri-apps/plugin-dialog";

/** Question oui/non : boîte de dialogue native dans l'application, `confirm` dans un simple navigateur. */
export async function ask(question: string, okLabel = "Oui"): Promise<boolean> {
  try {
    return await tauriAsk(question, { title: "Carnet", kind: "warning", okLabel, cancelLabel: "Annuler" });
  } catch {
    return window.confirm(question);
  }
}
