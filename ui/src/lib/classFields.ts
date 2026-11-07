// Champs de la classe : des mots entre accolades, sans accents, que « Remplir avec ma classe » remplace
// ({prenom}, {age}, {PRENOM_1}, {anniversaires_mars}…). Le moteur (crates/carnet-core/src/fields.rs) en est
// l'autorité : un test là-bas vérifie que cette liste et ces libellés sont les siens.

export interface FieldInfo {
  key: string;
  label: string;
  hint: string;
}

/** Les champs d'un élève, dans l'ordre où on les propose. */
export const FIELDS: FieldInfo[] = [
  { key: "prenom", label: "Prénom", hint: "Le prénom de l'enfant" },
  { key: "PRENOM", label: "PRÉNOM", hint: "Le prénom en capitales" },
  { key: "nom", label: "Nom", hint: "Le nom de famille" },
  { key: "groupe", label: "Groupe", hint: "Son groupe dans la classe" },
  { key: "age", label: "Âge", hint: "L'âge : « 4 ans »" },
  { key: "anniversaire", label: "Anniversaire", hint: "Le jour de son anniversaire : « 12 mars »" },
];

/** Les mois, comme le moteur les écrit (class::MONTHS). */
const MONTHS = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];

/** Sans accents : « février » → « fevrier ». */
const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "");

/** Anciennes écritures lues comme synonymes. */
const ALIASES: Record<string, string> = { prénom: "prenom", PRÉNOM: "PRENOM", âge: "age" };

/** La n-ième lettre du prénom : en capitale (`PRENOM_2`) ou telle qu'elle s'écrit (`prenom_2`). */
export const letterKey = (n: number, capital: boolean) => `${capital ? "PRENOM" : "prenom"}_${n}`;

/** Les anniversaires d'un mois (1 à 12), pour une affiche de toute la classe. */
export const monthKey = (month: number) => `anniversaires_${fold(MONTHS[month - 1])}`;

function letterOf(key: string): { n: number; capital: boolean } | null {
  const m = /^(PRENOM|prenom)_(\d+)$/.exec(key);
  const n = m ? Number(m[2]) : 0;
  return m && n >= 1 ? { n, capital: m[1] === "PRENOM" } : null;
}

function monthOf(key: string): number | null {
  const m = /^anniversaires[_ ](.+)$/.exec(key);
  const i = m ? MONTHS.findIndex((x) => fold(x) === fold(m[1])) : -1;
  return i >= 0 ? i + 1 : null;
}

/** Ce qu'on lit à la place d'un champ non rempli : sur la pastille et à l'impression. */
export function fieldLabel(key: string): string | null {
  const letter = letterOf(key);
  if (letter) return `${letter.capital ? "Lettre" : "lettre"} ${letter.n}`;
  const month = monthOf(key);
  if (month) {
    const name = MONTHS[month - 1];
    return `Anniversaires ${/^[ao]/.test(name) ? "d'" : "de "}${name}`;
  }
  return FIELDS.find((f) => f.key === (ALIASES[key] ?? key))?.label ?? null;
}

/** Ce qui s'affiche à la place du champ sur la page : son nom, sauf une lettre, dont la case est petite et qui ne
 *  montre que son numéro (comme fields::shown dans le moteur). */
export function fieldShown(key: string): string | null {
  const letter = letterOf(key);
  return letter ? String(letter.n) : fieldLabel(key);
}

/** Un champ d'élève (lettre comprise), pas un champ de toute la classe. */
export const isStudentField = (key: string) => !!letterOf(key) || FIELDS.some((f) => f.key === (ALIASES[key] ?? key));

export const isMonthField = (key: string) => monthOf(key) !== null;

export type Piece = { text: string } | { field: string };

/** Le texte en morceaux de texte et en champs reconnus ; des accolades inconnues restent du texte. */
export function splitFields(text: string): Piece[] {
  const out: Piece[] = [];
  let start = 0;
  let at = 0;
  for (;;) {
    const open = text.indexOf("{", at);
    if (open < 0) break;
    const close = text.indexOf("}", open + 1);
    if (close < 0) break;
    const inner = text.slice(open + 1, close);
    if (fieldLabel(inner) !== null) {
      if (open > start) out.push({ text: text.slice(start, open) });
      out.push({ field: inner });
      start = at = close + 1;
    } else {
      at = open + 1;
    }
  }
  if (start < text.length) out.push({ text: text.slice(start) });
  return out;
}
