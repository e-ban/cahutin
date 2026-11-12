import { ask as tauriAsk } from "@tauri-apps/plugin-dialog";

/** Question oui/non : boîte de dialogue native dans l'application, `confirm` dans un simple navigateur. */
export async function ask(question: string, okLabel = "Oui", cancelLabel = "Annuler"): Promise<boolean> {
  try {
    return await tauriAsk(question, { title: "Cahutin", kind: "warning", okLabel, cancelLabel });
  } catch {
    return window.confirm(question);
  }
}
