// Avis de nouvelle version : demandé une fois, après l'accueil et la visite ; avec l'accord de l'utilisateur,
// GitHub est consulté au plus une fois par jour (côté Rust) et un bandeau propose la page des téléchargements.
import { getVersion } from "@tauri-apps/api/app";
import { checkUpdate } from "./api";
import { readPref, writePref } from "./prefs";

export interface UpdateState {
  /** Jour de la dernière consultation (AAAA-MM-JJ). */
  checkedOn: string | null;
  /** Version de l'application lors de cette consultation : une autre version installée consulte à nouveau. */
  forVersion: string | null;
  /** Version plus récente trouvée, s'il y en a une. */
  available: string | null;
  /** Version dont le bandeau a été fermé. */
  dismissed: string | null;
}

export function shouldAskConsent(consent: boolean | null, s: { tourDone: boolean; tourRunning: boolean; welcomeOpen: boolean }): boolean {
  return consent === null && s.tourDone && !s.tourRunning && !s.welcomeOpen;
}

export function shouldCheck(consent: boolean | null, state: UpdateState, appVersion: string, today: string): boolean {
  return consent === true && (state.checkedOn !== today || state.forVersion !== appVersion);
}

export function bannerVersion(consent: boolean | null, state: UpdateState, appVersion: string): string | null {
  const { available } = state;
  return consent === true && state.forVersion === appVersion && available !== null && available !== state.dismissed ? available : null;
}

const today = () => new Date().toISOString().slice(0, 10);

class Updates {
  /** Réponse à la question : rien tant qu'elle n'a pas été posée. */
  consent = $state<boolean | null>(readPref("updateConsent", null));
  state = $state<UpdateState>(readPref("update", { checkedOn: null, forVersion: null, available: null, dismissed: null }));
  appVersion = $state("");

  setConsent(value: boolean) {
    this.consent = value;
    writePref("updateConsent", value);
    if (value) void this.checkIfDue();
  }

  /** Au lancement et à l'accord : consulte GitHub si c'est le moment ; un échec ne change rien. */
  async checkIfDue() {
    if (!this.appVersion) this.appVersion = await getVersion().catch(() => "");
    if (!this.appVersion || !shouldCheck(this.consent, this.state, this.appVersion, today())) return;
    let available: string | null;
    try {
      available = await checkUpdate();
    } catch {
      return;
    }
    this.save({ ...this.state, checkedOn: today(), forVersion: this.appVersion, available });
  }

  dismiss(version: string) {
    this.save({ ...this.state, dismissed: version });
  }

  private save(next: UpdateState) {
    this.state = next;
    writePref("update", next);
  }
}

export const updates = new Updates();
